# T06 — Wire the Zed extension to `ark lsp`

- **Status:** In review (2026-07-13) — implementer finished; awaiting Dagmar's in-Zed acceptance checks + commit
- **Risk:** low · **Size:** small (~1h)
- **Depends on:** T04 (a working `ark lsp`), T05 (loadable extension)

## Goal

Zed launches our locally built ark and full intellisense works in `.R` files.
This is the task where the two halves meet.

## Background

- `documentation/03-what-zed-needs.md` — `language_server_command` signature,
  `zed::Command { command, args, env }`, settings override example.
- Check the current `zed_extension_api` docs (docs.rs) for the settings API
  (`LspSettings` or equivalent) — name may have drifted.

## Requirements

1. Implement `language_server_command` in `editors/zed/src/lib.rs`, resolution order:
   1. Zed LSP settings override (`lsp.ark.binary.path` / `.arguments`) — respect fully;
   2. `worktree.which("ark")` with args `["lsp"]`;
   3. otherwise `Err` with a message that tells the user exactly what to do
      (build ark, or set the settings override — include the JSON snippet).
2. Pass the worktree's environment through if the API offers it (so `PATH`/`R_HOME`
   resolve the same R the user gets in a terminal).
3. Update `editors/zed/README.md`: configuration section with the settings JSON
   pointing at `<repo>/target/debug/ark`, plus "how to check it's alive"
   (Zed language-server logs) and where `--log` output goes if configured via
   `arguments`.

## Acceptance criteria (manual, in Zed — the payoff moment)

With the dev extension installed and `"lsp": {"ark": {"binary": {"path": ".../target/debug/ark", "arguments": ["lsp"]}}}` in settings:

- [ ] completions: typing `libra` in a `.R` file offers `library`
- [ ] package completions: `dplyr::mut` offers `mutate` (dplyr installed in T00)
- [ ] hover on `mean` shows documentation
- [ ] go-to-definition jumps to a function defined in another file of the project
- [ ] a syntax error produces a squiggle
- [ ] quitting Zed leaves no `ark` processes behind (`ps aux | rg '[a]rk'`)
- [ ] same checks pass with the settings override removed and `target/debug` on `PATH`
      (the `worktree.which` path)

## Deliverables

- `editors/zed/src/lib.rs`, `editors/zed/README.md`, version bump in `extension.toml`.

## Escalate if

- Zed launches the binary but the server never initializes (capture Zed's LSP log +
  `ark lsp --log` output; likely a T04 bug, not an extension bug — report, don't patch here).
