---
layout: home

hero:
  name: kuda
  text: a hippocampus for your codebase
  tagline: repo memory — compressed, freshness-tracked, queryable
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
    details: CLI for humans and scripts, MCP for agents — cards end in verify: lists so memory stays a hypothesis
    link: /how-to/mcp
    linkText: MCP setup
---
