# Changelog

## [0.3.0] - 2026-11-24

### Added

- fuzzy find — exact miss falls back to LIKE substring tier, hits marked (fuzzy); search on store port (`f440700`)

### Changed

- crate restructure — drop CI domain, land memory topology (ADR-0001) (`3432d05`)

### Fixed

- monorepo scale — bounded cards, real dep edges, SIGPIPE-safe output (`e360ec6`)
- keep agent CoT out of cards, honest generator stamp, no self-edges (`ba00670`)
