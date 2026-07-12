# 0005 — Grammar: r-lib/tree-sitter-r; queries: adapt from ocsmit/zed-r (MIT)

- **Status:** Accepted (Dagmar, 2026-07-12)
- **Date:** 2026-07-11
- **Decided by:** Dagmar
- **Related:** [../03-what-zed-needs.md](../03-what-zed-needs.md), task T05

## Context

Zed needs a tree-sitter grammar pinned by commit, plus `.scm` query files mapping parse
nodes to editor concepts. The canonical R grammar is
<https://github.com/r-lib/tree-sitter-r> (maintained by the R core tooling community;
also what ark's own tooling ecosystem aligns with). The existing
<https://github.com/ocsmit/zed-r> extension (MIT) already ships working Zed queries for
R against that grammar (pin `a0d3e3307489c3ca54da8c7b5b4e0c5f5fd6953a` as of v0.2.6).

## Decision

Pin `r-lib/tree-sitter-r` (bump to zed-r's current pin at implementation time) and
start from zed-r's query files, copied with MIT attribution (their license header +
a NOTICE line in `editors/zed/README.md`), then adjust as needed.

Alternative: write queries from scratch — educational but slow, and highlighting
quality is not this project's point.

## Consequences

- Day-one highlighting quality equals the existing R extension.
- We owe attribution and should upstream fixes to zed-r where sensible.
- Grammar version drift between zed-r queries and our pin is possible; T05 verifies by
  opening real R files.
