---
layout: home

hero:
  name: kuda
  text: a hippocampus for your codebase
  tagline: Code-map engine — verified per-module cards agents read before code
  actions:
    - theme: brand
      text: Getting Started
      link: /tutorials/getting-started
    - theme: alt
      text: Multi-repo workspaces
      link: /how-to/multi-repo

features:
  - icon: '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 18V5" /> <path d="M15 13a4.17 4.17 0 01-3-4 4.17 4.17 0 01-3 4" /> <path d="M17.707 6.294A3 3 0 1012 5a3 3 0 10-5.707 1.294" /> <path d="M17.996 5.125a4 4 0 012.526 5.77" /> <path d="M18 18a4 4 0 002-7.465" /> <path d="M19.967 17.482a4 4 0 11-7.967.517 4 4 0 11-7.967-.517" /> <path d="M6 18a4 4 0 01-2-7.465" /> <path d="M6.004 5.125a4 4 0 00-2.526 5.77" /></svg>'
    title: Tiered memory
    details: L0 repo card (always-load), L1 module cards, L2 symbol/dep index — an index into code, never a copy
    link: /explanation/architecture
    linkText: architecture
  - icon: '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" /> <path d="M3 3v5h5" /> <path d="M12 7v5l4 2" /></svg>'
    title: Provable freshness
    details: every card carries files, content hashes, and the commit sha it was built at — staleness is a diff, not a guess
    link: /reference/cli
    linkText: kuda status
  - icon: '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 6a9 9 0 0 0-9 9V3" /> <circle cx="18" cy="6" r="3" /> <circle cx="6" cy="18" r="3" /></svg>'
    title: Multi-repo workspaces
    details: sibling checkouts and git-dep edges resolve across repo boundaries — dependents() spans the whole workspace
    link: /how-to/multi-repo
    linkText: workspace setup
  - icon: '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15.39 4.39a1 1 0 0 0 1.68-.474 2.5 2.5 0 1 1 3.014 3.015 1 1 0 0 0-.474 1.68l1.683 1.682a2.414 2.414 0 0 1 0 3.414L19.61 15.39a1 1 0 0 1-1.68-.474 2.5 2.5 0 1 0-3.014 3.015 1 1 0 0 1 .474 1.68l-1.683 1.682a2.414 2.414 0 0 1-3.414 0L8.61 19.61a1 1 0 0 0-1.68.474 2.5 2.5 0 1 1-3.014-3.015 1 1 0 0 0 .474-1.68l-1.683-1.682a2.414 2.414 0 0 1 0-3.414L4.39 8.61a1 1 0 0 1 1.68.474 2.5 2.5 0 1 0 3.014-3.015 1 1 0 0 1-.474-1.68l1.683-1.682a2.414 2.414 0 0 1 3.414 0z" /></svg>'
    title: Ports, not frameworks
    details: extract (manifest | treesitter | lsp), summarize (template | llm), store, vcs — swap via config or a new adapter crate
    link: /explanation/architecture
    linkText: architecture
  - icon: '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z" /> <path d="M14 2v5a1 1 0 0 0 1 1h5" /> <path d="M10 9H8" /> <path d="M16 13H8" /> <path d="M16 17H8" /></svg>'
    title: Language as data
    details: per-language knowledge lives in grammar + query + manifest files — a new language is a data change, not a release
  - icon: '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 8V4H8" /> <rect width="16" height="12" x="4" y="8" rx="2" /> <path d="M2 14h2" /> <path d="M20 14h2" /> <path d="M15 13v2" /> <path d="M9 13v2" /></svg>'
    title: Built for agents
    details: 'CLI for humans and scripts, MCP for agents — cards end in verify: lists so memory stays a hypothesis'
    link: /how-to/mcp
    linkText: MCP setup
---

kuda is the org's **code-map engine** — it turns a checkout into
verified per-module cards that agents load before planning instead
of re-exploring from scratch. It owns repo memory (extraction,
cards, the symbol graph, freshness) and nothing else: verification
is `mimus`'s job, distribution is tamias's, orchestration is
peregrin's.

## Which quadrant answers which goal

- **Learn it** → [Getting started](tutorials/getting-started.md) —
  install, first index, querying, drift
- **Do a task** → [How-to](tutorials/getting-started.md#where-next) — multi-repo
  workspaces, MCP/ACP surfaces, languages, corpora, sturn, LLM
  summarizers
- **Understand why** → [Architecture](explanation/architecture.md),
  [concepts](explanation/concepts.md),
  [decisions](explanation/decisions.md),
  [release model](explanation/release-model.md)
- **Look something up** → [CLI](reference/cli.md),
  [config](reference/config.md), [card schema](reference/cards.md),
  [crate API](reference/crate-api.md)
