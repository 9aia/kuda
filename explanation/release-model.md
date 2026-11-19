---
url: /kuda/explanation/release-model.md
---
# The release model

Releases are **tag-only** — there is no crates.io publish and no
GitHub Releases artifact. `kuda`'s crates are lib-only engines;
the user-facing command is the tamias pack in `libraries/kuda`.

## The flow

Releases are operator-run: `tarsi-serve release --dir .` talks to
[`tarsi-serve`](https://9aia.com/tarsi/) — the org release plane
(project ADR-0005) — directly; the per-repo `scripts/release.sh`
shim was removed org-wide. One call runs plan → changelog → bump →
tag → push; `tarsi-serve release` is dry-run by default and
`--yes` is the explicit opt-in.

Repo policy lives in `tarsi.toml`:

* `[publish.crates] enabled = false` — crates.io publishing is
  tamias-only (ADR-0004).
* `[publish.ghreleases] enabled = false` — org releases are
  tag-only; the capability stays for opt-in.
* `[release.sync_refs]` rewrites `libraries/kuda/tamias.yaml`'s
  `version:` and the README's `kuda@<ver>` install ref inside the
  release commit, so the tagged tree is self-consistent.

Units default to cargo workspace members; versions are
workspace-inherited, so releasing units share one bump.

## Shipping the command

Tagging doesn't publish anything user-facing. The pack ships
manually:

```sh
scripts/pack.sh                                # stage driver payload
#   → release-builds tamias-driver-kuda into
#     libraries/kuda/drivers/bin/<triple>/ (gitignored)
tamias publish libraries/kuda                  # → 9aia:kuda@<ver>
```

Install is then `tamias install 9aia:kuda@<ver>`
(needs `TAMIAS_REGISTRY_TOKEN`) or
`tamias install --allow-drivers ./libraries/kuda`
from a checkout.

## Why this shape

* **Tag-only + pack publish** matches the org distribution
  model: git/tag deps and the tamias registry are the channels;
  crates.io and GitHub Releases stay unused.
* **tarsi over bespoke scripts**: plan/changelog/bump/tag is the
  same pipeline every org repo runs — the repo keeps only policy
  (`tarsi.toml`), not machinery.
* **`dist-workspace.toml` is configured but idle** — prebuilt
  binary distribution (cargo-dist + GitHub Releases) remains a
  possible future option, not part of the release flow.
