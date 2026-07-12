# T07 — Documentation, troubleshooting, polish

- **Status:** Blocked on T06
- **Risk:** low · **Size:** small (~1h)
- **Depends on:** T06 (working end-to-end)

## Goal

Someone who is not us — or us, in six months — can set this up from scratch in
fifteen minutes, and can debug it when it misbehaves.

## Requirements

1. `documentation/05-walkthrough.md` (new): from `git clone` to completions in Zed,
   every command, expected output at each step, screenshots optional.
2. `documentation/06-troubleshooting.md` (new), covering at least:
   - where the three logs live (bridge `--log`, kernel sidecar log, Zed's LSP log)
     and one line on reading each;
   - "server doesn't start": R not found (`R_HOME`/`PATH`), wrong binary path in
     settings, extension not rebuilt after Zed update;
   - "completions missing for package X": package not installed in the R the sidecar
     found;
   - orphan processes: how to check (`ps aux | rg '[a]rk'`), why they should not
     happen (T02/T04 lifecycle + parent monitor), what to report if they do;
   - known limitations from `documentation/01-goal-and-scope.md` (one R session per
     Zed window; no Rmd/Quarto; no formatting — pointer to Air; no debugging yet).
3. Sweep `documentation/`: statuses on the task board and decisions updated
   (Implemented + commit links), stale statements fixed, success-criteria checklist
   in 01 ticked with dates.
4. `editors/zed/README.md` final pass: quickstart at top, config, attribution intact.

## Acceptance criteria

- Dagmar (or a fresh model given only the walkthrough) reproduces the setup on a
  clean checkout without asking questions.
- Every doc status in `documentation/` reflects reality on the day this task closes.

## Deliverables

- `documentation/05-walkthrough.md`, `documentation/06-troubleshooting.md` (new)
- status/statement updates across `documentation/` and `editors/zed/README.md`
