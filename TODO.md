# TODO — 0.1.0

## Core model

- [ ] T-001 kuda-core — RepoCard / ModuleCard / Symbol / DepEdge / Provenance{files,hash,sha}; Lang descriptors loaded from data/langs/\*.toml — a new language is a data change.

## Ports + first adapters

- [ ] T-002 kuda-extract port + kuda-manifest adapter (cargo/npm/go/py manifests -> package graph; no parser needed — lands first)
- [ ] T-003 kuda-treesitter adapter — grammar+query data per language (rust first)
- [ ] T-004 kuda-store port + kuda-store-sqlite adapter (L2 graph)
- [ ] T-005 kuda-vcs port + kuda-git adapter (sha, changed files -> staleness) — bupha's vcs/git crates already fit; repurpose
- [ ] T-006 kuda-summarize port + kuda-template adapter (deterministic cards: API list, deps, commands — zero-model default)

## Pipeline + queries

- [ ] T-007 kuda-query — find / dependents / card / refs (--docs)
- [ ] T-008 kuda-cards — .kuda/index.yaml + modules/\*.yaml render; machine owns api/deps fields, humans own notes: region; every card ends with verify: file list

## Init ergonomics

- [ ] T-009 kuda init / sync should append .kuda/index.sqlite + .kuda/meta.json to .gitignore automatically — cards are committed artifacts, the graph is rebuildable. Found via dogfood: mimus+bupha committed the sqlite binary by hand.

## Semantic depth

- [ ] T-010 Embedding-based claim clustering for doc↔doc contradiction detection (today: identifier-token clustering — finds the obvious pairs, misses paraphrases). Needs a local embedder or embedding API; evaluate cost vs precision first.

## Languages

- [ ] T-011 .hnl — add a data/langs/hnl.toml descriptor (extensions, manifests if any, symbol/dep queries). Blocked on a tree-sitter-handl grammar: check for an existing one; else write/publish it — grammar is the only real work, the descriptor is data. Pays off immediately: handl-lang fixtures/packages stop being invisible to the index.

## Scaffold debt (forked from bupha — rewrite crate-by-crate)

- [ ] T-012 modules/dagger — keep as kuda's own CI module (dogfoods bupha).
