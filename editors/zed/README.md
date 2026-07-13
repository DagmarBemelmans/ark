# R (Ark) — Zed extension

A [Zed](https://zed.dev) extension that adds editing support for the R language,
intended to be backed by [Ark](https://github.com/posit-dev/ark)'s language
server.

## Status

**Highlighting only (task T05).** This build contributes:

- the [`r-lib/tree-sitter-r`](https://github.com/r-lib/tree-sitter-r) grammar,
- syntax highlighting, bracket matching, indentation, and an outline for `.R`
  and `.r` files.

The language server (launching `ark` in LSP mode for completions,
diagnostics, jump-to-definition, …) is **not wired up yet** — it arrives in task
T06. Until then, opening an R file and checking the language server logs will
show an explanatory error from the extension; that is expected.

If you want to experiment with the server before T06 lands, point Zed at an
`ark` binary yourself in your Zed `settings.json`:

```jsonc
{
  "lsp": {
    "ark": {
      "binary": {
        "path": "/absolute/path/to/ark",
        "arguments": ["lsp"]
      }
    }
  }
}
```

## Developing / installing locally

1. Have a recent Rust toolchain with the `wasm32-wasip2` target installed:
   `rustup target add wasm32-wasip2`. Zed compiles the extension to WASM with
   your toolchain and may not add the target for you — without it, the install
   fails with ``error[E0463]: can't find crate for `core` `` (visible in Zed's
   log, `~/.local/share/zed/logs/Zed.log`).
2. In Zed, open the command palette and run **`zed: install dev extension`**.
3. Select this directory (`editors/zed/`).
4. Open any `.R` file — highlighting, bracket matching, and the outline panel
   should work.
5. After editing the extension, run **`zed: reload extensions`** (or reinstall
   the dev extension) to pick up changes.

This extension lives outside the Ark cargo workspace (its `Cargo.toml` has an
empty `[workspace]` table), so a plain `cargo build` at the repository root does
not build it — that is Zed's job during "install dev extension".

## Attribution / NOTICE

The tree-sitter query files in `languages/r/` (`highlights.scm`, `brackets.scm`,
`indents.scm`, `outline.scm`) were copied from
[**ocsmit/zed-r**](https://github.com/ocsmit/zed-r) (commit `87438f0`), which is
distributed under the MIT License. Each file keeps an attribution header
pointing back here. We also reuse zed-r's tree-sitter grammar pin
(`r-lib/tree-sitter-r` @ `a0d3e3307489c3ca54da8c7b5b4e0c5f5fd6953a`).

The grammar itself, [`r-lib/tree-sitter-r`](https://github.com/r-lib/tree-sitter-r),
is also MIT licensed and is fetched and compiled by Zed at install time.

### MIT License (ocsmit/zed-r)

```
MIT License

Copyright (c) 2026 Owen Smith

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
