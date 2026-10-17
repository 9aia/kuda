# Reference: CLI

Generated from `kuda --help` by `bun run docs:cli` — seed listing:

```
kuda init              detect langs -> kuda.toml + .kuda/
kuda sync [--full]     incremental index
kuda status            provenance vs HEAD -> per-card drift
kuda map               render the L0 repo card
kuda card <module>     L1 card + freshness
kuda find <symbol>     symbol -> file:line
kuda dependents <x>    reverse dep lookup
kuda refs <x> [--docs] code + docs/comments mentions
kuda mcp [--workspace] serve the index over MCP
kuda doctor            extractor/store health
```

Global flags: `--config <path>` (else `$KUDA_CONFIG`, `./kuda.toml`,
`~/.config/kuda/kuda.toml`), `--workspace` (multi-repo mode).
