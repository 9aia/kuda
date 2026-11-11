# kuda-cli

*Part of [kuda](https://github.com/9aia/kuda) — the oxpecker on your repo's back.*

The `kuda` engine as a library — `Cli`/`Cmd`,
`command()`/`parse_from()`/`dispatch()`/`run_from()`. There is no
`kuda` binary in this crate: the command ships as the tamias pack in
`libraries/kuda` (a generated `bins[]` shim → `tamias-driver-kuda
cli` → `kuda::run_from`). Engine e2e tests live in
`crates/kuda-driver/tests/` and spawn `tamias-driver-kuda run`.
