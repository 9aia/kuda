# Changelog

## [0.3.0] - 2026-11-24

### Added

- **extract:** records adapter — domain corpora as indexable data (`b1927ee`)
- **records:** schema-validated corpora — verify record structure, not just shape (`31359c5`)
- file-level incremental sync — walk once, in-process blob hashes, store diff -> treesitter parses only changed modules' files; per-module file-hash rows in sqlite; seed from committed cards on upgrade; module identity = manifest pkg name (fixes dir-name split) (`2f0a2e7`)

### Changed

- **crates:** rename extract adapters to kuda-extract-<impl> (`78f3201`)
- crate restructure — drop CI domain, land memory topology (ADR-0001) (`3432d05`)
