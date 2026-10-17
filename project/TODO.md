# TODO — 0.1.0

## Core model
- [ ] `kuda-core` — RepoCard / ModuleCard / Symbol / DepEdge /
      Provenance{files,hash,sha}; Lang descriptors loaded from
      `data/langs/*.toml` — a new language is a data change.

## Ports + first impls
- [ ] `kuda-extract` port + `kuda-manifest` impl (cargo/npm/go/py
      manifests -> package graph; no parser needed — lands first)
- [ ] `kuda-treesitter` impl — grammar+query data per language
      (rust first)
- [ ] `kuda-store` port + `kuda-store-sqlite` impl (L2 graph)
- [ ] `kuda-vcs` port + `kuda-git` impl (sha, changed files ->
      staleness) — bupha's vcs/git crates already fit; repurpose
- [ ] `kuda-summarize` port + `kuda-template` impl (deterministic
      cards: API list, deps, commands — zero-model default)

## Pipeline + queries
- [ ] `kuda sync` — incremental: hash -> re-extract changed ->
      regen affected cards only; `--full` for cold start
- [ ] `kuda status` — provenance sha vs HEAD -> per-card drift
- [ ] `kuda-query` — find / dependents / card / refs (--docs)
- [ ] `kuda-cards` — .kuda/index.yaml + modules/*.yaml render;
      machine owns api/deps fields, humans own `notes:` region;
      every card ends with `verify:` file list

## Scaffold debt (forked from bupha — rewrite crate-by-crate)
- [ ] Crates still implement bupha's CI domain. Per ADR-0001:
      repurpose `kuda-core`/`kuda-config*`/`kuda-secret*`/`kuda-vcs`/
      `kuda-git`/`kuda-cli`; drop `kuda-trigger`, `kuda-queue*`,
      `kuda-runner`/`kuda-dagger`/`kuda-shell`, `kuda-tunnel*`,
      `kuda-service`/`kuda-systemd`, `kuda-status`, `kuda-github`,
      `kuda-cache*`; hold `kuda-ssh` (remote-index candidate).
- [ ] `modules/dagger` — keep as kuda's own CI module (dogfoods bupha).
