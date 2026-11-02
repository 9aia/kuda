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

1. **The problem** — every session starts from zero
2. **How it works** — crawl, compress, commit, serve
3. **The mechanism** — tiers, provenance, ports
4. **In practice** — the CLI, dogfooded on itself
5. **Where it sits** — the 9aia stack
6. **Status** — what ships today

---
layout: default
---

# Every session starts from zero.

- file walks and grep guesses burn the first tokens of every task
- context windows fill with orientation, not the work itself
- the notes it writes go stale — nothing tracks the drift

---
layout: section
---

# How it works

---
layout: default
---

# Crawl, compress, commit, serve

```text
repos ──► extract ──► summarize ──► index ──► store ──► serve
           │            │            │         │         │
   manifest/treesitter  template    graph +   .kuda/    cli |
   /lsp                | acp        cards     artifacts  mcp | acp
```

- `kuda sync` is incremental — content hashes pick what re-summarizes
- `kuda watch` re-syncs on fs events; only changed modules re-summarize
- `--workspace` merges `find`/`dependents`/`refs` across repo roots

---
layout: default
---

# An index into code — never a copy of it

| tier | artifact | load |
| --- | --- | --- |
| L0 | `.kuda/index.yaml` | always — repo card, ~1k tokens |
| L1 | `.kuda/modules/*.yaml` | on demand — one card per module |
| L1.5 | `.kuda/files/**/*.yaml` | on demand — per-file summaries |
| L2 | `.kuda/index.sqlite` | queried — symbol + dep graph |

- agents read L0, then pull only the card or symbol needed
- bounded by design: `api` caps at 40 public symbols per card

---
layout: default
---

# Committed cards, computed staleness

```yaml
# .kuda/modules/kuda-core.yaml — excerpt
name: kuda-core
purpose: 'kuda domain core: cards, symbols, edges, provenance'
api_omitted: 2            # the remainder beyond the 40-symbol cap
verify:                   # the files this card claims to describe
- crates/kuda-core/Cargo.toml
- crates/kuda-core/src/lib.rs
provenance:
  hashes: [858a2d26…, e218eed6…]
  commit_sha: e360ec6…
```

- cards are checked in — every regeneration is a reviewable diff
- `index.sqlite` is gitignored — a rebuildable cache, never source of truth
- `kuda status` re-hashes each card's files → per-card drift report
- `mimus audit` verifies the same cards independently

---
layout: default
---

# Everything interesting sits behind a port

| port | adapters | chosen by |
| --- | --- | --- |
| extract | manifest · treesitter · lsp | `extract.kinds` |
| summarize | template · acp | `summarize.kind` |
| store | sqlite | `store.kind` |
| secrets | keyring · dotenv | env → keyring → file |

- best available wins — an absent LSP server is a silent skip
- new language = a `data/langs/*.toml` file, not an engine change
- `kind = "acp"` spawns your agent for card prose — no API keys

---
layout: fact
---

# 81

lines of YAML in kuda's own L0 card `.kuda/index.yaml` —
the always-load summary of a 20-crate, ~8.2k-line Rust workspace

---
layout: default
---

# One binary, three wire protocols

```sh
kuda init                # kuda.toml + .kuda/ scaffold
kuda sync [--full]       # hash → extract → summarize → store
kuda status              # provenance drift vs HEAD
kuda map                 # L0 repo card — the always-load blob
kuda card <mod>          # L1 module card + freshness
kuda find <sym>          # exact; miss → fuzzy hits
kuda dependents <name>   # reverse dep edges (impact)
kuda refs <name> --docs  # code + docs/comments mentions
kuda watch               # fs events → incremental sync
kuda mcp | kuda acp      # MCP tools | ACP session surface
kuda doctor              # config + adapter health
```

MCP tools: `repo_card` `module_card` `find` `dependents` `status` `refs`

---
layout: default
---

# Dogfooded — this repo indexes itself

```sh
$ kuda status
drifted  kuda-core
tree drifted (tree_digest mismatch — 50 files walked, 49 at sync)
19 fresh, 1 drifted (HEAD e360ec6)

$ kuda find Provenance
crates/kuda-core/src/lib.rs:20 Type kuda-core

$ kuda dependents kuda-core
kuda-acp --Imports--> kuda-core
kuda --PackageDep--> kuda-core
```

Committed here: `.kuda/` — L0 card + 20 module cards + 27 file cards

---
layout: quote
---

"Every card carries `Provenance` — staleness is a property of the data, not a promise in prose."

`crates/kuda-core/src/lib.rs`

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

Above: `myco` · below: `cicada` · `mimus` audits these cards.

---
layout: default
---

# Status — v0.1.0, honestly

- works today: the full CLI plus `watch`, `mcp`, `acp`, `--workspace`
- incremental sync by content hash; stale cards swept; `.kuda/` dogfooded
- languages: rust, typescript, python, go — more = a `data/langs/` file
- parked: `kuda-llm` summarizer (`acp` covers prose); dist releases deferred
- agent-maintained — issues and PRs disabled, CI dogfooded

---
layout: end
---

# kuda

github.com/9aia/kuda · 9aia.com/kuda

`cargo install --git https://github.com/9aia/kuda kuda` — index a repo, diff the cards.
