---
theme: 9aia
title: kuda — A hippocampus for your codebase
info: |
  Repo memory for agent-driven workflows: a compressed, freshness-tracked
  index into code, for engineers evaluating repo-memory tooling.
transition: fade
mdc: true
layout: cover
---

# kuda

*A hippocampus for your codebase.*

Repo memory for agent-driven workflows — bounded, provenanced, reviewable.

9aia · 2026

---
layout: default
---

# Contents

1. **A 20-crate codebase compresses into 81 lines** — measured here
2. **Staleness is data, not a promise** — provenance, per card
3. **The index is a cache; the cards are the artifact** — what git keeps
4. **Every card is reviewable like source** — regen is a diff
5. **One binary speaks cli, mcp and acp** — the verbs

---
layout: statement
---

# A 20-crate codebase compresses into 81 lines.

---
layout: fact
---

# 81

lines of `.kuda/index.yaml` — the always-load card for
20 crates and 8 194 lines of Rust

---
layout: statement
---

# Staleness is data, not a promise.

---
layout: default
---

# `status` re-hashes the files each card names

```yaml
# .kuda/modules/kuda-core.yaml
provenance:
  files:
  - crates/kuda-core/Cargo.toml
  - crates/kuda-core/src/lib.rs
  hashes:
  - 858a2d26…   # content hash
  - e218eed6…
  commit_sha: e360ec66…
```

- `kuda status` re-hashes each card's `files` — drift is computed
- the argument in code: `crates/kuda-core/src/lib.rs:6-7`

---
layout: statement
---

# The index is a cache; the cards are the artifact.

---
layout: default
---

# 48 files git carries; the cache is ignored

```sh
$ git ls-files .kuda | wc -l
48
$ git check-ignore .kuda/index.sqlite
.kuda/index.sqlite
```

---
layout: statement
---

# Every card is reviewable like source.

---
layout: default
---

# Regeneration shows up as a diff

| artifact | count | in git | verification |
| --- | --- | --- | --- |
| repo card (L0) | 1 | committed | `kuda status` re-hash |
| module cards (L1) | 20 | committed | `kuda status`, `mimus audit` |
| file cards (L1.5) | 27 | committed | `kuda status`, `mimus audit` |
| symbol index (L2) | sqlite | gitignored | rebuilt by `kuda sync` |

- `mimus audit` trust-scores each card — fresh, drifted, or contradicted

---
layout: statement
---

# One binary speaks cli, mcp and acp.

---
layout: default
---

# Every verb, one binary

```sh
kuda sync            # hash → extract → summarize → store
kuda status          # provenance drift vs HEAD
kuda map             # L0 repo card — always-load
kuda card <mod>      # L1 module card
kuda find <sym>      # exact; miss → fuzzy
kuda dependents <x>  # reverse dep edges
kuda refs <x>        # code + doc mentions
kuda watch           # fs events → incremental sync
kuda mcp             # tools: repo_card module_card find dependents status refs
kuda acp             # ACP session surface
```

---
layout: default
---

# Where this sits

| Layer | Repo | Role |
| --- | --- | --- |
| orchestration | peregrin | policy, campaigns, nodes |
| sessions | sepia | one session IR, every agent |
| execution | bupha | headless CI, latest-wins |
| coordination | sturn | claims, WAL, topics |
| verification | mimus | claims vs code |
| intent | myco | tasks, handoffs |
| memory | **kuda** | repo index, cards |
| time | cicada | calendars as code |

Above: `myco` (intent) · below: `cicada` (time) · `mimus` audits these cards.

---
layout: default
---

# Status — v0.1.0

- works: the full CLI surface, plus `--workspace` multi-repo merges
- agent surfaces: `kuda mcp` (six tools) and `kuda acp`
- languages: rust, typescript, python, go — more = a `data/langs/` file
- parked: `dist` releases (`cargo install --git` suffices); `kuda-llm` summarizer
- agent-maintained — issues and PRs disabled; CI dogfooded

---
layout: end
---

# kuda

github.com/9aia/kuda · 9aia.com/kuda

Index your repo, read the cards, diff the next sync.
