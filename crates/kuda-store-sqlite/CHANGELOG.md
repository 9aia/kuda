# Changelog

## [0.3.0] - 2026-11-24

### Added

- fuzzy find — exact miss falls back to LIKE substring tier, hits marked (fuzzy); search on store port (`f440700`)
- **driver:** tamias-driver-kuda — pavo view driver over the kuda index (`f0a4339`)
- **extract:** records adapter — domain corpora as indexable data (`b1927ee`)
- file-level incremental sync — walk once, in-process blob hashes, store diff -> treesitter parses only changed modules' files; per-module file-hash rows in sqlite; seed from committed cards on upgrade; module identity = manifest pkg name (fixes dir-name split) (`2f0a2e7`)

### Changed

- crate restructure — drop CI domain, land memory topology (ADR-0001) (`3432d05`)

### Fixed

- monorepo scale — bounded cards, real dep edges, SIGPIPE-safe output (`e360ec6`)
