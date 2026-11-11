# Getting started

Pick by what you need to do.

- **Tutorials** (learn): [first-run](tutorials/first-run.md)
- **How-to** (tasks): [multi-repo](how-to/multi-repo.md) ·
  [mcp](how-to/mcp.md) ·
  [llm summarizer](how-to/llm-summarizer.md)
- **Reference** (lookup): [cli](reference/cli.md) ·
  [config](reference/config.md) ·
  [crate api](reference/crate-api.md)
- **Explanation** (why): [architecture](explanation/architecture.md) ·
  [release model](explanation/release-model.md) ·
  [comparison](comparison.md) — vs grep / RAG

Install — the `kuda` command is a tamias pack (`libraries/kuda`), a
generated `bins[]` shim → `tamias-driver-kuda` → the engine lib. With
`tamias` installed:

```sh
tamias install 9aia:kuda@0.1.0    # org registry (needs TAMIAS_REGISTRY_TOKEN)
```

Or from a kuda checkout:

```sh
scripts/pack.sh                                  # stage the driver payload
tamias install --allow-drivers ./libraries/kuda   # deploys `kuda`
```

First index:

```sh
cd your-repo
kuda init    # detect languages -> kuda.toml + .kuda/
kuda sync    # incremental index -> .kuda/ artifacts
kuda status  # provenance vs HEAD
kuda map     # the L0 card — what agents load first
```
