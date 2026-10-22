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
    for sub in ["init", "sync", "status", "map", "card", "find", "dependents", "refs", "mcp", "acp", "doctor"] {
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

/// Mini python monorepo: dash-named dist `neuralmind-shared`,
/// imported as `neuralmind_shared`. Regression for the field
/// report — dependents returned a false empty, index.yaml embedded
/// every file, module cards dumped private symbols.
#[test]
fn sync_monorepo_edges_and_bounded_cards() {
    let tmp = std::env::temp_dir().join(format!("kuda-mono-{}", std::process::id()));
    let mk = |p: &str, text: &str| {
        let f = tmp.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, text).unwrap();
    };
    mk("README.md", "# mono\nfixture repo for e2e\n");
    mk("packages/neuralmind-shared/pyproject.toml",
       "[project]\nname = \"neuralmind-shared\"\ndependencies = [\"httpx>=0.27\"]\n");
    mk("packages/neuralmind-shared/neuralmind_shared/__init__.py",
       "def helper():\n    pass\n\nclass Registry:\n    pass\n");
    mk("packages/neuralmind-shared/neuralmind_shared/_internal.py",
       "def _secret():\n    pass\n");
    mk("packages/api/pyproject.toml",
       "[project]\nname = \"api\"\ndependencies = [\"neuralmind-shared\"]\n");
    mk("packages/api/main.py",
       "import neuralmind_shared\nfrom neuralmind_shared._internal import _secret\n\ndef serve():\n    pass\n");
    mk("packages/api/tests/test_api.py", "def test_serve():\n    pass\n");
    // >40 symbols incl. privates in one module -> api cap must bite
    let mut big = String::new();
    for i in 0..50 {
        big.push_str(&format!("def public_{i}():\n    pass\ndef _priv_{i}():\n    pass\n"));
    }
    mk("packages/api/surface.py", &big);
    mk("Makefile", ".PHONY: test\nVAR := 1\ntest:\n\tpytest\nlint: check\n\truff\n");
    mk("package.json", "{\"name\":\"mono\",\"scripts\":{\"test\":\"vitest\",\"build\":\"tsc\"}}");

    let out = kuda(&["sync"], &tmp);
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));

    // dependents: both the manifest dep and the import land on the
    // canonical dash-named module.
    let out = kuda(&["dependents", "neuralmind-shared"], &tmp);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("api"), "dependents: {stdout}");
    assert!(!stdout.contains("nothing depends"), "{stdout}");

    // L0 card stays an always-load blob, not a file dump.
    let index = std::fs::read_to_string(tmp.join(".kuda/index.yaml")).unwrap();
    assert!(index.lines().count() < 200, "index.yaml {} lines", index.lines().count());
    assert!(index.contains("tree_digest"), "{index}");
    assert!(index.contains("files_total"), "{index}");
    assert!(index.contains("make test") || index.contains("make lint"),
        "commands: {index}");
    assert!(index.contains("npm run"), "commands: {index}");
    assert!(index.contains("pytest"), "commands: {index}");

    // module card: no privates, api capped with the remainder counted.
    let card = std::fs::read_to_string(tmp.join(".kuda/modules/api.yaml")).unwrap();
    assert!(!card.contains("_priv_"), "{card}");
    assert!(card.contains("api_omitted"), "{card}");
    std::fs::remove_dir_all(&tmp).ok();
}

/// `kuda map | head` must exit 0, not panic on SIGPIPE.
#[test]
fn map_survives_broken_pipe() {
    let tmp = std::env::temp_dir().join(format!("kuda-pipe-{}", std::process::id()));
    std::fs::create_dir_all(tmp.join(".kuda")).unwrap();
    // >64KB so the write can't land entirely in the pipe buffer
    let big = "x".repeat(200_000);
    std::fs::write(tmp.join(".kuda/index.yaml"), &big).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(["map"])
        .current_dir(&tmp)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take()); // reader gone before the write lands
    assert!(child.wait().unwrap().success());
    std::fs::remove_dir_all(&tmp).ok();
}
