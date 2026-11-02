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

**I. The claims** — four things kuda says it does

**II. The evidence** — the receipt for each claim, in order

**III. Try it** — four commands, five minutes

*Then: where it sits in the 9aia stack, status, references.*

---
layout: section
---

# I. The claims

Four statements, each provable inside this repo.

---
layout: default
---

<Badge type="info">Claim 1 of 4</Badge>

# A 20-crate codebase compresses into 81 lines.

- the L0 card is what an agent loads first, every session
- purposes, module names and entry points — never source
- bounded by design, so it stays loadable as the repo grows

---
layout: default
---

<Badge type="info">Claim 2 of 4</Badge>

# Staleness is data, not a promise.

- every card names the files it describes, with content hashes
- drift is computed by re-hashing those files, not asserted
- a stale card is a fact about bytes, not a vibe

---
layout: default
---

<Badge type="info">Claim 3 of 4</Badge>

# The index is a cache; the cards are the artifact.

- 48 `.yaml` cards are committed and reviewed like source
- `index.sqlite` is gitignored — delete it, `kuda sync` rebuilds
- a bad summary is a diff someone can reject

---
layout: default
---

<Badge type="info">Claim 4 of 4</Badge>

# One binary speaks cli, mcp and acp.

- same `kuda.toml`, same `.kuda/`, three entry points
- mcp for tool-calling agents, acp for session drivers
- no daemon, no server, no API key

---
layout: section
---

# II. The evidence

One receipt per claim, in the same order.

---
layout: fact
---

<Badge type="tip">Evidence 1</Badge>

# 81

lines of `.kuda/index.yaml` — the always-load card for
20 crates and 8 194 lines of Rust

---
layout: default
---

<Badge type="tip">Evidence 2</Badge>

# Each card names its files, and their hashes

```yaml
# .kuda/modules/kuda-core.yaml
provenance:
  files:
  - crates/kuda-core/Cargo.toml
  - crates/kuda-core/src/lib.rs
  hashes:
  - 858a2d26…
  - e218eed6…
  commit_sha: e360ec66…
```

`kuda status` re-hashes those files; the argument is in `crates/kuda-core/src/lib.rs`.

---
layout: default
---

<Badge type="tip">Evidence 3</Badge>

# 48 files git carries; the cache is ignored

```sh
$ git ls-files .kuda | wc -l
48
$ git check-ignore .kuda/index.sqlite
.kuda/index.sqlite
```

---
layout: default
---

<Badge type="tip">Evidence 4</Badge>

# Three surfaces, one binary

| surface | entry point | what it exposes |
| --- | --- | --- |
| cli | `kuda <verb>` | sync, status, map, card, find, dependents, refs, watch |
| mcp | `kuda mcp` | tools: `repo_card` `module_card` `find` `dependents` `status` `refs` |
| acp | `kuda acp` | the session surface an agent driver talks to |

All three read the same `.kuda/` and the same `kuda.toml`.

---
layout: section
---

# III. Try it

Four commands, five minutes, one visible result.

---
layout: default
---

<Badge type="tip">Tutorial</Badge>

# Four commands

```sh
cargo install --git https://github.com/9aia/kuda kuda
kuda init      # kuda.toml + .kuda/ skeleton
kuda sync      # extract -> cards -> index
kuda map       # the L0 card, ~1k tokens
```

---
layout: default
---

<Badge type="tip">Tutorial</Badge>

# What you should see

```text
kuda.toml
.kuda/
  index.yaml      L0  — the whole repo, 81 lines
  modules/        L1  — one card per crate (20)
  files/          L1.5 — per-file cards (27)
  index.sqlite    L2  — gitignored, rebuilt by `kuda sync`
```

Then `kuda status` tells you which cards drifted.

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
- parked: `dist` releases — `cargo install --git` + the `v0.1.0` tag suffice (TODO.md)
- agent-maintained — issues and PRs disabled; CI dogfooded

---
layout: default
---

# References

- **Repository** — [github.com/9aia/kuda](https://github.com/9aia/kuda)
- **Docs** — [9aia.com/kuda](https://9aia.com/kuda)
- **The argument in code** — `crates/kuda-core/src/lib.rs`
- **The committed cards** — `.kuda/index.yaml` and `.kuda/modules/`
- **The auditor** — [mimus](https://github.com/9aia/mimus) trust-scores these cards
- **Install** — `cargo install --git https://github.com/9aia/kuda kuda`

---
layout: end
---

# kuda

github.com/9aia/kuda · 9aia.com/kuda

Index your repo, read the cards, diff the next sync.
