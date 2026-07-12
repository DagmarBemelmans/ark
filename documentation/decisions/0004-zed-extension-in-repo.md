# 0004 — Zed extension lives in this repo under `editors/zed/`

- **Status:** Accepted (Dagmar, 2026-07-12)
- **Date:** 2026-07-11
- **Decided by:** Dagmar
- **Related:** [../03-what-zed-needs.md](../03-what-zed-needs.md), tasks T05–T06

## Context

The extension is a separate Rust crate compiled to WASM by Zed. It could live in its
own repository or inside this one. Zed's "install dev extension" takes any local path.
Precedent: Posit keeps Air's Zed extension alongside the tool
(<https://posit-dev.github.io/air/editor-zed.html>).

## Decision

Keep it in-repo at `editors/zed/`, **outside the cargo workspace** (its `Cargo.toml`
gets an empty `[workspace]` table so `cargo` doesn't try to adopt it; the workspace
globs only `crates/*`, so no conflict).

Alternative: separate repo — cleaner separation, but splits docs/reviews/history for a
one-person learning project. Easy to extract later if the extension is ever published.

## Consequences

- One repo to clone, one documentation folder, one review flow.
- `cargo build` at the root ignores the extension; building it is Zed's job during
  "install dev extension".
- Anything borrowed from ocsmit/zed-r (MIT) carries attribution in `editors/zed/`
  (see [0005](0005-grammar-and-queries-source.md)).
