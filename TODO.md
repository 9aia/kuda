# TODO — 0.1.0

## Core model

- [ ] T-026 presentation deck — verify + integrate: slides v2 (claim/proof, 3 chapters) pushed on presentation/initial (e1844e5); deck session reported it still fails verification — run a repair/verify pass, then merge per sturn flow (no PRs — pushed branch is the deliverable) · created:2026-11-02
- [ ] T-035 kuda watch — domain roots + WAL-ref coverage: poll-refs flag, root(cli) fix, dir\_excluded filter (sync-lag reduction) · pri:high · owner:devin · created:2026-11-10

## Ports + first adapters

## Pipeline + queries

## Init ergonomics

## Semantic depth

## Languages

- [ ] T-011 .hnl — add a data/langs/hnl.toml descriptor (extensions, manifests if any, symbol/dep queries). Blocked on a tree-sitter-handl grammar: check for an existing one; else write/publish it — grammar is the only real work, the descriptor is data. Pays off immediately: handl-lang fixtures/packages stop being invisible to the index. · status:blocked

## Scaffold debt (forked from bupha — rewrite crate-by-crate)

- [ ] T-012 modules/dagger — keep as kuda's own CI module (dogfoods bupha).

## Recovered
