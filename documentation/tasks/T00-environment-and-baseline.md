# T00 — Environment & baseline (Dagmar + Claude, not delegated)

- **Status:** Done (2026-07-12; Zed REPL check skipped)
- **Goal:** prove the toolchain, the build, the tests, and "ark already runs R in Zed"
  before we change anything. Also: first guided tour of the codebase.

## Steps

1. **Toolchain check**
   ```sh
   rustc --version          # pinned by rust-toolchain.toml; rustup obeys it automatically
   cargo --version
   cargo nextest --version  # if missing: cargo install cargo-nextest
   just --version           # if missing: install `just` (task runner; recipes in ./justfile)
   R --version              # ark needs R >= the minimum in crates/ark/src/version.rs
   ```
2. **R test packages** (required by the test suite, see CLAUDE.md):
   in R: `install.packages(c("data.table", "dplyr", "rstudioapi", "tibble", "haven", "htmltools", "R6", "readr"))`
3. **Build** (first build compiles ~20 crates; expect minutes):
   ```sh
   cargo build
   ls -l target/debug/ark
   ```
4. **Test a slice** (full suite is long; start with LSP integration tests):
   ```sh
   just test -p ark lsp
   ```
5. **Fun win — R in Zed today, via the kernel:**
   ```sh
   ./target/debug/ark --install     # writes the Jupyter kernelspec
   ```
   In Zed: open any `.R` file, type a `# %%` cell, run with ctrl-shift-enter
   (see https://zed.dev/docs/repl). This is the kernel half working; the LSP half is
   this project.
6. **Reading**: documentation/01 → 02 → 03 → 04, questions welcome; then Dagmar
   accepts/rejects decisions 0002–0005.

## Done when

- [x] `cargo build` and the LSP test slice pass locally
- [ ] Zed REPL executed R code through ark — *skipped by Dagmar (kernelspec install declined); kernel boot is covered by the integration tests*
- [x] Decisions 0002–0005 each say Accepted or have review comments

## Outcome (2026-07-12)

- Toolchain: rustc 1.96.1, cargo 1.96.1, nextest 0.9.114, just 1.43.1 — all present.
  All eight R test packages already installed.
- `just test -p ark lsp` initially failed (28 of 363): R's `libR.so` (from
  nixpkgs-unstable, built against glibc 2.41) could not be dlopen'd into ark
  binaries linked against the system's glibc 2.40 — one process cannot mix two
  glibc generations. Fixed in `~/nixos-config`: system flake upgraded NixOS
  25.11 → 26.05 (glibc 2.42) and R moved from `unstable.rWrapper` to the base
  nixpkgs (R 4.6.0 → 4.5.3) so ark and R always share one glibc. After
  `cargo clean && cargo build`, 363/363 pass.
