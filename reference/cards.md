# Reference: card schema

Two artifact files, both committed YAML under `.kuda/` (location
configurable via `[cards].dir`).

## `.kuda/index.yaml` — the L0 repo card

```yaml
name: kuda                     # repo display name
root: /abs/path/to/repo
purpose: ''                    # manifest/tagline — human-editable
modules:                       # sorted module names
- kuda-core
- kuda-store
commands:                      # detected verify/build commands
- cargo check
- cargo test --workspace
tree_digest: 93183a1c…         # sha1 over sorted path:hash lines —
                               # every walked file, one line
files_total: 12401             # walked-file count (diagnostic)
provenance:
  files: [Cargo.toml, crates/kuda-store/Cargo.toml, README.md]
  hashes: [sha1, ...]          # card inputs only: manifests + README
  commit_sha: af26b8c…
  generated_at: '1712345678'   # unix seconds
  generator: kuda/0.1.0
```

`provenance.files` on the L0 card is deliberately bounded —
manifests decide module identity, the README feeds `purpose`, and
`tree_digest`/`files_total` fingerprint the rest. Embedding the
full walked set put ~10k rows in the always-load card; per-file
hash lists still live on the L1 module cards where `status`
actually checks them.

## `.kuda/modules/<name>.yaml` — an L1 module card

```yaml
name: kuda-store
path: crates/kuda-store/Cargo.toml
purpose: Store port — IndexStore trait          # manifest description
api:                           # extracted public symbols — bounded:
- IndexStore                   # `_`-privates dropped, sorted, capped
- SqliteStore                  # at 40 entries
api_omitted: 217               # symbols beyond the cap ("+N more")
deps:                          # outgoing edges
- from: kuda-store
  to: kuda-core
  kind: PackageDep
notes: ''                      # HUMAN-OWNED — preserved on regen
verify:                        # files to open before trusting —
- crates/kuda-store/src/lib.rs # entry points first, capped at 10
provenance:
  files: [...]
  hashes: [...]
  commit_sha: …
  generated_at: '…'
  generator: kuda-template/0.1.0
```

## Provenance semantics

`files` + `hashes` are git `hash-object` digests of the card's
inputs at sync time. `kuda status` (and `mimus audit`) re-hash
the same files and compare — a mismatch marks the card `drifted`.
`commit_sha` records the index point in history; `generator`
records which adapter produced the card so format drift is
auditable.

## Ownership

Machine fields (`api`, `deps`, `provenance`) are regenerated on
every `sync`. `notes:` and `purpose:` (when hand-edited past the
manifest default) are the human regions — a real merge story is
a TODO; today regen overwrites `purpose` only when it still
equals the old machine value.
