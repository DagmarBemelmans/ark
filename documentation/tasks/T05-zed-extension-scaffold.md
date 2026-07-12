# T05 — Zed extension scaffold (highlighting only)

- **Status:** Blocked on decisions 0004 + 0005 (can run parallel to T02–T04)
- **Risk:** low · **Size:** small-medium (~1-2h)
- **Depends on:** decisions [0004](../decisions/0004-zed-extension-in-repo.md), [0005](../decisions/0005-grammar-and-queries-source.md)

## Goal

`editors/zed/` contains a loadable Zed dev extension giving `.R` files syntax
highlighting, brackets, indents, and outline. No language server yet (T06).

## Background

- Layout + manifest shape: `documentation/03-what-zed-needs.md`.
- Reference extension: <https://github.com/ocsmit/zed-r> (MIT). Copy its
  `languages/r/*.scm` queries and `config.toml` as the starting point, with
  attribution. Check its current `extension.toml` for the exact grammar `rev` it pins
  (v0.2.6 pinned `r-lib/tree-sitter-r @ a0d3e3307489c3ca54da8c7b5b4e0c5f5fd6953a`) and
  reuse that pin.
- Zed docs to follow (fetch fresh — the API moves):
  <https://zed.dev/docs/extensions/languages> and
  <https://zed.dev/docs/extensions/developing-extensions>.
- Current `zed_extension_api` version: check <https://crates.io/crates/zed_extension_api>.

## Requirements

1. `editors/zed/extension.toml` — id `ark-r`, name `R (Ark)`, version `0.0.1`,
   grammar pin as above. Declare the language server entry already
   (`[language_servers.ark]`, languages `["R"]`, language_ids `"R" = "r"`); the Rust
   side comes in T06.
2. `editors/zed/Cargo.toml` — `crate-type = ["cdylib"]`, dep `zed_extension_api`
   (latest), and an **empty `[workspace]` table** so it never joins the ark workspace.
   Verify: `cargo build` at the repo root must not compile or complain about
   `editors/zed`.
3. `editors/zed/src/lib.rs` — minimal `zed::Extension` impl (register + a
   `language_server_command` returning a clear "configure in T06" error), so the
   extension compiles.
4. `editors/zed/languages/r/` — `config.toml` (name `R`, grammar `r`,
   `path_suffixes = ["R", "r"]`, `line_comments = ["# "]`) + queries from zed-r with
   MIT attribution headers.
5. `editors/zed/README.md` — what it is, dev-install steps, attribution/NOTICE section.

## Acceptance criteria & verification

- `cargo build` at repo root: unaffected.
- In Zed: `zed: install dev extension` → select `editors/zed/` → succeeds.
- Open `crates/ark/src/modules/positron/` … any `.R` file in this repo: highlighting,
  bracket matching, outline panel populated.
- No language-server errors beyond the intentional "configure in T06" one.

## Deliverables

- everything under `editors/zed/` (new directory); nothing else.

## Escalate if

- Zed rejects the manifest schema (API drift) — capture the exact error and current
  docs, report.
- zed-r's queries fail against the pinned grammar (drift between the two).
