# TODO — 0.1.0

## Core model

- [ ] T-026 presentation deck — verify + integrate: slides v2 (claim/proof, 3 chapters) pushed on presentation/initial (e1844e5); deck session reported it still fails verification — run a repair/verify pass, then merge per sturn flow (no PRs — pushed branch is the deliverable) · created:2026-11-02
- [ ] T-028 agent-awareness managed block — kuda init should write \<!-- kuda:agent-awareness --> into AGENTS.md (pattern: myco init) + validate block freshness in kuda status/check. Block text already deployed to the 6 .kuda-wired repos (cards-are-artifacts, kuda status drift, sync-on-change) · created:2026-11-02

## Ports + first adapters

- [ ] T-004 kuda-store port + kuda-store-sqlite adapter (L2 graph)
- [ ] T-005 kuda-vcs port + kuda-git adapter (sha, changed files -> staleness) — bupha's vcs/git crates already fit; repurpose
- [ ] T-006 kuda-summarize port + kuda-template adapter (deterministic cards: API list, deps, commands — zero-model default)
- [ ] T-003 kuda-treesitter adapter — grammar+query data per language (rust first) · status:in-progress

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

## Recovered

- [ ] T-025 verify claim .kuda/modules/Cargo.toml.yaml:3 ('Cargo.toml') — file missing, card likely stale; recovered from dead sessions 1792289007778\_292qm, 1792289232239\_Jpne2, 1792289295855\_Xr\_YW, 1792289355290\_bAAOh, 1792289567602\_H2XTF · created:2026-10-30
