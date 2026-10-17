---
url: /kuda/tutorials/first-run.md
---
# Tutorial: your first index

Goal: go from a checkout to a queryable memory in minutes.

## 1. Init

```sh
cd your-repo
kuda init
```

Detects languages from file extensions + manifests, writes a
commented `kuda.toml` and a `.kuda/` skeleton. Review the toml —
defaults (`manifest` + `treesitter` extract, `template` summarize,
`sqlite` store) are fully local and deterministic.

## 2. Sync

```sh
kuda sync
```

Hashes files, runs extractors, writes:

```
.kuda/index.yaml      L0 repo card  — commit this
.kuda/modules/*.yaml  L1 cards      — commit these
.kuda/index.sqlite    L2 graph      — gitignored, rebuildable
.kuda/meta.json       provenance    — sha + hashes
```

## 3. Query

```sh
kuda map                    # the card an agent would load
kuda card <module>          # purpose, api, deps, freshness
kuda find <symbol>          # symbol -> file:line
kuda dependents <symbol>    # who calls it — impact analysis
kuda refs <name> --docs     # code + docs/comments mentions
```

## 4. Stay fresh

```sh
kuda status   # "3/40 cards stale" — provenance sha vs HEAD
kuda sync     # regenerates only the drifted cards
```

Commit `index.yaml` + `modules/` — the prose is reviewable in diffs.
