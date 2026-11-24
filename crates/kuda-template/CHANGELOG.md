# Changelog

## [0.3.0] - 2026-11-24

### Added

- **records:** schema-validated corpora — verify record structure, not just shape (`31359c5`)

### Changed

- crate restructure — drop CI domain, land memory topology (ADR-0001) (`3432d05`)

### Fixed

- monorepo scale — bounded cards, real dep edges, SIGPIPE-safe output (`e360ec6`)
- template doc-comment truth + assert acp in help smoke (semantic dogfood findings) (`9cc2ada`)
