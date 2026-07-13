# R (Ark) — Zed extension

A [Zed](https://zed.dev) extension that adds editing support for the R language,
intended to be backed by [Ark](https://github.com/posit-dev/ark)'s language
server.

## What this contributes

- the [`r-lib/tree-sitter-r`](https://github.com/r-lib/tree-sitter-r) grammar,
- syntax highlighting, bracket matching, indentation, and an outline for `.R`
  and `.r` files,
- the **Ark language server** — Zed launches `ark lsp` for completions, hover
  documentation, diagnostics, and jump-to-definition.

## Configuration

The extension finds the `ark` binary in one of two ways:

1. **A Zed setting** (`lsp.ark.binary`) — takes priority. Point it at the `ark`
   you built. Assuming this repository is checked out at `/path/to/ark`, add the
   following to your Zed `settings.json` (command palette → **`zed: open
   settings`**):

   ```jsonc
   {
     "lsp": {
       "ark": {
         "binary": {
           "path": "/path/to/ark/target/debug/ark",
           "arguments": ["lsp"]
         }
       }
     }
   }
   ```

   `path` and `arguments` are both honored as written; you can also add a
   `binary.env` object of extra environment variables.

2. **`ark` on your `$PATH`** — if no `lsp.ark.binary.path` is set, the extension
   runs `ark lsp` using whichever `ark` your shell finds. Put the build on your
   PATH, e.g. `export PATH="/path/to/ark/target/debug:$PATH"`. The server
   inherits your shell environment, so `PATH` and `R_HOME` resolve the same R
   you get in a terminal.

If neither is available, Zed's language-server log shows an error explaining how
to build `ark` or set the override.

### Checking it's alive

Open a `.R` file, then run **`zed: open language server logs`** from the command
palette and pick **Ark**. A healthy server logs its LSP handshake there; the
error message above appears instead if the binary could not be found.

### Logging to a file (`--log`)

`ark lsp` writes its diagnostics to stderr by default. To capture them to a
file, add `--log` to `arguments`:

```jsonc
"arguments": ["lsp", "--log", "/tmp/ark-lsp.log"]
```

The bridge process writes to that file, and the R kernel it manages writes to
`kernel.log` in the **same directory** (here `/tmp/kernel.log`). Without `--log`,
the bridge logs to stderr (visible in Zed's language-server log) and the kernel
log goes to a private temp directory. Run `ark lsp --help` for the full option
list. Note that `stdout` is reserved for LSP frames — logs never go there.

## Developing / installing locally

1. Have a recent Rust toolchain with the `wasm32-wasip2` target installed:
   `rustup target add wasm32-wasip2`. Zed compiles the extension to WASM with
   your toolchain and may not add the target for you — without it, the install
   fails with ``error[E0463]: can't find crate for `core` `` (visible in Zed's
   log, `~/.local/share/zed/logs/Zed.log`).
2. In Zed, open the command palette and run **`zed: install dev extension`**.
3. Select this directory (`editors/zed/`).
4. Build `ark` (`cargo build` at the repository root) and configure the binary
   as described under [Configuration](#configuration).
5. Open any `.R` file — highlighting, bracket matching, and the outline panel
   should work, and the Ark language server should provide completions, hover,
   diagnostics, and jump-to-definition.
6. After editing the extension, run **`zed: reload extensions`** (or reinstall
   the dev extension) to pick up changes. After rebuilding `ark`, restart the
   language server (or the Zed window) so Zed relaunches the new binary.

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
