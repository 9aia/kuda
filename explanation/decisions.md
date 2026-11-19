---
url: /kuda/explanation/decisions.md
---
# Decisions

ADR-lite — the decisions that shape kuda, the alternatives
rejected, and the consequence each carries. Full history lives in
git and the org ledger (`project/`).

## Cards are committed; the graph is not

**Decision:** L0/L1 YAML cards are committed artifacts reviewed in
diffs; the L2 sqlite graph is gitignored and rebuildable.

**Alternatives:** commit everything (the graph churns every sync —
noise that drowns the prose); commit nothing (cards become
derived state nobody reviews, and provenance stops meaning
anything).

**Consequence:** cards are *claims with evidence* — humans and
`mimus audit` can falsify them. The graph is free to be a cache.

## Modules group by manifest package, not directory

**Decision:** extraction groups source files by the package name
declared in `Cargo.toml`/`package.json`/`pyproject.toml`, falling
back to directories only when no manifest exists.

**Alternatives:** directory grouping (splits one crate across
`src/`, `tests/`, `benches/` into fake "modules" and merges
unrelated dirs that happen to share a parent); repo-wide flat
indexing (no module boundary = no place to hang an L1 card).

**Consequence:** `docs/` (a git worktree) indexes as its own
package `kuda-docs`; workspace members stay distinct even when
they share a directory tree.

## Language knowledge is data, not code

**Decision:** per-language grammars, symbol queries, and manifest
names live in `data/langs/*`; extractor code only knows the
adapter vocabulary (`SymbolKind`, `EdgeKind`).

**Alternatives:** per-language extractor crates (a release per
language, adapter sprawl); tree-sitter-only (no package-graph
signal — the free `manifest` tier disappears).

**Consequence:** adding a language is a data change reviewable in
a diff — see [adding a language](../how-to/languages.md). The cost
is a small descriptor DSL the extractors must interpret.

## Sync is hash-gated end to end

**Decision:** every file gets an in-process git blob hash compared
against the store; unchanged modules are never re-parsed and cards
with matching provenance are reused verbatim — including skipping
the summarizer/agent entirely.

**Alternatives:** mtime-based staleness (unreliable across
checkouts and rebases); always-resummarize (agent spend per sync
instead of per drift).

**Consequence:** steady-state sync is sub-second and agent spend
scales with *change*, not repo size. The `files` table in sqlite
is the source of truth for "did this file change."

## Releases are tag-only; the command is a tamias pack

**Decision:** releases run `tarsi-serve release --dir .` directly
— bump, tag, push, no publish targets (`tarsi.toml` disables
crates.io and GitHub Releases); the per-repo `scripts/release.sh`
shim was removed org-wide. The `kuda` command
ships as the `libraries/kuda` tamias pack; `kuda-cli` is lib-only.

**Alternatives:** crates.io + cargo-dist binaries; a `[[bin]]`
in `kuda-cli`; release-plz/GHA (hosted CI for a project whose
point is that CI is a local loop).

**Consequence:** one version across the workspace, one tag scheme,
and install/update lifecycle owned by tamias — but there is no
`cargo install kuda`, and pack publishing stays a manual
credentials-gated step. See [the release model](release-model.md).
