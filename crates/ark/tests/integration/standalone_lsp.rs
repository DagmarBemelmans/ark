//
// standalone_lsp.rs
//
// Copyright (C) 2026 Posit Software, PBC. All rights reserved.
//
//

//! End-to-end integration tests for the `ark lsp` standalone bridge.
//!
//! These speak real LSP over the process boundary: they spawn `ark lsp` as a
//! separate process, write `Content-Length` framed JSON-RPC to its stdin, and
//! read framed replies back from its stdout. The bridge pumps those bytes to a
//! sidecar kernel's LSP server, so this exercises the full stack.
//!
//! The stdout reader is strict: it only accepts well-formed LSP frames, so any
//! stray non-LSP bytes on stdout (a decision-0002 violation) fail the test with
//! the offending bytes attached rather than being silently tolerated.
//!
//! Like every other kernel test in this crate, these require R to be installed:
//! the bridge boots a real R session.

#![cfg(unix)]

use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;
use std::process::Child;
use std::process::ChildStdin;
use std::process::ChildStdout;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use serde_json::json;
use serde_json::Value;
use tempfile::TempDir;

/// R startup is slow, so the first reply (which waits for the kernel to boot)
/// gets a generous window.
const FIRST_REPLY_TIMEOUT: Duration = Duration::from_secs(90);

/// Once the session is live, replies come back promptly.
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// After `exit`, the bridge must shut the kernel down and exit within this window.
const EXIT_TIMEOUT: Duration = Duration::from_secs(15);

/// The bridge should serve a standard LSP `initialize`/`shutdown`/`exit`
/// handshake over stdio, then exit cleanly with no orphaned kernel child.
#[test]
fn test_standalone_lsp_initialize_over_stdio() {
    let mut bridge = Bridge::spawn();

    let id = bridge.send_request("initialize", json!({ "capabilities": {} }));
    let result = bridge.recv_response(id, FIRST_REPLY_TIMEOUT);
    assert!(result.get("capabilities").is_some());

    bridge.send_notification("initialized", json!({}));

    let id = bridge.send_request("shutdown", Value::Null);
    bridge.recv_response(id, REPLY_TIMEOUT);
    bridge.send_notification("exit", Value::Null);

    let status = bridge.wait_exit(EXIT_TIMEOUT);
    assert!(status.success());

    bridge.assert_no_orphan();
}

/// Completions should flow end-to-end over the stdio bridge: opening a buffer
/// with the prefix `librar` and requesting completions at its end should surface
/// the base `library` function.
#[test]
fn test_standalone_lsp_completions_over_stdio() {
    let mut bridge = Bridge::spawn();

    let id = bridge.send_request("initialize", json!({ "capabilities": {} }));
    bridge.recv_response(id, FIRST_REPLY_TIMEOUT);
    bridge.send_notification("initialized", json!({}));

    let uri = "file:///test/librar.R";
    bridge.send_notification(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": "r",
                "version": 0,
                "text": "librar\n",
            }
        }),
    );

    let id = bridge.send_request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 6 },
        }),
    );
    let result = bridge.recv_response(id, REPLY_TIMEOUT);

    let labels = completion_labels(&result);
    assert!(labels.iter().any(|label| label == "library"));

    let id = bridge.send_request("shutdown", Value::Null);
    bridge.recv_response(id, REPLY_TIMEOUT);
    bridge.send_notification("exit", Value::Null);

    let status = bridge.wait_exit(EXIT_TIMEOUT);
    assert!(status.success());
}

/// A minimal LSP-over-stdio client driving an `ark lsp` child process.
struct Bridge {
    /// Holds the log directory on disk for the lifetime of the test.
    _dir: TempDir,
    child: Child,
    stdin: ChildStdin,
    /// Parsed LSP frames (or a hygiene violation) read off the child's stdout.
    frames: Receiver<FrameResult>,
    bridge_log: PathBuf,
    next_id: i64,
}

