# The release model

> User-facing install is the tamias pack (`libraries/kuda` →
> `tamias install 9aia:kuda@<ver>`); the `kuda` crates are lib-only
> engines behind `tamias-driver-kuda` — there is no cargo-installed
> `kuda` binary and **no crates.io publish** (org policy keeps every
> repo but tamias off crates.io). This page describes the tag release
> pipeline that feeds the pack.

Releases are manual: `scripts/release.sh` is run by an operator
on `main`. The script's leading sleep is the debounce window — a
newer push kills a running release via `cancel_running`, so only
the last push in a quiet window releases.

The org release plane is **tarsi** (`tarsi-serve` ops — project
ADR-0005): `scripts/release.sh` is being converted to a thin shim
over it (PL-014 E1), and until then this script implements the
same tag-only + `tamias publish` model directly.

## What a release run does

1. **Diff since the last tag** — release-worthy paths are the
   workspace crates (`crates/`), the tamias pack payload
   (`libraries/`), the language/domain corpora (`data/` — compiled
   into the binary via `include_str!`), the pack staging script
   (`scripts/pack.sh`), the workspace manifests/lockfile, and the
   release script itself. Changes to anything else trigger
   nothing. No release-set changes, no release.
2. **Workspace version bump** — every crate inherits
   `version.workspace`, so `cargo set-version --workspace --bump
   patch` moves them all at once and rewrites the
   `workspace.dependencies` version reqs and `Cargo.lock`
   together. The script then syncs the user-facing version refs:
   `libraries/kuda/tamias.yaml` (`version:`) and the install
   example in `README.md` (`9aia:kuda@<ver>`).
3. **Commit + tag + push** — one `release: bump kuda to <ver>`
   commit, tagged `kuda-v<ver>`, pushed to `main` with the tag.
4. **Pack publish (operator)** — the script prints the follow-up:
   `scripts/pack.sh && tamias publish libraries/kuda` ships
   `9aia:kuda@<ver>` to the org registry (`pack.sh` first
   release-builds `tamias-driver-kuda` into the pack's
   `drivers/bin/<triple>/` payload dir). Publishing is a
   credentials-gated, registry-side action and deliberately stays
   outside the script.

With no tags at all the script exits early — the flow is seeded
by tagging once.

## Constraints

- There is no crates.io publish path and no per-crate tag scheme —
  the whole workspace shares one version and one `kuda-v<ver>`
  tag. The old push-driven `kuda deploy release` seed (which
  `cargo publish`ed every member and uploaded cargo-dist binaries
  to a GitHub Release) is retired: org policy keeps every repo but
  tamias off crates.io, and prebuilt binary distribution is not
  part of the flow.
- Only the tamias pack is user-facing; `tamias-driver-kuda` and
  the engine crates are internal, installed via the pack's
  generated `bins[]` shim (`tamias install --allow-drivers
  ./libraries/kuda` from a checkout).
