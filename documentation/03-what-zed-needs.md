# 03 — What Zed needs from us

Verified against Zed's docs and existing extensions on 2026-07-11. Zed's APIs move;
task briefs re-check details at implementation time.

## How Zed gets language support

Zed has no built-in R support. Languages arrive via **extensions**: small Rust crates
compiled to WebAssembly that Zed loads. An extension contributes up to three things:

1. **A tree-sitter grammar** — for highlighting/structure. Declared in `extension.toml`,
   pinned to a git commit; Zed fetches and compiles it itself.
2. **Language metadata + queries** — `languages/r/config.toml` plus `.scm` query files
   (`highlights.scm`, `brackets.scm`, `indents.scm`, `outline.scm`, …).
3. **A language server hookup** — Rust code answering "what command should I run?".
   Zed then spawns that command and speaks **LSP over the process's stdin/stdout**.

Key consequence for us: whatever we launch must keep **stdout byte-clean for LSP**.

## Anatomy of the extension we'll build

```
editors/zed/                     (proposed location, see decision 0004)
├── extension.toml               id, name, grammar pin, language server declaration
├── Cargo.toml                   crate-type = ["cdylib"], dep: zed_extension_api
│                                + an empty [workspace] table (must NOT join ark's workspace)
├── src/lib.rs                   impl zed::Extension → language_server_command()
└── languages/r/
    ├── config.toml              name = "R", grammar = "r", path_suffixes = ["R", "r"], line_comments = ["# "]
    ├── highlights.scm           ┐
    ├── brackets.scm             │ borrowed from ocsmit/zed-r (MIT) with attribution,
    ├── indents.scm              │ or written fresh against r-lib/tree-sitter-r
    └── outline.scm              ┘
```

`extension.toml` core pieces (shape confirmed from Zed's docs and zed-r):

```toml
id = "ark-r"
name = "R (Ark)"
version = "0.0.1"
schema_version = 1

[grammars.r]
repository = "https://github.com/r-lib/tree-sitter-r"
rev = "a0d3e3307489c3ca54da8c7b5b4e0c5f5fd6953a"   # pin used by zed-r v0.2.6; re-check at impl time

[language_servers.ark]
name = "Ark"
languages = ["R"]

[language_servers.ark.language_ids]
"R" = "r"
```

The Rust side implements one trait method (signature per Zed docs):

```rust
fn language_server_command(
    &mut self,
    language_server_id: &LanguageServerId,
    worktree: &zed::Worktree,
) -> Result<zed::Command>   // Command { command, args, env }
```

Our logic: honor a user-configured binary path from Zed settings if present,
otherwise `worktree.which("ark")`, and pass the argument that starts LSP mode.
Users can also override per Zed's standard settings mechanism:

```jsonc
// Zed settings.json
{
  "lsp": {
    "ark": {
      "binary": {
        "path": "/home/dagmar/projects/ark/target/debug/ark",
        "arguments": ["lsp"]
      }
    }
  }
}
```

## The development loop

1. Build ark: `cargo build` (binary at `target/debug/ark`).
2. In Zed: command palette → **`zed: install dev extension`** → select `editors/zed/`.
   Zed compiles the extension to WASM with your Rust toolchain.
3. Open a folder with `.R` files; check the language server status/logs
   (command palette: "language server logs").
4. Iterate: rebuild ark and/or the extension, then "rebuild dev extension" / reopen.

## Prior art (useful, none solves our problem)

- **[ocsmit/zed-r](https://github.com/ocsmit/zed-r)** (MIT, active, v0.2.6 Jan 2026) —
  R support wired to the CRAN `languageserver` package. We reuse its structure,
  grammar pin, and (with attribution) its queries — but swap the server for ark.
- **[Air's Zed extension](https://posit-dev.github.io/air/editor-zed.html)** — Posit's
  R *formatter* LSP for Zed. Complementary; can run alongside ours later.
- **[Zed REPL docs](https://zed.dev/docs/repl)** — ark is already Zed's documented way
  to *run* R code (Jupyter kernelspec, no extension involved). Our project adds the
  editing intelligence on top.
- **[Zed language extension docs](https://zed.dev/docs/extensions/languages)** — the
  reference for everything in this file.

## Hard requirements Zed imposes (summary)

| Requirement | Consequence for ark |
|---|---|
| Editor spawns the server as a child process | ark needs a launchable "LSP mode" (today: comm + TCP only) |
| LSP over stdio | stdout must carry nothing but LSP frames; logging → stderr/file |
| One server instance per project window | each Zed project gets its own R session (RAM cost, document it) |
| Extension = WASM sandbox | the extension can't do ZMQ/TCP itself; all cleverness lives in the ark binary |
