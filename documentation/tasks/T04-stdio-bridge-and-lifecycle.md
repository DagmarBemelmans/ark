# T04 — stdio↔TCP pump and full lifecycle

- **Status:** Blocked on T03
- **Risk:** medium · **Size:** medium (~2-3h)
- **Depends on:** T03

## Goal

Complete `ark lsp`: bytes from stdin flow to the LSP TCP socket, bytes from the socket
flow to stdout, and every way the party can end is handled without orphans. After this
task, any stdio LSP client can use ark — Zed included, but also a test script.

## Background

- LSP framing (`Content-Length: N\r\n\r\n{json}`) is transport-independent — the pump
  copies raw bytes, it never parses LSP. Two directions, two loops.
- Implementation choice: plain threads with `std::io::copy` are fine (bridge process
  has no tokio requirement of its own); if reusing tokio feels natural after T02/T03,
  that's acceptable too. Prefer whichever yields simpler shutdown code.
- **stdout hygiene is the acceptance-critical property**: nothing but LSP bytes on
  stdout, ever (decision 0002). Logs → `--log` file or stderr.
- Exit paths to handle:
  | Trigger | Expected behavior |
  |---|---|
  | stdin EOF (editor closed us) | stop pumps, sidecar shutdown (T02), exit 0 |
  | TCP closed by kernel (LSP `exit`) | flush remaining bytes to stdout, sidecar shutdown, exit 0 |
  | child kernel died unexpectedly | error to stderr + log, exit non-zero |
  | SIGTERM/SIGINT | same as stdin EOF; note `crates/ark/src/signals.rs` blocks signals in kernel threads — check what applies to the bridge process before installing handlers |

## Requirements

1. New file `crates/ark/src/standalone_lsp/pump.rs` with the two copy loops and a
   small supervisor that turns "either direction ended" into an orderly shutdown.
2. `standalone_lsp::run` becomes the real thing: sidecar boot (T02) → LSP connect
   (T03) → pump (this task) → shutdown. Startup log line order: "kernel ready",
   "lsp connected", "bridge running".
3. Integration test speaking real LSP over the process boundary: spawn
   `env!("CARGO_BIN_EXE_ark") lsp`, write a framed `initialize` request to its stdin,
   read the framed response from stdout (assert it contains `"capabilities"`), then
   `shutdown` request + `exit` notification, assert exit code 0 and no orphan
   processes. Helper for framing lives in the test file. Follow
   `crates/ark/tests/integration/` conventions (module in `main.rs`).
4. A smoke check that completions flow end-to-end: after `initialize`/`initialized`,
   send `textDocument/didOpen` for a buffer containing `librar` and a
   `textDocument/completion` request; assert the response mentions `library`.
   Model the JSON on existing LSP integration tests (`crates/ark/tests/integration/lsp_completions.rs`).

## Acceptance criteria & verification

```sh
cargo build
just test -p ark standalone      # incl. the new end-to-end stdio tests
just clippy && cargo +nightly fmt --all
# manual: printf an initialize frame into `ark lsp` and watch the reply
```

## Deliverables

- `crates/ark/src/standalone_lsp/pump.rs` (new), `mod.rs` finalization
- integration tests
- CHANGELOG.md entry under the unreleased section (one line, matching existing style)

## Escalate if

- stdout ever receives non-LSP bytes in tests (find the source, report it — do not
  filter/patch the stream).
- Clean shutdown requires touching kernel-side code.
