# Changelog

## [0.3.0] - 2026-11-24

### Added

- kuda-acp — live-tested vs cline --acp: session reuse, request_permission auto-answer, section parser for agent prose (purpose/invariants/notes) (`ce36f25`)
- L1.5 file cards — chunked per-file agent summaries (text-splitter, code-aware via tree-sitter), per-file hash-gated; AcpSummarizer::complete() shares one session; [summarize] file_cards=true opts in (`a16f1e1`)
- kuda-acp summarizer — spawns ACP agent for grounded card prose, template fallback; kuda-llm parked (acp is the semantic path) (`444ff6e`)
- **records:** schema-validated corpora — verify record structure, not just shape (`31359c5`)

### Changed

- **acp:** consume canonical sepia-acp blocking client (PL-029) (`430615d`)

### Fixed

- monorepo scale — bounded cards, real dep edges, SIGPIPE-safe output (`e360ec6`)
- keep agent CoT out of cards, honest generator stamp, no self-edges (`ba00670`)
- **sync:** preserve human notes + reject monologue acp replies (I-002) (`572cee6`)
- **acp:** guard unparseable replies — warn + fallback, never pollute cards (I-002) (`0572a88`)
