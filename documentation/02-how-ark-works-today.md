# 02 — How ark works today (guided code tour)

Everything here was verified against the code on 2026-07-11. File references are
clickable in most editors: `path:line`.

## The crates, in one table

| Crate | What it is |
|---|---|
| `ark` | the product: one binary that is an R Jupyter kernel + LSP + DAP |
| `amalthea` | Jupyter protocol framework (ZMQ sockets, wire messages, comms) |
| `harp` | safe wrappers over R's C API (also: finds R installations) |
| `libr` | raw R bindings, `dlopen`ed at runtime |
| `ark_test` | test utilities: boots a real kernel in-process and plays "fake frontend" |
| `oak_*` (~10 crates) | newer static-analysis engine for R (parsing, scopes, resolution) feeding LSP features without asking the live R session; `oak_r_process` even spawns short-lived R subprocesses to scan installed packages |
| `echo`, `stdext`, `harp_macros`, `ark_macros`, `aether_path` | toy kernel, std extensions, proc-macros, path utils |

## One process, many threads

When Positron launches ark, a single process holds:

```
┌─────────────────────────── ark process ───────────────────────────┐
│  main thread: R interpreter (must own the main thread)            │
│  amalthea threads: 5 ZMQ sockets ⇄ Positron (Jupyter protocol)    │
│  LSP threads: tokio runtime running tower-lsp        ← our target │
│  everything talks through channels                                │
└────────────────────────────────────────────────────────────────────┘
```

## Startup sequence (kernel)

1. `crates/ark/src/main.rs:79` — hand-rolled flag parsing (`--connection_file`,
   `--session-mode console|notebook|background`, `--startup-file`, `--log`, `--install`, …).
   Note `main.rs:440`: without `--connection_file` there is nothing ark can do today.
2. `crates/ark/src/start.rs:34` `start_kernel()`:
   - `start.rs:44` — locate R via `harp::command::r_home_setup()` (env `R_HOME` or `R` on `PATH`).
   - `start.rs:68-129` — create ~8 channel pairs wiring R ⇄ Jupyter machinery.
     Several are **bounded** (e.g. `iopub` at capacity 10, `start.rs:68`): if nobody
     drains them, the sender blocks. Remember this; it matters for our design.
   - `start.rs:133-135` — register "server handlers": a map `{"lsp" → Lsp, "ark_dap" → Dap}`.
     These are lazy: nothing starts until a frontend asks via a comm.
   - `start.rs:137` — `amalthea::kernel::connect()`: open the ZMQ sockets from the connection file.
   - `start.rs:166` — `Console::start(...)`: boot R **on the main thread**. Blocks forever;
     this call *is* the R session.
3. `start.rs:161` — Linux-only nicety: parent-process monitoring, so ark exits if
   whoever spawned it dies. Useful later.

## How the LSP starts (the part we must replicate)

The LSP does **not** start with the kernel. It starts when the frontend asks:

1. Positron sends a Jupyter `comm_open` on the shell socket with target `"lsp"` and
   data `{"ip_address": "127.0.0.1"}`.
2. Amalthea routes that to `Lsp::start` — `crates/ark/src/lsp/handler.rs:60`
   (the `ServerHandler` trait impl). It first blocks until R finished initializing
   (`handler.rs:71`, waits on the `kernel_init` bus).
3. `crates/ark/src/lsp/backend.rs:553` `start_lsp()`:
   - `backend.rs:564` — binds a TCP listener on port **0** (OS picks a free port),
   - `backend.rs:580` — sends the port number back over the comm (`server_started` message),
   - `backend.rs:586` — waits for **the client to connect to it**,
   - `backend.rs:635` — serves LSP over that one TCP stream via tower-lsp.

So today's transport is **TCP with reversed roles** (server listens, editor connects),
negotiated over a Jupyter comm. Zed can do none of that: it spawns a process and
speaks LSP over **stdio**. That's gap #1.

## Why the LSP needs a live R session (gap #2)

The LSP is not a static analyzer only — many features call into the running R
interpreter through `r_task()` (`crates/ark/src/r_task.rs:272`), which queues a
closure onto the R main thread **and blocks until R exists**. Counts of `r_task`
references in LSP code (2026-07-11):

| File | r_task refs | Used for |
|---|---|---|
| `lsp/diagnostics.rs` | 23 | checks against session state |
| `lsp/signature_help.rs` | 11 | live function signatures |
| `lsp/completions/sources/unique/custom.rs` + others | ~35 across sources | package exports, namespaces, call args |
| `lsp/help.rs` | 5 | hover documentation from R's help system |

Also, the LSP's init hook registers itself with the R console:
`backend.rs:605` runs `r_task(|| Console::get_mut().set_lsp_channel(...))` so console
activity can refresh LSP state. And `GlobalState::new` takes `r_home` (`backend.rs:593`)
so the oak machinery can spawn R subprocesses for package metadata.

**Conclusion:** a serious Zed integration must bring a full R session with it.
An "LSP without R" would block forever on the first `r_task`.

## The cheat code: the test suite already does what we need

`ark_test` boots the entire kernel **without Positron**:

- `crates/ark_test/src/dummy_frontend.rs:1947-1990` — generates a loopback connection
  file (via `amalthea::fixtures::dummy_frontend::DummyConnection`, a public module:
  `crates/amalthea/src/lib.rs:11`) and runs `start_kernel()` with it.
- `dummy_frontend.rs:898-933` — `start_lsp()`: sends the `comm_open` with
  `{"ip_address": "127.0.0.1"}`, reads the `server_started` reply to learn the port,
  connects a TCP LSP client. The whole LSP integration test suite
  (`crates/ark/tests/integration/lsp*.rs`) runs through this path.

In other words: **"boot ark + open its LSP without Positron" is already proven code** —
it just lives in test clothing. Design option B in [04](04-design-options.md) reuses
exactly this recipe.

## Where LSP features live (for later reference)

- Request routing: `lsp/backend.rs` (tower-lsp surface) → `lsp/main_loop.rs` (the
  actual state + handlers) → `lsp/handlers.rs` and feature modules
  (`completions/`, `hover.rs`, `goto_definition.rs`, `diagnostics.rs`, …).
- Newer features increasingly call into `oak_ide` (`crates/oak_ide/src/lib.rs`):
  goto-definition, find-references, rename are static-analysis based already.
