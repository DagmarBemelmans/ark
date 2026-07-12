# Tasks — the work, split for delegation

Each `TNN-*.md` file is a **self-contained brief** for an implementer model
(e.g. Opus 4.8). The implementer receives the brief text plus the repo — nothing else.

## Status board

| # | Task | Depends on | Risk | Status |
|---|---|---|---|---|
| T00 | [Environment & baseline](T00-environment-and-baseline.md) | — | low | Done (Zed REPL check skipped) |
| T01 | [`ark lsp` CLI skeleton](T01-cli-subcommand-skeleton.md) | ~~0003 accepted~~ ✓ | low | In review |
| T02 | [Sidecar kernel boot](T02-sidecar-kernel-boot.md) | T01, ~~0002 accepted~~ ✓ | **high** | Blocked on T01 |
| T03 | [LSP comm + TCP connect](T03-lsp-comm-and-tcp-connect.md) | T02 | medium | Blocked |
| T04 | [stdio bridge + lifecycle](T04-stdio-bridge-and-lifecycle.md) | T03 | medium | Blocked |
| T05 | [Zed extension scaffold](T05-zed-extension-scaffold.md) | ~~0004+0005 accepted~~ ✓ | low | Ready |
| T06 | [Zed language-server wiring](T06-zed-language-server-wiring.md) | T04, T05 | low | Blocked |
| T07 | [Docs, troubleshooting, polish](T07-docs-and-polish.md) | T06 | low | Blocked |

Statuses: `Blocked` → `Ready` → `Delegated` → `In review` → `Done`.
T05 can run in parallel with T02–T04 (different half of the system).

## Delegation protocol

1. Dagmar says "go" on a Ready task.
2. Claude spawns a subagent (Opus-class) whose prompt is the brief file, verbatim,
   plus: "Follow `/home/dagmar/projects/ark/CLAUDE.md`. Do not commit, push, or touch
   files outside the brief's Deliverables. If an Escalate-if condition triggers, stop
   and report instead of working around it."
3. The subagent leaves changes in the working tree and a short report.
4. Dagmar + Claude review the diff together (`git diff`), Claude explains every hunk.
5. On acceptance: Dagmar commits; the board above and any affected docs are updated.

## Definition of Done (every code task)

- [ ] `cargo build` succeeds
- [ ] `just clippy` clean
- [ ] `cargo +nightly fmt --all` applied
- [ ] The brief's acceptance criteria demonstrably pass (commands included in brief)
- [ ] New/changed behavior covered by a test where the brief requires one
- [ ] No changes outside the brief's Deliverables list
- [ ] `documentation/` updated if the change makes any doc stale
