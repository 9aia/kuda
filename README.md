# kuda

*Code-map engine — verified per-module cards agents read before code.*

Repo memory for agent-driven workflows. `kuda sync` crawls your
repos into a compressed, freshness-tracked index — repo cards,
module cards, a symbol/dependency graph — that agents query instead
of re-exploring from scratch every session. Cards are committed
artifacts (`.kuda/*.yaml`) with provenance; `kuda status` reports
drift vs HEAD.

```
repos ──► extract ──► summarize ──► index ──► store ──► serve
           │            │            │         │         │
   manifest/treesitter  template    graph +   .kuda/    cli |
   /lsp                | acp        cards     artifacts  mcp | acp
```

## Quickstart

```sh
# the `kuda` command is a tamias pack (libraries/kuda)
tamias install 9aia:kuda@<version>    # org registry
# or from a checkout: scripts/pack.sh && tamias install --allow-drivers ./libraries/kuda

cd your-repo
kuda init          # kuda.toml + .kuda/ skeleton
kuda sync          # extract -> cards -> index
kuda status        # provenance drift vs HEAD
kuda map           # the L0 repo card
kuda card <mod>    # an L1 module card
kuda find <sym>    # exact; miss -> fuzzy hits
kuda dependents X  # reverse dep edges (impact)
kuda refs X --docs # code + docs/comments mentions
kuda mcp           # MCP stdio tool server
kuda acp           # ACP session surface
```

## Why

Every agent session re-explores the same code. kuda makes memory an
index *into* code, never a copy: tiered cards (L0 repo card always
loadable, L1 module cards on demand, L2 symbol/dep index),
freshness-tracked provenance, and pluggable extraction
(manifest → treesitter → LSP → records corpora) and summarization
(template default, or your own ACP agent for grounded prose).

**kuda remembers; [mimus](https://github.com/9aia/mimus) doubts** —
claim extraction + checkers + rules + `fix` + `audit` of these
cards. Memory with a falsifier.

## Docs

- [Docs site](https://9aia.com/kuda) — tutorials, how-tos,
  reference, explanation (VitePress + Diátaxis, `docs/` worktree)
- [AGENTS.md](AGENTS.md) — crate map, commands, rules, boundary
- [CONTRIBUTING.md](CONTRIBUTING.md) · [SECURITY.md](SECURITY.md)
- [LICENSE](LICENSE) (MIT) · [project ledger](project/index.md)
