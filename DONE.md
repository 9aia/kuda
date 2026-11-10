# Done

- T-030 kuda engine→tamias driver refactor (phases) · owner:devin · created:2026-11-10
- T-032 repo-local \[\[domain\]\] descriptors + \[domains\].disable in kuda.toml — DomainsCfg in kuda-core, resolve\_domains in cli, docs+tests · created:2026-11-10
- T-033 non-file-tree corpus support — sturn WAL/git-refs as first non-tree domain source (DomainDef.source enum, records extractor dispatch, data/domains/sturn.toml, docs) · owner:devin · created:2026-11-10
- T-031 surface drivers for kuda watch/mcp/acp — tamias-driver-cli / tamias-driver-mcp pattern doesn't exist yet (pavo-driver-kit is view-driver only); kuda-driver ops.rs notes the gap · owner:devin · created:2026-11-10
- T-001 kuda-core — RepoCard / ModuleCard / Symbol / DepEdge / Provenance{files,hash,sha}; Lang descriptors loaded from data/langs/\*.toml — a new language is a data change.
- T-002 kuda-extract port + kuda-manifest adapter (cargo/npm/go/py manifests -> package graph; no parser needed — lands first)
- T-003 kuda-treesitter adapter — grammar+query data per language (rust first)
- T-004 kuda-store port + kuda-store-sqlite adapter (L2 graph)
- T-005 kuda-vcs port + kuda-git adapter (sha, changed files -> staleness) — bupha's vcs/git crates already fit; repurpose
- T-006 kuda-summarize port + kuda-template adapter (deterministic cards: API list, deps, commands — zero-model default)
- T-007 kuda-query — find / dependents / card / refs (--docs)
- T-008 kuda-cards — .kuda/index.yaml + modules/\*.yaml render; machine owns api/deps fields, humans own notes: region; every card ends with verify: file list
- T-009 kuda init / sync should append .kuda/index.sqlite + .kuda/meta.json to .gitignore automatically — cards are committed artifacts, the graph is rebuildable. Found via dogfood: mimus+bupha committed the sqlite binary by hand.
- T-025 verify claim .kuda/modules/Cargo.toml.yaml:3 ('Cargo.toml') — file missing, card likely stale; recovered from dead sessions 1792289007778\_292qm, 1792289232239\_Jpne2, 1792289295855\_Xr\_YW, 1792289355290\_bAAOh, 1792289567602\_H2XTF · created:2026-10-30
- T-034 Rename extract-impl crates to kuda-extract-\<impl> (manifest, treesitter, lsp) + regen cards · pri:high · owner:devin · created:2026-11-10
- T-035 kuda watch — domain roots + WAL-ref coverage: poll-refs flag, root(cli) fix, dir\_excluded filter (sync-lag reduction) · pri:high · owner:devin · created:2026-11-10