/// One item read off the bridge's stdout: either a parsed LSP frame or evidence
/// that non-LSP bytes reached stdout.
enum FrameResult {
    Message(Value),
    Garbage(String),
}

impl Bridge {
    /// Spawn `ark lsp`, wiring up its stdio and a background stdout reader.
    fn spawn() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bridge_log = dir.path().join("bridge.log");

        let mut child = Command::new(env!("CARGO_BIN_EXE_ark"))
            .arg("lsp")
            .arg("--log")
            .arg(&bridge_log)
            // Ark's own `log::info!` lines only pass the filter with `ark`
            // verbosity; we need them to parse the kernel pid for the orphan check.
            .env("RUST_LOG", "ark=info")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let frames = spawn_stdout_reader(stdout);

        Self {
            _dir: dir,
            child,
            stdin,
            frames,
            bridge_log,
            next_id: 0,
        }
    }

    /// Send a JSON-RPC request, returning the id assigned to it.
    fn send_request(&mut self, method: &str, params: Value) -> i64 {
        self.next_id += 1;
        let id = self.next_id;

        let mut message = json!({ "jsonrpc": "2.0", "id": id, "method": method });
        if !params.is_null() {
            message["params"] = params;
        }

        self.send_raw(&message);
        id
    }

    /// Send a JSON-RPC notification (no reply expected).
    fn send_notification(&mut self, method: &str, params: Value) {
        let mut message = json!({ "jsonrpc": "2.0", "method": method });
        if !params.is_null() {
            message["params"] = params;
        }

        self.send_raw(&message);
    }

    /// Read frames until the reply matching `id` arrives, returning its `result`.
    ///
    /// Auto-replies to server→client requests (e.g. `client/registerCapability`)
    /// and ignores notifications, mirroring what a real editor does.
    fn recv_response(&mut self, id: i64, timeout: Duration) -> Value {
        loop {
            let message = self.recv_message(timeout);

            let has_id = message.get("id").is_some();
            let has_method = message.get("method").is_some();
            let has_result = message.get("result").is_some() || message.get("error").is_some();

            if has_id && has_result && !has_method {
                let reply_id = message["id"].as_i64().unwrap();
                assert_eq!(reply_id, id);
                if let Some(error) = message.get("error") {
                    self.fail(&format!("LSP error reply for id {id}: {error}"));
                }
                return message.get("result").cloned().unwrap_or(Value::Null);
            }

            if has_id && has_method {
                // Server→client request: acknowledge with an empty success so
                // the server doesn't block on it.
                let reply =
                    json!({ "jsonrpc": "2.0", "id": message["id"].clone(), "result": null });
                self.send_raw(&reply);
            }
            // Otherwise it's a notification we don't care about; ignore it.
        }
    }

    /// Receive the next frame, failing loudly on timeout, disconnect, or garbage.
    fn recv_message(&mut self, timeout: Duration) -> Value {
        match self.frames.recv_timeout(timeout) {
            Ok(FrameResult::Message(value)) => value,
            Ok(FrameResult::Garbage(details)) => {
                self.fail(&format!("Non-LSP bytes on stdout: {details}"))
            },
            Err(RecvTimeoutError::Timeout) => self.fail("Timed out waiting for an LSP frame"),
            Err(RecvTimeoutError::Disconnected) => {
                self.fail("Bridge stdout closed before the expected frame")
            },
        }
    }

    /// Frame `message` and write it to the child's stdin.
    fn send_raw(&mut self, message: &Value) {
        let body = serde_json::to_string(message).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.stdin.flush().unwrap();
    }

    /// Wait up to `timeout` for the child to exit, returning its status.
    fn wait_exit(&mut self, timeout: Duration) -> ExitStatus {
        let deadline = Instant::now() + timeout;

        loop {
            match self.child.try_wait().unwrap() {
                Some(status) => return status,
                None => {
                    if Instant::now() >= deadline {
                        self.fail("Bridge did not exit after `exit`");
                    }
                    thread::sleep(Duration::from_millis(20));
                },
            }
        }
    }

    /// Assert the kernel child (parsed from the bridge log) did not outlive us.
    fn assert_no_orphan(&self) {
        let log = std::fs::read_to_string(&self.bridge_log).unwrap_or_default();
        let pid = parse_kernel_pid(&log)
            .unwrap_or_else(|| panic!("Could not parse kernel pid from log:\n{log}"));

        let deadline = Instant::now() + Duration::from_secs(3);
        while process_alive(pid) {
            if Instant::now() >= deadline {
                panic!("Kernel child {pid} outlived the bridge");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Kill the child and panic with diagnostics from both log files.
    fn fail(&mut self, message: &str) -> ! {
        let _ = self.child.kill();
        let _ = self.child.wait();
        panic!("{message}\n{}", self.diagnostics());
    }

    /// Read both log files for failure diagnostics.
    fn diagnostics(&self) -> String {
        let dir = self.bridge_log.parent().unwrap();
        let bridge = std::fs::read_to_string(&self.bridge_log).unwrap_or_default();
        let kernel = std::fs::read_to_string(dir.join("kernel.log")).unwrap_or_default();
        format!("--- bridge.log ---\n{bridge}\n--- kernel.log ---\n{kernel}")
    }
}

/// Spawn a thread that reads LSP frames off `stdout` and forwards them on a channel.
///
/// Stops at EOF (closing the channel) or the first non-LSP bytes (sending a
/// [`FrameResult::Garbage`]).
fn spawn_stdout_reader(stdout: ChildStdout) -> Receiver<FrameResult> {
    let (tx, rx) = channel();

    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);

        loop {
            match read_frame(&mut reader) {
                Ok(Some(body)) => match serde_json::from_slice::<Value>(&body) {
                    Ok(value) => {
                        if tx.send(FrameResult::Message(value)).is_err() {
                            return;
                        }
                    },
                    Err(err) => {
                        let text = String::from_utf8_lossy(&body).into_owned();
                        let _ = tx.send(FrameResult::Garbage(format!(
                            "frame body was not valid JSON ({err}): {text:?}"
                        )));
                        return;
                    },
                },
                Ok(None) => return,
                Err(details) => {
                    let _ = tx.send(FrameResult::Garbage(details));
                    return;
                },
            }
        }
    });

    rx
}

