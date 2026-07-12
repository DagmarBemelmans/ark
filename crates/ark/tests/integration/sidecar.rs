//
// sidecar.rs
//
// Copyright (C) 2026 Posit Software, PBC. All rights reserved.
//
//

//! Integration test for the `ark lsp` standalone bridge.
//!
//! Spawns `ark lsp` as a real process (not a thread), waits for it to boot a
//! kernel child, start ark's LSP server, and log "lsp connected", then closes
//! its stdin and asserts that both the bridge and the kernel child exit
//! cleanly.
//!
//! Like every other kernel test in this crate, this requires R to be installed:
//! the sidecar boots a real R session.

#![cfg(unix)]

use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

/// R startup can be slow, so allow a generous window for the kernel to boot.
const BOOT_TIMEOUT: Duration = Duration::from_secs(60);

/// After closing stdin, both processes must exit within this window.
const EXIT_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn test_standalone_lsp_connects_and_shuts_down_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let bridge_log = dir.path().join("bridge.log");

    let mut bridge = Command::new(env!("CARGO_BIN_EXE_ark"))
        .arg("lsp")
        .arg("--log")
        .arg(&bridge_log)
        // Ark's own log lines (including "kernel ready" and "lsp connected")
        // only pass the filter when `ark` verbosity is requested; production
        // leaves them quiet.
        .env("RUST_LOG", "ark=info")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Wait for the bridge to boot the kernel and connect to the LSP, failing
    // fast if it dies first. "lsp connected" only appears after "kernel ready".
    let log_contents = wait_for_log_marker(&mut bridge, &bridge_log, dir.path(), "lsp connected");
    assert!(log_contents.contains("kernel ready"));

    let kernel_pid = parse_kernel_pid(&log_contents)
        .unwrap_or_else(|| panic!("Could not parse kernel pid from log:\n{log_contents}"));
    assert!(process_alive(kernel_pid));

    // Close the bridge's stdin. EOF triggers a clean shutdown of both the
    // bridge and the kernel child.
    drop(bridge.stdin.take());

    // The bridge must exit cleanly within the window.
    let status = wait_for_exit(&mut bridge, EXIT_TIMEOUT)
        .unwrap_or_else(|| panic!("ark lsp did not exit within {EXIT_TIMEOUT:?} of stdin close"));
    assert!(status.success());

    // The kernel child must be gone too: no orphan left behind.
    let orphan_deadline = Instant::now() + Duration::from_secs(3);
    while process_alive(kernel_pid) {
        if Instant::now() >= orphan_deadline {
            panic!("Kernel child {kernel_pid} outlived the bridge");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Poll the bridge log until it contains `marker`, returning its contents.
fn wait_for_log_marker(
    bridge: &mut std::process::Child,
    bridge_log: &Path,
    dir: &Path,
    marker: &str,
) -> String {
    let deadline = Instant::now() + BOOT_TIMEOUT;

    loop {
        if let Ok(Some(status)) = bridge.try_wait() {
            panic!(
                "ark lsp exited before {marker:?} (status {status}).\n{}",
                diagnostics(dir)
            );
        }

        let log = std::fs::read_to_string(bridge_log).unwrap_or_default();
        if log.contains(marker) {
            return log;
        }

        if Instant::now() >= deadline {
            let _ = bridge.kill();
            panic!("Timed out waiting for {marker:?}.\n{}", diagnostics(dir));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Wait up to `timeout` for the process to exit, returning its status.
fn wait_for_exit(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + timeout;

    loop {
        match child.try_wait().unwrap() {
            Some(status) => return Some(status),
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            },
        }
    }
}

/// Extract the kernel child's pid from a "kernel ready (kernel pid: N)" log line.
fn parse_kernel_pid(log: &str) -> Option<i32> {
    let marker = "kernel pid: ";
    let start = log.find(marker)? + marker.len();
    let digits: String = log[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Whether the process with the given pid is still alive.
fn process_alive(pid: i32) -> bool {
    // `kill(pid, 0)` performs error checking without sending a signal: it
    // returns 0 if the process exists and `ESRCH` if it doesn't.
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Read both log files for failure diagnostics.
fn diagnostics(dir: &Path) -> String {
    let bridge = std::fs::read_to_string(dir.join("bridge.log")).unwrap_or_default();
    let kernel = std::fs::read_to_string(dir.join("kernel.log")).unwrap_or_default();
    format!("--- bridge.log ---\n{bridge}\n--- kernel.log ---\n{kernel}")
}
