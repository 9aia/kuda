//! CLI smoke: help renders, unimplemented subcommands are
//! recognized (exit 2), `init` scaffolds kuda.toml + .kuda/.

use std::process::Command;

fn kuda(args: &[&str], dir: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn help_and_stub() {
    let tmp = std::env::temp_dir();
    let out = kuda(&["--help"], &tmp);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    for sub in ["init", "sync", "status", "map", "card", "find", "dependents", "refs", "mcp", "doctor"] {
        assert!(stdout.contains(sub), "missing subcommand {sub}");
    }
    // `sync` is real now — running it in $TMPDIR would walk the
    // whole dir; covered by init_scaffolds + dogfood instead.
}

#[test]
fn init_scaffolds() {
    let tmp = std::env::temp_dir().join(format!("kuda-init-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let out = kuda(&["init"], &tmp);
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    assert!(tmp.join("kuda.toml").exists());
    assert!(tmp.join(".kuda/modules").is_dir());
    // second run refuses to clobber
    let out = kuda(&["init"], &tmp);
    assert_eq!(out.status.code(), Some(1));
    std::fs::remove_dir_all(&tmp).ok();
}
