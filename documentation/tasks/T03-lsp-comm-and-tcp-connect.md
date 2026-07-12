# T03 — Open the LSP comm and connect over TCP

- **Status:** Delegated (2026-07-12)
- **Risk:** medium · **Size:** medium (~2h)
- **Depends on:** T02

## Goal

Given a running sidecar kernel (T02), make the bridge start ark's LSP server inside it
and hold a connected TCP stream to it. After this task the bridge has both ends in
hand: Zed-facing stdio (unused yet) and kernel-facing LSP TCP.

## Background (read first — imitate this exactly)

- `crates/ark_test/src/dummy_frontend.rs:905-933` — `start_server()`: the canonical
  sequence. Send `comm_open` on the shell socket with a fresh UUID `comm_id`, target
  name `"lsp"`, data `{"ip_address": "127.0.0.1"}`; then read from iopub: a `busy`
  status, a `comm_msg` whose `data.method == "server_started"` carrying
  `data.params.port`, an `idle` status. Then TCP-connect to `127.0.0.1:port`.
- **Conflict to resolve:** T02 introduced a continuous iopub drain, but this sequence
  must *read* specific iopub messages. Design accordingly — e.g. the drain thread
  matches `comm_msg`/`server_started` and hands the port over a channel, or the drain
  offers a temporary subscription. Keep it simple; document the choice in code
  structure (not comments).
- `crates/ark/src/lsp/handler.rs:60-95` — what the kernel does with the comm (waits
  for R init at `handler.rs:71`, then binds TCP and reports the port,
  `crates/ark/src/lsp/backend.rs:553-587`). Consequence: the `server_started` reply can
  take seconds on first start — use a generous timeout (≥30s) with a clear error.
- The wire types for `CommOpen` live in amalthea (`crates/amalthea/src/wire/`); see how
  `dummy_frontend.rs` (ark_test) constructs and sends them.

## Requirements

1. New file `crates/ark/src/standalone_lsp/lsp_connection.rs`:
   `pub fn start(sidecar: &..., timeout: Duration) -> anyhow::Result<std::net::TcpStream>`
   implementing the sequence above. Clear, distinct error messages for: comm send
   failure, timeout waiting for `server_started`, TCP connect failure.
2. Wire into `standalone_lsp::run`: after "kernel ready", start the LSP connection,
   log "lsp connected (port {port})", then (still) wait for stdin EOF and shut down
   cleanly: TCP stream dropped first, then sidecar shutdown from T02.
3. Extend the T02 integration test (or add a sibling): spawn `ark lsp`, wait for
   "lsp connected" in the log, close stdin, assert clean exit, no orphans.

## Acceptance criteria & verification

```sh
cargo build
./target/debug/ark lsp --log /tmp/ark-lsp.log   # log shows kernel ready + lsp connected (port N)
just test -p ark standalone                      # integration tests pass
just clippy && cargo +nightly fmt --all
```

## Deliverables

- `crates/ark/src/standalone_lsp/lsp_connection.rs` (new), `mod.rs`/`sidecar.rs` touch-ups
- integration test update
- nothing under `crates/ark/src/lsp/`, `crates/amalthea/`, or `crates/ark_test/` changes

## Escalate if

- The `server_started` message never arrives while the same sequence passes in
  `just test -p ark lsp` (points at a session-mode or handshake difference — report).
- You need to modify the kernel-side LSP handler to make this work.
