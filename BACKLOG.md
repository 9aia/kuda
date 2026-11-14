# Backlog

## Extractors

- T-013 kuda-lsp adapter (wanted next)
- T-014 kuda-acp adapter — ACP (Agent Client Protocol) surface so any agent client (cline, cursor, zed) queries the index natively — SCIP / workspace-symbol crawl for cross-file refs where a server exists (rust-analyzer, gopls, tsserver)
- T-015 Per-lang data packs: python, ts, go, java — grammar+query files under data/langs/
- T-028 agent-awareness managed block — kuda init should write \<!-- kuda:agent-awareness --> into AGENTS.md (pattern: myco init) + validate block freshness in kuda status/check. Block text already deployed to the 6 .kuda-wired repos (cards-are-artifacts, kuda status drift, sync-on-change) · created:2026-11-02
- T-010 Embedding-based claim clustering for doc↔doc contradiction detection (today: identifier-token clustering — finds the obvious pairs, misses paraphrases). Needs a local embedder or embedding API; evaluate cost vs precision first.
- T-026 presentation deck — verify + integrate: slides v2 (claim/proof, 3 chapters) pushed on presentation/initial (e1844e5); deck session reported it still fails verification — run a repair/verify pass, then merge per sturn flow (no PRs — pushed branch is the deliverable) · created:2026-11-02

## Summarizers

- T-016 kuda-llm adapter — prose cards, provider-pluggable; grounding discipline: cards cite files ( verify: ), no invented facts
- T-017 Embedding/RAG adapter — optional L2 backend for semantic search (see ADR-0001 option B)

## Workspace / multi-repo

- T-018 kuda-workspace — workspace.yaml at parent level; git-dep -> sibling-checkout resolution; cross-repo dependents

## Surfaces

- T-019 kuda watch — reindex on file events (daemon)
- T-020 kuda session — episodic memory: learnings from agent sessions distilled into cards (semantic vs episodic split)

## Ecosystem

- T-021 malurus — sibling critic tool (9aia/malurus): extracts claims from docs/comments/cards and verifies them against this index; third tool "reconcile" auto-fixes confirmed violations
- T-022 Dogfood target: index the handl-lang org (~70 sibling repos) as the proving ground

## CI / release

- T-023 Dogfood bupha for kuda's own CI (bupha.toml + modules/dagger)
