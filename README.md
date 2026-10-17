# kuda

*A hippocampus for your codebase.*

Repo memory for agent-driven workflows. A single binary that crawls
your repos and builds a compressed, freshness-tracked index — repo
cards, module cards, a symbol/dependency graph — that agents query
instead of re-exploring from scratch every session.

```
repos ──► extract ──► summarize ──► index ──► store ──► query
           │            │            │         │         │
     tree-sitter/LSP  card gen   graph +   .kuda/    CLI + MCP
     manifests       per module   cards    artifacts   server
```

## Features

- tiered memory: **L0** repo card (~1k tokens, always-loadable),
  **L1** module cards (on-demand), **L2** symbol/dep index —
  memory is an index into code, never a copy of it
- provenance on every card — `{files, content hashes, commit sha}`;
  `kuda status` diffs against HEAD and reports per-card drift
- every concern behind a port: extractors (`manifest` |
  `tree-sitter` | `lsp`), summarizers (`template` | `llm`),
  store (`sqlite`), vcs (`git`), serve (`cli` | `mcp`) —
  ports/adapters are separate crates
- language knowledge is *data* (grammar + query + manifest files),
  not engine code — a new language is a data change, not a release
- summarizer defaults to `template` (deterministic, zero-model);
  `llm` is an adapter detail behind the port
- multi-repo workspaces — sibling checkouts and git-dep edges
  resolve across repo boundaries
- agents consume via MCP; humans and scripts via CLI

## vs. grep + re-read every session

- planning: `kuda dependents EvalScheme` → every caller, across repos
- doc sweeps: `kuda refs OldFlow --docs` → code *and* markdown/
  comment mentions, enumerable and verifiable
- staleness is explicit: cards say which files they describe and
  which sha they were built at — trust is checked, not assumed
- cards end in `verify:` file lists — memory is a hypothesis,
  not ground truth

## Install

Platform: **Linux + git**. Indexing needs nothing else for the
`manifest`/`treesitter` path; `lsp` adapter needs the language server.

```sh
cargo install kuda                                  # crates.io (post-0.1.0)
cargo install --git https://github.com/9aia/kuda kuda   # today
```

## Quickstart

```sh
cd your-repo
kuda init              # detect languages -> kuda.toml + .kuda/
kuda sync              # incremental index (hash -> extract -> cards)
kuda status            # provenance vs HEAD: "3/40 cards stale"
kuda map               # render the L0 repo card (always-load blob)

kuda card plugin-engine-exec     # module card + freshness
kuda find EvalScheme             # symbol -> file:line
kuda dependents apply_desugar    # reverse dep lookup
kuda refs OldFlow --docs         # code + docs/comments mentions
kuda mcp                         # serve the index over MCP
kuda doctor                      # extractor/store health
```

## Docs

Full docs: **[9aia.com/kuda](https://9aia.com/kuda/)** — tutorial,
how-tos, cli/config/crate-API reference, architecture (Diátaxis).

[CONTRIBUTING](CONTRIBUTING.md) · [SECURITY](SECURITY.md) ·
[LICENSE](LICENSE) (MIT) · [project](project/index.md) — TODO, backlog, ADRs
