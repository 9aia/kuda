#!/usr/bin/env bash
# Run manually: `./scripts/release.sh` from the repo root.
# The leading sleep is the debounce window: a newer push kills this
# run via cancel_running, so only the last push in a quiet window
# releases. Runs inside a detached worktree — commits/tags are pushed
# from here, not from the main checkout.
#
# crates.io publish is retired: org policy keeps every repo but tamias
# off crates.io, and the kuda crates are internal lib-only engines
# behind `tamias-driver-kuda` — the `kuda` command ships as the tamias
# pack in `libraries/kuda` (`tamias install 9aia:kuda@<ver>`), not as
# a cargo-installed binary. A release is therefore one workspace
# version bump + a `kuda-v<ver>` tag; the operator then publishes the
# pack itself (`scripts/pack.sh` stages the driver payload, then
# `tamias publish libraries/kuda`). Binary dist (cargo-dist + gh
# release upload) is out of scope.
# Needs: `cargo-edit` (cargo set-version).
set -euo pipefail
# worktree-local env (gitignored) — kept for parity with the other
# org release scripts; nothing here needs secrets today.
[ -f ./.env ] && { set -a; source ./.env; set +a; }
sleep "${KUDA_RELEASE_DEBOUNCE:-300}"

git fetch origin --tags -q
last="$(git describe --tags --abbrev=0 2>/dev/null || true)"
if [ -z "$last" ]; then
    echo "no tags yet — tag the first release manually to seed the flow"; exit 0
fi
changed="$(git diff --name-only "${last}..HEAD")"

# release-worthy paths: workspace crates, the tamias pack payload,
# the language/domain corpora compiled into the binary (`data/` is
# include_str!-ed), the pack staging script, and the release
# machinery itself. Docs live on the orphan `docs` worktree branch —
# they never appear in this diff.
relevant="$(grep -E '^(crates/|libraries/|data/|Cargo\.(toml|lock)|scripts/(release|pack)\.sh)' <<<"$changed" || true)"
if [ -z "$relevant" ]; then echo "no release-relevant changes since $last"; exit 0; fi

# every crate inherits `version.workspace` — one bump moves them all,
# and `cargo set-version --workspace` rewrites the workspace.package
# version, the workspace.dependencies reqs, and Cargo.lock together.
old="$(grep -m1 '^version = "' Cargo.toml | cut -d'"' -f2)"
cargo set-version --workspace --bump patch
ver="$(grep -m1 '^version = "' Cargo.toml | cut -d'"' -f2)"

# keep the user-facing version refs in lockstep: the tamias pack
# manifest and the install example.
sed -i "s/^version: \"${old}\"/version: \"${ver}\"/" libraries/kuda/tamias.yaml
sed -i "s|kuda@${old}|kuda@${ver}|g" README.md

git commit -aqm "release: bump kuda to ${ver} — tamias pack release"
git tag "kuda-v${ver}"
git push -q origin HEAD:main --tags

echo "released: kuda ${ver} (tag kuda-v${ver})"
echo "next: scripts/pack.sh && tamias publish libraries/kuda  # ship 9aia:kuda@${ver} to the org registry"
