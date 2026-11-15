---
layout: home

hero:
  name: kuda
  text: a hippocampus for your codebase
  tagline: Code-map engine — verified per-module cards agents read before code
  actions:
    - theme: brand
      text: Getting Started
      link: /getting-started
    - theme: alt
      text: Tutorial
      link: /tutorials/first-run

features:
  - icon: 🧠
    title: Tiered memory
    details: L0 repo card (always-load), L1 module cards, L2 symbol/dep index — an index into code, never a copy
    link: /explanation/architecture
    linkText: architecture
  - icon: 🕰️
    title: Provable freshness
    details: every card carries files, content hashes, and the commit sha it was built at — staleness is a diff, not a guess
    link: /reference/cli
    linkText: kuda status
  - icon: 🔀
    title: Multi-repo workspaces
    details: sibling checkouts and git-dep edges resolve across repo boundaries — dependents() spans the whole workspace
    link: /how-to/multi-repo
    linkText: workspace setup
  - icon: 🧩
    title: Ports, not frameworks
    details: extract (manifest | treesitter | lsp), summarize (template | llm), store, vcs — swap via config or a new adapter crate
    link: /explanation/architecture
    linkText: architecture
  - icon: 📄
    title: Language as data
    details: per-language knowledge lives in grammar + query + manifest files — a new language is a data change, not a release
  - icon: 🤖
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

- **Learn it** → [Tutorial: your first index](tutorials/first-run.md)
- **Do a task** → [How-to](getting-started.md) — multi-repo
  workspaces, MCP/ACP surfaces, languages, corpora, sturn, LLM
  summarizers
- **Understand why** → [Architecture](explanation/architecture.md),
  [concepts](explanation/concepts.md),
  [decisions](explanation/decisions.md),
  [release model](explanation/release-model.md)
- **Look something up** → [CLI](reference/cli.md),
  [config](reference/config.md), [card schema](reference/cards.md),
  [crate API](reference/crate-api.md)
