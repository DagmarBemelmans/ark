# 04 — Design options

The gap, restated: **Zed** wants to spawn one process and speak LSP over stdio.
**ark** only starts its LSP when a Jupyter frontend opens a comm, then serves it over
TCP — and the LSP only functions with a live R session in the same process
(see [02](02-how-ark-works-today.md)).

Three ways to close the gap were considered.

## Option A — Native standalone mode ("the proper surgery")

Add a mode where ark boots R **without any Jupyter machinery** and serves the LSP
directly over stdio: replicate the channel wiring of `start_kernel()`
(`crates/ark/src/start.rs:34`) but with no `kernel::connect`, dummy-draining the
channels a frontend would normally consume, then run a stdio variant of
`start_lsp()` (`crates/ark/src/lsp/backend.rs:553`).

- ✅ Architecturally clean; single process; the shape upstream ark will eventually want
  (their README: LSP "will be made available to other frontends in the future").
- ❌ Walks straight into every hidden assumption that "a frontend exists": bounded
  channels that block when undrained (`start.rs:68`), stdin requests, comm lifecycles,
  kernel-init handshakes. Each is a potential silent deadlock.
- ❌ R and the LSP share one process ⇒ one stray `printf` from native code can corrupt
  the stdio LSP stream; needs careful file-descriptor juggling.
- ❌ Highest-risk option for delegated implementation by smaller models; hardest to
  verify piecewise.

## Option B — Sidecar kernel + stdio↔TCP bridge ("reuse the proven path") ← recommended

Teach the ark binary a bridge mode (working name: `ark lsp`). The bridge process:

```
Zed ⇄ stdio ⇄ [ark lsp = bridge] ⇄ TCP ⇄ [ark --connection_file … = full kernel + R]
                     │                                   ▲
                     └── loopback Jupyter (ZMQ): spawn ──┘
                         comm_open "lsp" → port
```

1. Generates a loopback connection file and spawns **itself** as a normal kernel child:
   `current_exe() --connection_file <tmp> --session-mode background`.
2. Plays "minimal Jupyter frontend" using `amalthea::fixtures::dummy_frontend`
   (public module, `crates/amalthea/src/lib.rs:11`) — exactly like the test suite
   (`crates/ark_test/src/dummy_frontend.rs:1947`, `:898`).
3. Opens the `"lsp"` comm, learns the TCP port, connects.
4. Pumps bytes both ways: stdin→TCP, TCP→stdout. (LSP framing is identical on both
   transports, so this is a dumb pipe.)
5. Keeps draining iopub in the background (bounded channels! `start.rs:68`) and
   handles shutdown: stdin EOF → Jupyter `shutdown_request` → kill child on timeout.

- ✅ Zero changes to ark's existing code paths — the kernel child runs stock code
  exercised by the entire integration test suite and by Positron daily.
- ✅ Full feature parity automatically (real R session, real comms, oak, everything).
- ✅ Perfect stdout hygiene by construction: R lives in the child process; the bridge's
  stdout carries only LSP bytes.
- ✅ Process cleanup partially built already: on Linux the kernel exits when its parent
  dies (`start.rs:161`).
- ✅ Decomposes into small, independently testable tasks (see [tasks/](tasks/)).
- ✅ Later bonus: the same trick can expose ark's **DAP** to Zed.
- ❌ Two processes and loopback ZMQ+TCP — more moving parts at runtime (though all stock).
- ❌ Depends on a module named `fixtures` for production use; if upstreaming later,
  that plumbing should be promoted to a proper `amalthea` client API.

## Option C — LSP without R ("static only") — rejected

Start only the tower-lsp part, never boot R. Rejected outright: `r_task()`
(`crates/ark/src/r_task.rs:272`) blocks until R exists, so completions, hover,
signature help and diagnostics would hang or need invasive feature-gating across
dozens of call sites. The oak static engine is impressive but not yet a full
replacement — package completions, help, etc. still consult the session.

## Recommendation

**Option B for v1** — it converts an architecture problem into a plumbing problem,
and every pipe already exists in this repo. Keep **Option A as the documented
long-term shape** (potential upstream contribution); the CLI name `ark lsp` is chosen
so its *observable behavior* (LSP on stdio) stays stable if the internals are ever
swapped from "sidecar bridge" to "native standalone".

Formalized in [decisions/0002](decisions/0002-sidecar-kernel-architecture.md) and
[decisions/0003](decisions/0003-ark-lsp-subcommand.md) — awaiting Dagmar's approval.
