# Changelog

## [0.3.0] - 2026-11-24

### Added

- **driver:** tamias-driver-kuda — pavo view driver over the kuda index (`f0a4339`)
- **kuda-driver:** cli verb is an engine passthrough — the tamias bins[] shim contract (`ebca8ed`)
- **kuda-driver:** embedded cli/mcp surfaces + typed op params (`ae8744c`)
- **kuda-driver:** engine ops over the wire — ops manifest + cli.* dispatch to kuda::dispatch (`28493c4`)

### Changed

- **kuda-driver:** adopt pavo-driver-kit for serve loop and manifest builders (`fa432a5`)
- **config:** scarab-config-file discovery — kuda.{yaml,yml,toml} (P-017) (`bda9016`)
- **vcs:** consume lasius-vcs port — drop kuda-vcs/kuda-git (`7e9ec7c`)
- **driver:** derive ops manifest from kuda::command() (`6690b1b`)
