# Documentation — ark LSP in Zed

This folder is the source of truth for the "use ark's R language server in Zed" project.
Nothing gets changed in the codebase before it is written down and agreed here.

## Who does what

| Role | Who | Responsibility |
|---|---|---|
| Owner / reviewer | Dagmar | Reads the docs, approves decisions, follows along, learns Rust on the way |
| Architect / teacher | Claude (Fable) | Explores the code, writes docs and task briefs, explains everything, delegates. **Writes no code.** |
| Implementers | Smaller models (e.g. Opus 4.8) | Execute one task brief at a time, leave a diff for joint review |

## The workflow

```
 1. Scope docs          01–04 explain the world as it is and the design choice
 2. Decision records    decisions/NNNN-*.md — one decision per file
        status: Proposed  →  (Dagmar reads & approves)  →  Accepted
 3. Task briefs         tasks/TNN-*.md — small, self-contained work packages
 4. Delegation          a task is handed to an implementer model, verbatim
 5. Review              we read the diff together; Dagmar learns, Claude explains
 6. Done                task marked Done, decision marked Implemented
```

Rules:

- **No code change without an Accepted decision** that covers it.
- **Every task brief must be self-contained**: an implementer model gets the brief
  and the repo, nothing else. If a brief needs "tribal knowledge", the brief is broken.
- Implementer models never commit or push; they leave changes in the working tree.
- When reality contradicts a doc, the doc gets fixed in the same breath.

## Reading order

| File | What it answers |
|---|---|
| [01-goal-and-scope.md](01-goal-and-scope.md) | What are we building, what counts as done, what's out of scope |
| [02-how-ark-works-today.md](02-how-ark-works-today.md) | How ark starts R and the LSP today (guided code tour) |
| [03-what-zed-needs.md](03-what-zed-needs.md) | How Zed language extensions work, what we must provide |
| [04-design-options.md](04-design-options.md) | The three possible architectures and the recommendation |
| [glossary.md](glossary.md) | Every piece of jargon used in these docs, one line each |
| [decisions/](decisions/) | One file per decision, with status |
| [tasks/](tasks/) | The work, split into delegable packages |
| [teaching/](teaching/) | Q&A deep-dives written up as we go (01: LSP communication from Zed's side; 02: the Positron↔ark side) |

## Status at a glance

- 2026-07-11 — Project started. Codebase explored, scope docs written,
  decisions 0002–0005 are **Proposed and waiting for Dagmar's review**.
- 2026-07-12 — Decisions 0002–0005 **Accepted**. T00 done. T01 (`ark lsp`
  skeleton) implemented and committed (`40f96503`). T02 (sidecar kernel boot)
  **Done** (committed `04f70346`). T03 (LSP comm + TCP connect) **Done**
  (committed `94ce68ac`). T04 (stdio bridge + lifecycle) **Done** (committed
  `6272fbed`) — the R-side bridge (T02–T04) is complete; `ark lsp` is a working
  standalone stdio language server. Next: the Zed extension half (T05 Ready, T06).
