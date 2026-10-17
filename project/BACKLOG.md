# BACKLOG — horizon

Ideas parked, not scheduled. Pull into project/TODO.md when picked up.

## Extractors
- [ ] `kuda-lsp` impl (wanted next)
- [ ] `kuda-acp` impl — ACP (Agent Client Protocol) surface so any
      agent client (cline, cursor, zed) queries the index natively — SCIP / workspace-symbol crawl for cross-file
      refs where a server exists (rust-analyzer, gopls, tsserver)
- [ ] Per-lang data packs: python, ts, go, java — grammar+query files
      under `data/langs/`

## Summarizers
- [ ] `kuda-llm` impl — prose cards, provider-pluggable; grounding
      discipline: cards cite files (`verify:`), no invented facts
- [ ] Embedding/RAG impl — optional L2 backend for semantic search
      (see ADR-0001 option B)

## Workspace / multi-repo
- [ ] `kuda-workspace` — workspace.yaml at parent level; git-dep ->
      sibling-checkout resolution; cross-repo `dependents`

## Surfaces
- [x] `kuda-mcp` — MCP server impl; agents query index as tools
- [ ] `kuda watch` — reindex on file events (daemon)
- [ ] `kuda session` — episodic memory: learnings from agent sessions
      distilled into cards (semantic vs episodic split)

## Ecosystem
- [ ] `malurus` — sibling critic tool (9aia/malurus): extracts claims
      from docs/comments/cards and verifies them against this index;
      third tool "reconcile" auto-fixes confirmed violations
- [ ] Dogfood target: index the handl-lang org (~70 sibling repos)
      as the proving ground

## CI / release
- [ ] Dogfood bupha for kuda's own CI (bupha.toml + modules/dagger)
