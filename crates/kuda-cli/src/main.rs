//! kuda — compat CLI driver. The engine is the `kuda` lib
//! (src/lib.rs); this binary is just `kuda::run()`.

fn main() {
    std::process::exit(kuda::run());
}
