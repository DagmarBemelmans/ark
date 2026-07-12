# 01 — Goal and scope

## Goal

Open an `.R` file in Zed and get real intellisense from **this repo's ark LSP**:
completions, hover help, go-to-definition, find-references, rename, and diagnostics —
the same engine Positron uses.

## Why ark, and not the existing R language server

The existing Zed R extension (ocsmit/zed-r) wires up
[languageserver](https://github.com/REditorSupport/languageserver), and Dagmar's
experience with it is the motivation for this project (recorded 2026-07-11):

1. **It is very slow.** By construction: it's written in R, running inside a
   single-threaded R session, re-analyzing code with R-level tooling. ark's LSP is
   Rust across a thread pool, and its analysis engine (the `oak_*` crates) is built
   on salsa — the incremental-computation framework from the rust-analyzer lineage —
   so edits recompute only what they invalidate (root `Cargo.toml:101`).
2. **It can't handle R packages properly.** ark's engine models packages as
   first-class objects: `oak_db`'s `Package` entity is keyed to the `DESCRIPTION`
   file, parses `NAMESPACE`, respects `Collate:` load order per *Writing R
   Extensions* §1.1.1, and distinguishes the loadable `R/` files from
   `tests/`/`inst/` scripts (`crates/oak_db/src/package.rs:16-84`). A workspace
   scanner discovers packages at any depth and a library scanner indexes installed
   packages the same way (`crates/oak_scan/src/packages.rs`). This is where the
   repo's active development is happening (see recent salsa/oak commit history).

Caveat, honestly held: oak is under active development, and some LSP features still
consult the live R session rather than the static index. Which features fall where
gets verified empirically in T06.

## What already works today (no code needed)

Zed's REPL feature runs code through Jupyter kernels, and ark *is* a Jupyter kernel.
Zed's own docs list ark as the way to run R. So after building this repo and running
`ark --install`, you can already **execute R code in Zed** (`# %%` cells, ctrl-shift-enter).

What does **not** work is the editing intelligence: Zed has no way to reach ark's LSP,
because ark only exposes it to Positron (see [02](02-how-ark-works-today.md)).
That gap is this project.

## Success criteria (v1 is "done" when…)

- [ ] Zed opens `.R` files with syntax highlighting (tree-sitter grammar wired up).
- [ ] A locally built `ark` binary is started by Zed and speaks LSP over stdio.
- [ ] Completions work, including for functions from installed packages (e.g. typing `dplyr::` offers `mutate`).
- [ ] Hover shows function documentation.
- [ ] Go-to-definition and find-references work for symbols defined in the open project.
- [ ] In an R *package* checkout specifically: navigation and completions work across
      the files in `R/`, including the package's own unexported functions — the
      package-handling pain point that motivated this project.
- [ ] Diagnostics (ark's lints/syntax errors) appear in the editor.
- [ ] Closing Zed (or the file) does not leave orphaned R processes behind.
- [ ] Everything above is reproducible from a written walkthrough doc.

## In scope

1. A way to start ark's LSP **without Positron**, speaking **stdio** (Zed's requirement).
2. A **Zed extension** for R that launches it.
3. Documentation and task briefs so the work is delegable and reviewable.

## Out of scope for v1 (deliberately)

| Item | Why deferred |
|---|---|
| R Markdown / Quarto support | needs language injection setup in Zed; R-only first |
| Formatting | Posit's [Air](https://posit-dev.github.io/air/editor-zed.html) already ships a Zed extension for that; they can coexist |
| Debugging (ark's DAP in Zed) | Zed's debugger extension API is newer territory; great phase-2 candidate |
| Windows/macOS testing | Dagmar develops on Linux; other OSes once v1 works |
| Publishing the extension to Zed's registry | dev-extension install is enough to use it ourselves |
| Upstreaming to posit-dev/ark | possible later; we build in a way that doesn't foreclose it |
| Sharing one R session between the LSP and Zed's REPL | in v1 they are separate ark processes: variables created in REPL cells won't appear in completions (unlike Positron, where console and LSP share one process). Unifying them is a v2-sized project. |

## Constraints on *how* we work

- Dagmar is a Rust beginner: docs and reviews must explain, not just state.
- Docs-first: every decision is recorded in [decisions/](decisions/) before code changes.
- Claude (Fable) writes no code; implementation is delegated per task brief to smaller models.

## Known risks

| Risk | Impact | Mitigation |
|---|---|---|
| ark's LSP is useless without a live R session (`r_task` blocks) | an "LSP-only" mode would hang or lose most features | chosen design boots a real R session (see [04](04-design-options.md)) |
| Anything printed to stdout corrupts the LSP stream | Zed shows a dead/hung server | bridge process keeps stdout exclusively for LSP; logs go to stderr/file; R lives in a separate child process |
| Zed runs one language server per project window | one R session per Zed project ⇒ RAM cost | acceptable for v1; document it |
| Zed extension API evolves | build breakage at implementation time | briefs say "check current docs/versions" instead of hardcoding |
| `SessionMode::Background` (headless) is less exercised than Console mode | startup quirks | briefs include an escalation clause: report instead of hacking around |
