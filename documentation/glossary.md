# Glossary

One-liners for every term the other docs use. Skim once, come back when needed.

## Rust

- **crate** — Rust's unit of compilation and distribution; either a *library* (code others use) or a *binary* (an executable with a `main()`).
- **workspace** — a group of crates developed together, sharing one `Cargo.lock` and build directory. This repo is one workspace with ~20 crates under `crates/`.
- **Cargo.toml** — a crate's manifest: name, dependencies, features. The workspace has a root one too.
- **feature (cargo)** — a named compile-time switch for a crate, e.g. `stdext`'s `testing` feature turns on test-only behavior.
- **trait** — Rust's interface concept. "Implementing trait X for type Y" = Y promises to provide X's methods. Example: ark's `Lsp` implements amalthea's `ServerHandler` trait.
- **channel** — a queue for sending values between threads. One end sends (`tx`), the other receives (`rx`). ark uses these heavily (`crossbeam`, `tokio::sync::mpsc`).
- **bounded channel** — a channel with limited capacity; senders **block** when it's full. Important: if nobody drains a bounded channel, the sender freezes.
- **async / tokio** — cooperative concurrency. `tokio` is the runtime that schedules async tasks onto a thread pool. ark's LSP runs inside a tokio runtime; R does not.
- **`unwrap()` / `panic!`** — crash the program on error. This repo's style avoids `unwrap()` (see `CLAUDE.md`).
- **clippy** — Rust's linter (`just clippy`). **rustfmt** — the formatter (`cargo +nightly fmt --all`).
- **nextest** — the test runner used here (`just test`); each test runs in its own process.

## Processes

- **process / parent / child** — a running instance of a program; a *child* is a process started by another one (its *parent*). The parent keeps a handle to the child: it can wait for it to exit, or kill it.
- **spawn** — start a child process. The v1 bridge spawns a second copy of its own binary (`std::env::current_exe()`) as the kernel child.
- **orphan** — a child whose parent died or forgot to clean it up; it keeps running unattended. The bridge owning the kernel's lifecycle exists to prevent orphaned R sessions.

## Protocols

- **LSP (Language Server Protocol)** — the editor↔language-brain protocol behind completions, hover, go-to-definition, diagnostics. JSON-RPC messages with `Content-Length:` headers over a byte stream.
- **stdio transport** — running the LSP over the server process's stdin/stdout. This is how Zed talks to every language server. Consequence: *nothing else* may write to stdout, or the stream corrupts.
- **JSON-RPC** — "call a method with params, get a result" encoded as JSON; LSP and Jupyter comms build on this idea.
- **Jupyter kernel** — a process that executes code for a frontend (notebook, console, IDE) over the Jupyter wire protocol. ark is one, for R.
- **connection file** — a small JSON file telling a kernel which ports and secret key to use for its sockets.
- **loopback (127.0.0.1 / localhost)** — the virtual network interface that never leaves the machine; packets "loop back" inside the OS. Ports bound to it are unreachable from other computers. A *loopback connection file* is one whose `ip` is 127.0.0.1.
- **ZeroMQ (ZMQ)** — the messaging library carrying Jupyter traffic over 5 sockets: **shell** (requests), **iopub** (broadcasts: output, status), **stdin** (kernel asks user for input), **control** (interrupt/shutdown), **heartbeat**.
- **shell (Jupyter socket)** — *not* bash/zsh. The main request→reply queue of a kernel (name is an IPython-era relic: it backed the interactive "shell" prompt). Carries `execute_request` and comm traffic; executions queue in strict order, which is why interrupt/shutdown travel on the separate **control** socket instead.
- **comm** — Jupyter's side-channel mechanism: frontend and kernel open named custom channels ("comm targets"). Positron uses a comm named `"lsp"` to ask ark to start its LSP.
- **DAP (Debug Adapter Protocol)** — LSP's sibling for debuggers. ark has one too (out of scope for v1).
- **kernelspec** — a registration file telling Jupyter frontends "an R kernel called ark exists, here's how to launch it" (`ark --install` writes it).

## This repo

- **ark** — the main crate: R Jupyter kernel + LSP + DAP in one binary.
- **amalthea** — the in-house framework implementing the Jupyter protocol (sockets, messages, comms).
- **harp** — safe Rust wrappers around R objects and R's C API.
- **libr** — raw bindings to R, loaded at runtime with `dlopen` (so ark doesn't link against one fixed R).
- **oak_\*** — a newer family of crates doing *static analysis* of R code (parsing, scopes, name resolution) so the LSP relies less on the live R session over time.
- **tower-lsp** — the Rust library ark uses to implement the LSP server plumbing.
- **R main thread** — R must run on the process's main thread; everything else (LSP, sockets) lives on background threads.
- **`r_task`** — ark's bridge: background threads submit closures to be executed on the R thread. Blocks until R is up — this is *the* reason the LSP needs a live R session.
- **R_HOME** — the directory of an R installation; ark locates it via the `R` on `PATH` or the `R_HOME` env var.

## Zed

- **Zed extension** — a Rust crate compiled to **WebAssembly (WASM)** that Zed loads; declares grammars and language servers via `extension.toml`.
- **tree-sitter** — the parsing framework editors use for syntax highlighting/structure; **grammar** = the parser for one language (R's lives at `r-lib/tree-sitter-r`).
- **queries (`.scm` files)** — pattern files that map tree-sitter parse nodes to editor concepts (`highlights.scm`, `indents.scm`, …).
- **worktree** — Zed's term for an open project folder.
- **dev extension** — a local, unpublished extension installed via the `zed: install dev extension` command; Zed compiles it from source on your machine.
- **sidecar** — our term for a helper process launched alongside another (here: a full ark kernel running next to the LSP bridge).
