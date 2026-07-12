# T02 — Sidecar kernel boot (spawn self as kernel, handshake)

- **Status:** Done (2026-07-12, committed `04f70346`)
- **Risk:** **high** (most unknowns live here) · **Size:** large (split further if needed)
- **Depends on:** T01, decision [0002](../decisions/0002-sidecar-kernel-architecture.md)

## Goal

Inside `standalone_lsp::run`: boot a full ark kernel as a **child process** of the
current binary and get a working "minimal Jupyter frontend" connection to it. After
this task, the bridge can talk Jupyter to a live R session. (LSP comes in T03.)

## Background (read first — this is the recipe to imitate)

- `crates/ark_test/src/dummy_frontend.rs:1947-1990` — how the test suite creates a
  `DummyConnection`, gets `(connection_file, registration_file)`, and boots a kernel.
  Difference here: the tests call `start_kernel()` on a thread; **we spawn a child
  process instead** (fd isolation — see decision 0002).
- `crates/amalthea/src/fixtures/dummy_frontend.rs:34-130` — `DummyConnection::new()`,
  `get_connection_files()`, `DummyFrontend::from_connection()`. This module is public
  (`crates/amalthea/src/lib.rs:11`); ark already depends on amalthea.
- `crates/amalthea/src/connection_file.rs` / `registration_file.rs` — the file formats.
  The child needs the connection info as a JSON file on disk, passed via
  `--connection_file` (see `crates/ark/src/main.rs:107`, `crates/ark/src/start.rs:34`).
- `crates/ark/src/console/console_repl.rs:57` — `SessionMode`; child runs `background`.
- CLAUDE.md for style. Note the existing thread-spawn helper `stdext::spawn!`.

## Requirements

1. New file `crates/ark/src/standalone_lsp/sidecar.rs`:
   - Generate the loopback connection setup (reuse `DummyConnection`).
   - Write the connection file to a private temp dir (`tempfile` is already a
     workspace dep — check root `Cargo.toml`; if not, escalate rather than adding deps).
   - Spawn the child: `std::env::current_exe()` with args
     `--connection_file <path> --session-mode background --log <bridge-log-dir>/kernel.log`,
     stdin null, stdout+stderr **piped and forwarded to the bridge's stderr** (never stdout),
     tagged line-by-line (e.g. `kernel out:`) on background threads.
   - Complete the frontend side: `DummyFrontend::from_connection(...)`, wait for the
     kernel to be fully up (imitate what `dummy_frontend.rs:1947-1990` does after boot,
     including any registration/handshake steps it performs).
   - Start a **continuous iopub drain**: a background thread that receives and discards
     iopub messages in a loop (log at trace level). Rationale: bounded channels,
     `crates/ark/src/start.rs:68`; a full iopub queue freezes the kernel.
   - Return a handle struct owning: the frontend, the child (`std::process::Child`),
     the temp dir, and a `shutdown()` method: Jupyter `shutdown_request`
     (`dummy_frontend.rs:236` shows the message), wait ≤5s for the reply and child
     exit, then `kill()`. `Drop` must also make the child not outlive us.
2. `standalone_lsp::run` uses it: boot sidecar, log "kernel ready", clean shutdown on
   (for now) receiving stdin EOF.
3. Integration test in `crates/ark/tests/integration/` (module added to that binary's
   `main.rs`, per CLAUDE.md): spawn `ark lsp` **as a process** (`env!("CARGO_BIN_EXE_ark")`),
   assert it logs "kernel ready" (via `--log` file polling), close its stdin, assert
   both processes exit cleanly ≤10s. Mark it appropriately if it needs R installed —
   look at how existing kernel tests handle that (they all need R).

## Acceptance criteria & verification

```sh
cargo build
# NOTE: ark's own logs default to ERROR level, so "kernel ready" is only *visible*
# with RUST_LOG=ark=info. The bridge boots and exits correctly without it.
RUST_LOG=ark=info ./target/debug/ark lsp --log /tmp/ark-lsp.log   # logs kernel ready; Ctrl-D exits cleanly
ps aux | rg '[a]rk'                             # no orphans after exit
just test -p ark sidecar
just clippy && cargo +nightly fmt --all
```

## Deliverables

- `crates/ark/src/standalone_lsp/sidecar.rs` (new), `mod.rs` updates
- one integration test file under `crates/ark/tests/integration/`
- no changes to `start.rs`, `kernel.rs`, `dummy_frontend.rs`, or any existing behavior

## Escalate if (report, don't work around)

- `DummyConnection`/`DummyFrontend` turn out to be unusable outside tests (hidden
  test-only assumptions, `IS_TESTING` gates, panicky asserts in the boot path).
- `--session-mode background` misbehaves where `console` works — report the difference.
- The handshake requires sockets/steps the fixtures don't cover.
- You feel the need to add a new dependency or modify amalthea: stop, report.