/// Read one `Content-Length` framed LSP message, returning its body.
///
/// Returns `Ok(None)` at a clean EOF. Any bytes that don't parse as an LSP frame
/// header yield `Err`, so non-LSP output on stdout is caught rather than hidden.
fn read_frame(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, String> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        // `read_line` also errors on invalid UTF-8, catching binary garbage.
        let count = reader
            .read_line(&mut line)
            .map_err(|err| format!("error reading stdout: {err}"))?;

        if count == 0 {
            return Ok(None);
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            if content_length.is_some() {
                break;
            }
            continue;
        }

        let Some((name, value)) = trimmed.split_once(':') else {
            return Err(format!("expected an LSP header, got {trimmed:?}"));
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value
                .trim()
                .parse()
                .map(Some)
                .map_err(|err| format!("bad Content-Length {value:?}: {err}"))?;
        }
    }

    let length =
        content_length.ok_or_else(|| String::from("frame had no Content-Length header"))?;
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|err| format!("error reading frame body: {err}"))?;

    Ok(Some(body))
}

/// Collect the `label` of each completion item, handling both the array and
/// list response shapes.
fn completion_labels(result: &Value) -> Vec<String> {
    let items = if let Some(array) = result.as_array() {
        array.as_slice()
    } else if let Some(array) = result.get("items").and_then(Value::as_array) {
        array.as_slice()
    } else {
        &[]
    };

    items
        .iter()
        .filter_map(|item| item.get("label").and_then(Value::as_str))
        .map(String::from)
        .collect()
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
