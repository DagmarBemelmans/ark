# 0002 — v1 architecture: sidecar kernel + stdio↔TCP bridge

- **Status:** Accepted (Dagmar, 2026-07-12)
- **Date:** 2026-07-11
- **Decided by:** Dagmar
- **Related:** [../04-design-options.md](../04-design-options.md), [0003](0003-ark-lsp-subcommand.md), tasks T02–T04

## Context

Zed spawns a language server as a child process speaking LSP over stdio. Today ark's
LSP starts only via a Positron comm and serves TCP (`crates/ark/src/lsp/backend.rs:553`),
and it requires a live R session in-process (`r_task`, `crates/ark/src/r_task.rs:272`).
The test suite already boots a full kernel without Positron and connects an LSP client
to it (`crates/ark_test/src/dummy_frontend.rs:898`, `:1947`), using the public
`amalthea::fixtures::dummy_frontend` module.

## Decision

Build v1 as a **bridge mode in the ark binary**: the bridge generates a loopback
connection file, spawns itself as a stock kernel child
(`--connection_file … --session-mode background`), opens the `"lsp"` comm to obtain
the LSP's TCP port, connects, and pumps bytes between its own stdio and that socket.

Alternatives:
- *Native standalone LSP (no Jupyter)* — cleaner but high-risk surgery through
  frontend assumptions and fd hygiene; kept as the long-term shape ([04](../04-design-options.md), option A).
- *LSP without R* — rejected; `r_task` blocks forever without R.

## Consequences

- ark's existing code paths stay untouched; new code is additive and isolated.
- The bridge's stdout is sacred: **only LSP bytes**, logs go to stderr or `--log` file.
- The bridge must continuously drain iopub (bounded channels block otherwise,
  `crates/ark/src/start.rs:68`) and own the child's lifecycle
  (stdin EOF → Jupyter shutdown → kill on timeout; Linux parent-monitor is a backstop, `start.rs:161`).
- One R session per Zed project window; documented as a v1 limitation.
- If this ever goes upstream, promote the needed parts of `amalthea::fixtures` into a
  proper client API first.
