# Changelog

## [0.3.0] - 2026-11-24

### Added

- **config:** repo-local [[domain]] descriptors + [domains].disable (`fe49c9e`)
- **records:** non-tree domain sources — sturn WAL corpus in git refs (`e19ac36`)
- **extract:** records adapter — domain corpora as indexable data (`b1927ee`)
- L1.5 file cards — chunked per-file agent summaries (text-splitter, code-aware via tree-sitter), per-file hash-gated; AcpSummarizer::complete() shares one session; [summarize] file_cards=true opts in (`a16f1e1`)
- sync pipeline — manifest extract + template cards + sqlite store; status/map/find/dependents/refs queries; dogfooded on self (`63d73a7`)
- kuda-acp summarizer — spawns ACP agent for grounded card prose, template fallback; kuda-llm parked (acp is the semantic path) (`444ff6e`)
- **records:** schema-validated corpora — verify record structure, not just shape (`31359c5`)
- **records:** file-as-record corpora — any bytes join the graph (`2426776`)
- [extract] exclude config + broader built-in walk exclusions (dist/build/out/.cache/vendor/.venv/__pycache__/coverage/…) — keeps generated+vendored dirs out of index, hashes, and agent prompts; e2e stub assertion retired (sync is real) (`1d6d512`)

### Changed

- **config:** scarab-config-file discovery — kuda.{yaml,yml,toml} (P-017) (`bda9016`)
- **crates:** rename extract adapters to kuda-extract-<impl> (`78f3201`)
- crate restructure — drop CI domain, land memory topology (ADR-0001) (`3432d05`)
- rename memory tier L1.5 -> L1 file granularity (`0c540dd`)

### Fixed

- dep_query per-language data (data/langs/*.dep_query) — no more rust-only use_declaration on py/ts (`ee77d75`)
- monorepo scale — bounded cards, real dep edges, SIGPIPE-safe output (`e360ec6`)
