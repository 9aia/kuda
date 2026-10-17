# Explanation: architecture

## Tiered memory, not one big summary

A monolithic repo summary fails twice: too coarse to plan with, or
too big for context. kuda produces a hierarchy agents zoom into:

- **L0 — repo card** (`index.yaml`, ~1k tokens): purpose, module
  map, entry points, build/test commands. Always-loadable.
- **L1 — module cards** (`modules/*.yaml`): API surface, deps,
  invariants, `verify:` file list. Fetched when planning.
- **L2 — symbol/dep graph** (`index.sqlite`): symbol -> file:line
  and dep edges both directions. Queried, not loaded.
- **L3 — source**: the code. Memory indexes it; never replaces it.

## Provenance is the staleness model

Every card records `{files, content_hash, commit_sha}`.
`kuda status` diffs the recorded sha against HEAD — drift is a
property of the data, not a promise in prose. `sync` regenerates
only cards whose `files` changed.

## Ports and impls

Every concern is a trait crate + impl crates (ADR-0001):

- extract: `manifest` (free, no parser) -> `treesitter`
  (grammar+query data) -> `lsp` (cross-file refs where a server
  exists)
- summarize: `template` (deterministic default) | `llm` (opt-in
  prose — provider key stays in env/keyring)
- store: `sqlite` (rebuildable, gitignored)
- serve: `cli` | `mcp`

## Language as data

Per-language knowledge — grammar refs, symbol queries, manifest
kinds — lives in `data/langs/*`, not in extractor code. A new
language is a data change; the engine's vocabulary is fixed.

## Trust model

Cards end with `verify:` — the files an agent should open before
acting on the card. Memory is a hypothesis that narrows the search
space; the code remains the only truth.
