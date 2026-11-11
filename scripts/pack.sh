#!/usr/bin/env bash
# Stage the tamias pack payload (T-012): release-build
# `tamias-driver-kuda` and drop it under
# `libraries/kuda/drivers/bin/<host-triple>/` — the bundled-driver
# dir `tamias install` registers into `<tamias-home>/drivers/`.
# Gitignored on purpose: the binary is a build artifact; the
# committed pack is just `tamias.yaml` + this script.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release -p kuda-driver

triple="$(rustc -vV | sed -n 's/^host: //p')"
triple="${triple:-x86_64-unknown-linux-gnu}"
dest_dir="libraries/kuda/drivers/bin/$triple"
mkdir -p "$dest_dir"

# CARGO_TARGET_DIR-aware: `cargo build` honors it, so the artifact
# lookup must too (default: ./target).
target_dir="${CARGO_TARGET_DIR:-$PWD/target}"
cp "$target_dir/release/tamias-driver-kuda" "$dest_dir/tamias-driver-kuda"
chmod 755 "$dest_dir/tamias-driver-kuda"

echo "pack staged: $dest_dir/tamias-driver-kuda"
echo
echo "install the kuda command (generated bins[] shim):"
echo "  tamias install --allow-drivers ./libraries/kuda"
echo "  # or a pinned tarball: tamias bundle pack libraries/kuda"
