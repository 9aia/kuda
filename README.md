# kuda

*A hippocampus for your codebase.*

Repo memory for agent-driven workflows. A single binary that crawls
your repos and builds a compressed, freshness-tracked index — repo
cards, module cards, a symbol/dependency graph — that agents query
instead of re-exploring from scratch every session.

```
repos ──► extract ──► summarize ──► index ──► store ──► serve
           │            │            │         │         │
   manifest/treesitter  template    graph +   .kuda/    cli |
   /lsp                | acp        cards     artifacts  mcp | acp
```

## Features

- **Tiered memory**: L0 repo card (~1k tokens, always-loadable),
  L1 module cards (on-demand), L2 symbol/dep index — memory is an
  index *into* code, never a copy of it
- **Cards are committed artifacts** — `.kuda/index.yaml` +
  `modules/*.yaml` are checked in and meant to be diff-reviewed.
  They stay bounded by design (`api` capped at 40 public symbols,
  L0 provenance = manifests only + a `tree_digest`) so even
  monorepo diffs stay reviewable; `.kuda/index.sqlite` is
  gitignored and rebuildable
- **Provenance on every card** — `{files, hashes, commit_sha}`;
  `kuda status` re-hashes and reports per-card drift; `mimus audit`
  verifies independently
- **Extraction tiers**: `manifest` (free) → `treesitter`
  (data-driven symbol queries) → `lsp` (rust-analyzer
  `documentSymbol`). Best available wins; absent server = silent skip
- **Semantic summarization via ACP** — `kind = "acp"` spawns your
  agent (`zed agent`, `claude`, `gemini`…) to write grounded card
  prose; no API keys, template fallback always
- **Multi-repo workspaces** — `kuda sync --workspace` indexes
  sibling repos; `dependents`/`find`/`refs` merge across roots
- **Agent surfaces**: MCP tool server *and* ACP session surface —
  same index, two wire protocols
- **Fuzzy find** — exact miss falls back to substring hits

## Install

```sh
cargo install --git https://github.com/9aia/kuda kuda
```

## Quickstart

```sh
cd your-repo
kuda init          # kuda.toml + .kuda/ skeleton
kuda sync          # extract -> cards -> index
kuda status        # provenance drift vs HEAD
kuda map           # the L0 card
kuda card <mod>    # an L1 module card
kuda find <sym>    # exact; miss -> fuzzy hits
kuda dependents X  # reverse dep edges (impact)
kuda refs X --docs # code + docs/comments mentions
kuda mcp           # MCP stdio tool server
kuda acp           # ACP session surface
```

## Config

```toml
[extract]
kinds = ["manifest", "treesitter"]   # add "lsp" with rust-analyzer
# lsp_server = "rust-analyzer"

[summarize]
kind = "template"                    # deterministic (default)
# kind = "acp"                       # spawn an agent for card prose
# acp_cmd = "zed agent"

[workspace]
roots = ["../cli", "../lsp"]
```

## The pair

**kuda** remembers; **[mimus](https://github.com/9aia/mimus)**
doubts — claim extraction + checkers + rules + `fix` + `audit`
of these cards. Memory with a falsifier.

## Docs

https://9aia.com/kuda — tutorials, how-tos, reference, explanation.
