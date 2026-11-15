# Concepts

kuda's vocabulary is small but load-bearing — most of it is about
*what a card is allowed to claim*.

## Entity

- **Repo** — the unit of indexing. One `.kuda/` per checkout.
- **Workspace** — a set of sibling repo roots declared in
  `.kuda/workspace.yaml` or `[workspace]` in `kuda.toml`; enables
  cross-repo queries like `dependents`.
- **Module** — the unit a card describes. Grouped by *manifest
  package name*, not directory name, so `kuda-docs/` (the `docs`
  worktree) indexes as `kuda-docs`, not `docs`.
- **File** — the unit of freshness. Every source file gets a git
  blob hash; a card's `verify:` list names the files an agent must
  open before trusting it.

## Memory tiers

- **L0 — repo card** (`.kuda/index.yaml`): the always-load blob.
  Purpose, module map, commands, provenance. ~1k tokens.
- **L1 — module cards** (`.kuda/modules/*.yaml`): fetched on
  demand when planning touches a module.
- **L1 file cards** (`.kuda/files/<module>/*.yaml`): per-source-file
  purpose, opt-in via `[summarize] file_cards = true`.
- **L2 — graph** (`.kuda/index.sqlite`): symbols, dependency edges
  both directions, per-file hashes. Gitignored, rebuildable —
  derived state, never reviewed prose.
- **L3 — the source**: the only truth. Memory is an index *into*
  code, never a replacement.

## Provenance

Every card carries `{files, hashes, commit_sha, generator}`.
`kuda status` re-hashes and reports drift per card — staleness is
a diff, not a guess. Cards are committed so humans review them in
git diffs; the L2 graph is ignored because it regenerates.

## Ports

- **extract** — mechanical symbol/edge pull. Adapters:
  `manifest` (package graph, free), `treesitter` (grammar+query
  data), `lsp` (a real language server, strongest where present).
- **summarize** — extraction → card prose. `template` is
  deterministic; `llm`/`acp` tiers are opt-in.
- **store** — where cards and the graph live (`sqlite` today).
- **vcs** — commit-sha and blob-hash provenance (`git` today).

## Surfaces

`kuda` CLI for humans and scripts; `kuda mcp` for MCP-speaking
agents; `kuda acp` to drive an external agent as the summarizer.
All read the same `.kuda/` artifacts.
