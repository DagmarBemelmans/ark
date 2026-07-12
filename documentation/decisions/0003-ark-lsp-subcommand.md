# 0003 — Expose the bridge as an `ark lsp` subcommand

- **Status:** Accepted (Dagmar, 2026-07-12)
- **Date:** 2026-07-11
- **Decided by:** Dagmar
- **Related:** [0002](0002-sidecar-kernel-architecture.md), task T01

## Context

Zed's extension needs one command to run. ark's CLI today is flag-based
(`--install`, `--connection_file`, …; hand-parsed in `crates/ark/src/main.rs:105`)
and has no subcommands.

## Decision

Add a first-argument subcommand: `ark lsp [--log FILE] [-- R_ARGS…]`. When
`argv[1] == "lsp"`, main dispatches to the bridge and never enters kernel parsing.
Everything else stays byte-for-byte compatible (Positron/Jupyter pass `--flags` first,
so no collision).

Alternatives:
- `--lsp` flag — fits existing style but reads worse (`ark --lsp --log …` mixes modes and
  flags); subcommand marks a genuinely different program mode.
- Separate binary (`ark-lsp-bridge`) — one more artifact to build/ship/find on PATH; the
  bridge reuses the same binary as the kernel child anyway (`current_exe()`).

## Consequences

- Zed extension config is simply: command `ark`, args `["lsp"]`.
- The name promises "LSP on stdio", not an implementation: internals may later switch
  from sidecar bridge to native standalone (option A) without breaking editors.
- In `lsp` mode the logger must never default to stdout (today's default can log to
  stdout per `main.rs:56` usage text) — enforced in T01/T04 acceptance criteria.
