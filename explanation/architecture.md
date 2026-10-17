---
url: /kuda/explanation/architecture.md
---
# Explanation: architecture

kuda answers one question — *"what does this project look like,
right now, in a form an agent can load?"* — and treats a wrong
answer as worse than none.

## The pipeline

```
repos ──► extract ──► summarize ──► index ──► store ──► serve
          │             │            │         │         │
   adapters:        adapters:    graph +   sqlite    cli |
   manifest         template     cards    + yaml    mcp | acp
   treesitter       (llm planned)  provenance
   lsp
```

**Extract** is mechanical — no model, fully rerunnable. It pulls
symbols, package structure, and import edges.

**Summarize** turns each module's extraction into an L1 card —
committed YAML humans review in diffs.

**Index** merges everything into the symbol/dependency graph with
per-node provenance.

**Serve** is read-side only: `find`, `dependents`, `refs`,
`card`, `map`, `status` — plus the MCP and ACP wire surfaces.

## The tiers of memory

* **L0 repo card** (`.kuda/index.yaml`) — the ~1k-token
  always-load blob: purpose, module map, commands, provenance.
* **L1 module cards** (`.kuda/modules/*.yaml`) — fetched on
  demand when planning touches a module.
* **L2 graph** (`.kuda/index.sqlite`) — symbols, edges both
  directions; gitignored, rebuildable, the query substrate.
* **L3** — the source itself. Memory is an index *into* code,
  never a replacement.

Committed L0/L1 + ignored L2 is deliberate: cards are reviewed
prose, the graph is derived state.

## Extraction tiers

Best-available wins, configured per repo via `[extract].kinds`:

* `manifest` — free, no parser: package graph + module
  boundaries from Cargo.toml/package.json/pyproject/go.mod.
* `treesitter` — grammar+query data from `data/langs/*`:
  symbols and `use`/import edges in ~any language.
* `lsp` — `documentSymbol` against a real server
  (`lsp_server = "rust-analyzer"`); the strongest signal where a
  server exists. Absent server = silent skip.

See [adding a language](../how-to/languages.md) — knowledge lives
in descriptor files; adding one is a data change.

## Language as data

Per-language knowledge — grammar refs, symbol queries, manifest
names — lives in `data/langs/*`, not in extractor code. The
adapters' vocabulary (`language()` registries, `SymbolKind`,
`EdgeKind`) is the only fixed part.

## Ports and adapters

`kuda-core` holds domain types only (`Symbol`, `ModuleCard`,
`Provenance`, `Claim`-free config). Every concern is a trait
crate + adapter crates:

* `kuda-extract` → `kuda-manifest`, `kuda-treesitter`, `kuda-lsp`
* `kuda-store` → `kuda-store-sqlite`
* `kuda-summarize` → `kuda-template`, (`kuda-llm` planned)
* `kuda-vcs` → `kuda-git`; `kuda-config` → `kuda-config-toml`;
  `kuda-secret` → `kuda-keyring`, `kuda-dotenv`

New capability = new adapter crate, not a core rewrite.

## Trust model

Every artifact carries `provenance {files, hashes, commit_sha,
generator}` — `kuda status` re-hashes and reports drift per card.
Cards end with `verify:` — the files an agent should open before
acting on the card. Memory is a hypothesis that narrows the
search space; the code remains the only truth. `mimus audit`
independently re-verifies card provenance — memory with an
external falsifier.
