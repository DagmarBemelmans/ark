# T01 — `ark lsp` CLI subcommand skeleton

- **Status:** Blocked on decision 0003
- **Risk:** low · **Size:** small (~1h)
- **Depends on:** decision [0003](../decisions/0003-ark-lsp-subcommand.md)

## Goal

`ark lsp` becomes a recognized subcommand that parses its options and calls a stub.
No bridge logic yet — this task creates the entry point and the module skeleton so
later tasks have a home.

## Background (read first)

- `crates/ark/src/main.rs:79-105` — current hand-rolled flag loop. The subcommand must
  be detected from the **first** argument, before this loop, and must not disturb any
  existing flag behavior (Positron passes `--connection_file` etc.).
- `crates/ark/src/main.rs:32` — `print_usage()`: extend it to mention the subcommand.
- `crates/ark/src/logger.rs` — `logger::init(log_file, profile_file)`. Check where logs
  go when `log_file` is `None`; in `lsp` mode logs must NEVER go to stdout (stdout will
  carry LSP frames in T04). If the default writes to stdout, route `lsp`-mode default
  to stderr.
- Style rules: `/home/dagmar/projects/ark/CLAUDE.md` (no `bail!`, no `.unwrap()`,
  `anyhow::Result`, callee functions below callers, etc.).

## Requirements

1. New module `crates/ark/src/standalone_lsp/mod.rs` (add `pub mod standalone_lsp;` to
   `crates/ark/src/lib.rs`) with:
   - `pub struct Options { pub log_file: Option<String>, pub r_args: Vec<String> }`
   - `pub fn run(options: Options) -> anyhow::Result<()>` — for now returns
     `Err(anyhow!("ark lsp is not implemented yet"))`.
2. In `main()`: if the first CLI argument is exactly `lsp`, parse the remaining args
   (`--log FILE`, `--help`, `--` + passthrough R args; unknown args → error) into
   `Options`, initialize the logger (stderr default), call `run`, and exit with a
   non-zero code on `Err` (message to **stderr**).
3. `ark lsp --help` prints usage for the subcommand and exits 0.
4. `ark --help` mentions the subcommand in one line.
5. Unit tests for the new arg parsing (pure function taking `Vec<String>` so it's
   testable without spawning processes).

## Acceptance criteria & verification

```sh
cargo build
./target/debug/ark lsp --help        # exits 0, prints subcommand usage
./target/debug/ark lsp               # exits non-zero, stderr: not implemented yet
./target/debug/ark --help            # still works, mentions `lsp`
./target/debug/ark --version         # unchanged
just test -p ark standalone_lsp      # new parsing tests pass
just clippy && cargo +nightly fmt --all
```

## Deliverables (only these files)

- `crates/ark/src/main.rs` (dispatch + usage line)
- `crates/ark/src/lib.rs` (one `pub mod` line)
- `crates/ark/src/standalone_lsp/mod.rs` (new)
- possibly `crates/ark/src/logger.rs` (only if the no-file default writes to stdout)

## Escalate if

- The first-argument dispatch can't be added without restructuring the existing flag
  loop — stop and report the conflict instead of refactoring `main()`.
- `logger::init` can't target stderr without touching more than `logger.rs`.
