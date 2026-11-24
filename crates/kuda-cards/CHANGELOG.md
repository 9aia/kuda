# Changelog

## [0.3.0] - 2026-11-24

### Added

- L1.5 file cards — chunked per-file agent summaries (text-splitter, code-aware via tree-sitter), per-file hash-gated; AcpSummarizer::complete() shares one session; [summarize] file_cards=true opts in (`a16f1e1`)
- **records:** schema-validated corpora — verify record structure, not just shape (`31359c5`)

### Changed

- crate restructure — drop CI domain, land memory topology (ADR-0001) (`3432d05`)
- rename memory tier L1.5 -> L1 file granularity (`0c540dd`)

### Fixed

- **sync:** preserve human notes + reject monologue acp replies (I-002) (`572cee6`)
