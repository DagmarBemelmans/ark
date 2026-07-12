# 0001 — Work docs-first, with recorded decisions and delegable task briefs

- **Status:** Accepted (mandated by Dagmar, 2026-07-11)
- **Date:** 2026-07-11
- **Decided by:** Dagmar
- **Related:** [../README.md](../README.md)

## Context

Dagmar is a Rust beginner who wants to (a) understand every step, (b) approve every
decision before code changes, and (c) delegate the actual coding to smaller models
(e.g. Opus 4.8) with instructions clear enough that they can't wander off. Claude
(Fable) acts as architect and teacher only and writes no code.

## Decision

All project knowledge lives in `documentation/`. The pipeline is:
scope docs → decision record (Proposed → Accepted by Dagmar) → task brief →
delegated implementation → joint review → statuses updated. No code changes without
an Accepted decision covering them. Implementer models never commit or push.

## Consequences

- Slower start, much cheaper mistakes; every step is teachable and auditable.
- Briefs must be self-contained — writing them is real work, done by Claude.
- Docs that contradict reality are bugs and get fixed immediately.
