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
* **L1.5 file cards** (`.kuda/files/<module>/*.yaml`) — per-source-file
  `purpose` written by the semantic tier (opt-in,
  `[summarize] file_cards = true`). Files are chunked with a
  code-aware splitter (tree-sitter boundaries via text-splitter),
  fed to the agent, and hash-gated per file.
* **L2 graph** (`.kuda/index.sqlite`) — symbols, edges both
  directions, per-file hashes; gitignored, rebuildable, the query
  substrate.
* **L3** — the source itself. Memory is an index *into* code,
  never a replacement.

Committed L0/L1/L1.5 + ignored L2 is deliberate: cards are reviewed
prose, the graph is derived state.

## Incremental sync

`sync` is diff-driven end to end:

1. One walk groups manifests + source files by module (manifest
   package names, not dir names — `kuda-docs`, not `docs`).
2. Every file gets an in-process git blob hash — no subprocess
   per file.
3. Hashes are compared against the store's `files` table —
   unchanged modules are never re-parsed; a changed file re-parses
   only its own module.
4. Cards whose provenance matches are reused verbatim — the
   summarizer (and the agent, in `acp` mode) is only invoked for
   drift.

Steady-state on a warm index is sub-second; agent spend is
per-drift, never per-sync. Deleted files and vanished modules are
detected by the same diff — their cards and store rows are
removed.

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
