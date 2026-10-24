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
    for sub in [
        "init",
        "sync",
        "status",
        "map",
        "card",
        "find",
        "dependents",
        "refs",
        "mcp",
        "acp",
        "doctor",
    ] {
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
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
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
    mk(
        "packages/neuralmind-shared/pyproject.toml",
        "[project]\nname = \"neuralmind-shared\"\ndependencies = [\"httpx>=0.27\"]\n",
    );
    mk(
        "packages/neuralmind-shared/neuralmind_shared/__init__.py",
        "def helper():\n    pass\n\nclass Registry:\n    pass\n",
    );
    mk(
        "packages/neuralmind-shared/neuralmind_shared/_internal.py",
        "def _secret():\n    pass\n",
    );
    mk(
        "packages/api/pyproject.toml",
        "[project]\nname = \"api\"\ndependencies = [\"neuralmind-shared\"]\n",
    );
    mk("packages/api/main.py",
       "import neuralmind_shared\nfrom neuralmind_shared._internal import _secret\n\ndef serve():\n    pass\n");
    mk(
        "packages/api/tests/test_api.py",
        "def test_serve():\n    pass\n",
    );
    // >40 symbols incl. privates in one module -> api cap must bite
    let mut big = String::new();
    for i in 0..50 {
        big.push_str(&format!(
            "def public_{i}():\n    pass\ndef _priv_{i}():\n    pass\n"
        ));
    }
    mk("packages/api/surface.py", &big);
    mk(
        "Makefile",
        ".PHONY: test\nVAR := 1\ntest:\n\tpytest\nlint: check\n\truff\n",
    );
    mk(
        "package.json",
        "{\"name\":\"mono\",\"scripts\":{\"test\":\"vitest\",\"build\":\"tsc\"}}",
    );

    let out = kuda(&["sync"], &tmp);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    // dependents: both the manifest dep and the import land on the
    // canonical dash-named module.
    let out = kuda(&["dependents", "neuralmind-shared"], &tmp);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("api"), "dependents: {stdout}");
    assert!(!stdout.contains("nothing depends"), "{stdout}");

    // L0 card stays an always-load blob, not a file dump.
    let index = std::fs::read_to_string(tmp.join(".kuda/index.yaml")).unwrap();
    assert!(
        index.lines().count() < 200,
        "index.yaml {} lines",
        index.lines().count()
    );
    assert!(index.contains("tree_digest"), "{index}");
    assert!(index.contains("files_total"), "{index}");
    assert!(
        index.contains("make test") || index.contains("make lint"),
        "commands: {index}"
    );
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

/// L1.5 housekeeping: `kuda sync` must remove file cards for
/// deleted/renamed sources and dirs of vanished modules, and keep
/// cards whose file still exists. Runs under the default config
/// (no acp, `file_cards` off) on purpose — cleanup is
/// housekeeping, not summarization, so it must not be gated on
/// the summarize settings that produced the cards.
#[test]
fn sync_sweeps_stale_file_cards() {
    let tmp = std::env::temp_dir().join(format!("kuda-sweep-{}", std::process::id()));
    let mk = |p: &str, text: &str| {
        let f = tmp.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, text).unwrap();
    };
    mk("pkg/pyproject.toml", "[project]\nname = \"mymod\"\n");
    mk("pkg/a.py", "def a():\n    pass\n");
    mk("pkg/b.py", "def b():\n    pass\n");
    // second module with no .kuda/files dir at all -> readdir skip
    mk("pkg2/pyproject.toml", "[project]\nname = \"mod2\"\n");
    mk("pkg2/c.py", "def c():\n    pass\n");
    // hand-crafted L1.5 cards: one live (pkg/a.py -> pkg__a.py),
    // one for a deleted file, one under a module that vanished.
    let card = |path: &str, module: &str| {
        format!(
            "path: {path}\nmodule: {module}\npurpose: stub card\nsymbols: []\n\
             provenance:\n  files: [{path}]\n  hashes: [deadbeef]\n  \
             commit_sha: deadbeef\n  generated_at: \"0\"\n  generator: test\n"
        )
    };
    mk(
        ".kuda/files/mymod/pkg__a.py.yaml",
        &card("pkg/a.py", "mymod"),
    );
    mk(
        ".kuda/files/mymod/pkg__gone.py.yaml",
        &card("pkg/gone.py", "mymod"),
    );
    mk(
        ".kuda/files/ghostmod/pkg__x.py.yaml",
        &card("pkg/x.py", "ghostmod"),
    );

    let out = kuda(&["sync"], &tmp);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        tmp.join(".kuda/files/mymod/pkg__a.py.yaml").exists(),
        "live card swept"
    );
    assert!(
        !tmp.join(".kuda/files/mymod/pkg__gone.py.yaml").exists(),
        "stale card kept"
    );
    assert!(
        !tmp.join(".kuda/files/ghostmod").exists(),
        "ghost module dir kept"
    );
    std::fs::remove_dir_all(&tmp).ok();
}

/// `kuda status` cross-checks the L0 card's `tree_digest`: a source
/// file added after sync must report tree drift, not just per-card
/// drift.
#[test]
fn status_detects_tree_drift() {
    let tmp = std::env::temp_dir().join(format!("kuda-drift-{}", std::process::id()));
    let mk = |p: &str, text: &str| {
        let f = tmp.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, text).unwrap();
    };
    mk("pyproject.toml", "[project]\nname = \"app\"\n");
    mk("main.py", "def serve():\n    pass\n");

    let out = kuda(&["sync"], &tmp);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(std::fs::read_to_string(tmp.join(".kuda/index.yaml"))
        .unwrap()
        .contains("tree_digest"));

    // clean tree -> no drift lines (summary still says "0 drifted")
    let out = kuda(&["status"], &tmp);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(!stdout.contains("tree drifted"), "{stdout}");
    assert!(!stdout.contains("drifted  "), "{stdout}");

    // un-indexed source file -> digest must change
    mk("extra.py", "def extra():\n    pass\n");
    let out = kuda(&["status"], &tmp);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("tree drifted"), "{stdout}");
    std::fs::remove_dir_all(&tmp).ok();
}

// ---------- helpers for the extended suite ----------

fn tmp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("kuda-e2e-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run kuda with `input` piped to stdin.
fn kuda_stdin(args: &[&str], dir: &std::path::Path, input: &str) -> std::process::Output {
    use std::io::Write;
    let mut c = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    c.wait_with_output().unwrap()
}

fn mk_at(dir: &std::path::Path, p: &str, text: &str) {
    let f = dir.join(p);
    std::fs::create_dir_all(f.parent().unwrap()).unwrap();
    std::fs::write(&f, text).unwrap();
}

fn git(dir: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn git_init(dir: &std::path::Path) {
    git(dir, &["init", "-qb", "main"]);
    git(dir, &["config", "user.email", "t@t"]);
    git(dir, &["config", "user.name", "t"]);
}

/// Minimal python repo — enough for manifest+treesitter to index.
fn seed_py_repo(dir: &std::path::Path) {
    mk_at(
        dir,
        "pyproject.toml",
        "[project]\nname = \"app\"\ndependencies = [\"requests\"]\n",
    );
    mk_at(
        dir,
        "main.py",
        "import requests\n\ndef serve():\n    pass\n",
    );
}

// ---------- map / card / cat_file ----------

#[test]
fn map_and_card_hit_and_miss() {
    let dir = tmp("mapcard");
    seed_py_repo(&dir);
    assert!(kuda(&["sync"], &dir).status.success());
    let out = kuda(&["map"], &dir);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("modules:"));
    let out = kuda(&["card", "app"], &dir);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("name: app"));
    // misses -> exit 1 with stderr
    let out = kuda(&["card", "nope"], &dir);
    assert_eq!(out.status.code(), Some(1));
    // a module name needing `safe()`-ification
    let out = kuda(&["card", "a/b"], &dir);
    assert_eq!(out.status.code(), Some(1));
    let empty = tmp("mapcard-empty");
    assert_eq!(kuda(&["map"], &empty).status.code(), Some(1));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&empty).ok();
}

// ---------- find / dependents / refs ----------

#[test]
fn find_exact_fuzzy_miss() {
    let dir = tmp("find");
    seed_py_repo(&dir);
    assert!(kuda(&["sync"], &dir).status.success());
    // exact
    let out = kuda(&["find", "serve"], &dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("main.py"), "{stdout}");
    // fuzzy tier (substring, tagged)
    let out = kuda(&["find", "serv"], &dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("(fuzzy)"), "{stdout}");
    // miss
    let out = kuda(&["find", "zzz_missing"], &dir);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("no symbol"));
    // --workspace with no workspace.yaml -> falls back to this dir
    let out = kuda(&["--workspace", "find", "serve"], &dir);
    assert!(out.status.success());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn find_workspace_tags_roots() {
    let a = tmp("ws-a");
    let b = tmp("ws-b");
    for d in [&a, &b] {
        seed_py_repo(d);
        assert!(kuda(&["sync"], d).status.success());
    }
    mk_at(&a, ".kuda/workspace.yaml", "roots:\n  - .\n");
    // root list resolved relative to the yaml's dir: use absolute-ish
    // rel paths — sibling root via ../
    mk_at(
        &a,
        ".kuda/workspace.yaml",
        &format!("roots:\n  - {}\n  - {}\n", a.display(), b.display()),
    );
    let out = kuda(&["--workspace", "find", "serve"], &a);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    // hits tagged with their repo basename
    assert!(
        stdout.contains("ws-a") || stdout.contains("ws-b"),
        "{stdout}"
    );
    // --workspace dependents + refs walk every root too
    let out = kuda(&["--workspace", "dependents", "requests"], &a);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = kuda(&["--workspace", "refs", "serve"], &a);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("  ["), "{stdout}");
    std::fs::remove_dir_all(&a).ok();
    std::fs::remove_dir_all(&b).ok();
}

#[test]
fn dependents_all_messages() {
    let dir = tmp("deps");
    seed_py_repo(&dir);
    // no index yet -> store opens empty -> "no dep edges indexed"
    let out = kuda(&["dependents", "requests"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("no dep edges"));
    assert!(kuda(&["sync"], &dir).status.success());
    // edges exist, target found
    let out = kuda(&["dependents", "requests"], &dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("requests"), "{stdout}");
    // edges exist but not for this name
    let out = kuda(&["dependents", "nothing-uses-me"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("nothing depends"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn refs_text_and_docs_tiers() {
    let dir = tmp("refs");
    seed_py_repo(&dir);
    mk_at(&dir, "docs/guide.md", "call serve to start\n");
    mk_at(&dir, "README.md", "# app\nserve does the thing\n");
    mk_at(&dir, "skipme/ignored.md", "serve in excluded dir\n");
    let out = kuda(&["refs", "serve"], &dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("guide.md"), "{stdout}");
    assert!(stdout.contains("main.py"), "{stdout}");
    // --docs only searches doc extensions
    let out = kuda(&["refs", "serve", "--docs"], &dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("guide.md"), "{stdout}");
    assert!(!stdout.contains("main.py"), "{stdout}");
    // no mentions
    let out = kuda(&["refs", "zzz_nothing"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("no mentions"));
    // excluded dirs not walked (target in target/)
    mk_at(&dir, "target/noise.md", "zzz_thing\n");
    let out = kuda(&["refs", "zzz_thing"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("no mentions"));
    // unreadable dir -> read_dir error skipped quietly
    mk_at(&dir, "locked/inner.md", "zzz_secret\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.join("locked"), std::fs::Permissions::from_mode(0o000))
            .unwrap();
        let out = kuda(&["refs", "zzz_secret"], &dir);
        assert!(String::from_utf8_lossy(&out.stdout).contains("no mentions"));
        std::fs::set_permissions(dir.join("locked"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// `refs` semantic tier: lsp kind + stub server returns locations.
#[test]
fn refs_lsp_tier() {
    let dir = tmp("lsprefs");
    seed_py_repo(&dir);
    let server = dir.join("lsp.py");
    // lsp_server is spawned via Command::new — a single argv[0]:
    // executable script with a shebang, not "python3 file".
    std::fs::write(
        &server,
        r#"#!/usr/bin/env python3
import sys, json, os
root = os.environ.get("ROOT","")
def send(m):
    b = json.dumps(m).encode()
    sys.stdout.buffer.write(b"Content-Length: %d\r\n\r\n" % len(b) + b)
    sys.stdout.buffer.flush()
def rd():
    n = 0
    while True:
        l = sys.stdin.buffer.readline()
        if not l: sys.exit(0)
        l = l.strip()
        if not l: break
        if l.startswith(b"Content-Length:"): n = int(l.split(b":",1)[1])
    return json.loads(sys.stdin.buffer.read(n))
while True:
    m = rd(); meth = m.get("method"); mid = m.get("id")
    if meth == "exit": sys.exit(0)
    if mid is None: continue
    if meth == "initialize":
        send({"jsonrpc":"2.0","id":mid,"result":{"capabilities":{}}})
    elif meth == "workspace/symbol":
        if os.environ.get("MODE") == "empty":
            send({"jsonrpc":"2.0","id":mid,"result":[]})
        else:
            send({"jsonrpc":"2.0","id":mid,"result":[
                {"name":"serve","kind":12,"location":{"uri":"file://%s/main.py" % root,
                 "range":{"start":{"line":0,"character":0}}}}]})
    elif meth == "textDocument/references":
        send({"jsonrpc":"2.0","id":mid,"result":[
            {"uri":"file://%s/main.py" % root,"range":{"start":{"line":2,"character":4}}}]})
    elif meth == "shutdown":
        send({"jsonrpc":"2.0","id":mid,"result":None})
    else:
        send({"jsonrpc":"2.0","id":mid,"error":{"code":-32601,"message":"nf"}})
"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&server, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let cfg = format!(
        "[extract]\nkinds = [\"manifest\", \"treesitter\", \"lsp\"]\nlsp_server = \"{}\"\n",
        server.display()
    );
    mk_at(&dir, "kuda.toml", &cfg);
    let out = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(["refs", "serve"])
        .current_dir(&dir)
        .env("ROOT", &dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("main.py:3"), "{stdout}");
    // empty lsp result -> falls through to the text tier
    let out = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(["refs", "serve"])
        .current_dir(&dir)
        .env("ROOT", &dir)
        .env("MODE", "empty")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("main.py"));
    // multi-root workspace -> hits carry the repo tag
    let b = tmp("lsprefs-b");
    seed_py_repo(&b);
    assert!(kuda(&["sync"], &b).status.success());
    mk_at(
        &dir,
        ".kuda/workspace.yaml",
        &format!("roots:\n  - {}\n  - {}\n", dir.display(), b.display()),
    );
    let out = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(["--workspace", "refs", "serve"])
        .current_dir(&dir)
        .env("ROOT", &dir)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("  ["),
        "{:?}",
        String::from_utf8_lossy(&out.stdout)
    );
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&b).ok();
}

/// `refs` lsp tier with a dead server -> error logged, text tier still runs.
#[test]
fn refs_lsp_error_falls_through() {
    let dir = tmp("lspbad");
    seed_py_repo(&dir);
    mk_at(
        &dir,
        "kuda.toml",
        "[extract]\nkinds = [\"manifest\", \"treesitter\", \"lsp\"]\nlsp_server = \"nonexistent-server-xyz\"\n",
    );
    let out = kuda(&["refs", "serve"], &dir);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("lsp refs"));
    assert!(String::from_utf8_lossy(&out.stdout).contains("main.py"));
    std::fs::remove_dir_all(&dir).ok();
}

// ---------- doctor / gen-cli-docs / config errors ----------

#[test]
fn doctor_and_docs_and_bad_config() {
    let dir = tmp("doctor");
    let out = kuda(&["doctor"], &dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("config"), "{stdout}");
    // a valid config prints the ok line
    mk_at(&dir, "kuda.toml", "[extract]\nkinds = [\"manifest\"]\n");
    let out = kuda(&["doctor"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("config   ok"));
    // malformed config -> doctor shows it; other commands warn + default
    mk_at(&dir, "kuda.toml", "not [valid toml");
    let out = kuda(&["doctor"], &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("config   --"));
    let out = kuda(&["find", "x"], &dir);
    assert!(String::from_utf8_lossy(&out.stderr).contains("config:"));
    // docs pipeline
    let out = kuda(&["gen-cli-docs"], &dir);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("kuda"));
    std::fs::remove_dir_all(&dir).ok();
}

/// `--config` flag: root derives from the config's parent dir.
#[test]
fn config_flag_changes_root() {
    let dir = tmp("cfgflag");
    let sub = dir.join("proj");
    std::fs::create_dir_all(&sub).unwrap();
    seed_py_repo(&sub);
    let out = kuda(
        &["--config", &format!("{}/kuda.toml", sub.display()), "sync"],
        &dir,
    );
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(sub.join(".kuda/index.yaml").exists());
    // canonicalize failure keeps the raw path (and still fails cleanly)
    let out = kuda(&["--config", "/nonexistent-xyz/kuda.toml", "map"], &dir);
    assert_eq!(out.status.code(), Some(1));
    std::fs::remove_dir_all(&dir).ok();
}

// ---------- init edge cases ----------

#[test]
fn init_error_paths() {
    let dir = tmp("initerr");
    // .kuda exists as a *file* -> mkdir fails after the toml write
    std::fs::write(dir.join(".kuda"), "x").unwrap();
    let out = kuda(&["init"], &dir);
    assert_eq!(out.status.code(), Some(1));
    // --config into a nonexistent dir -> toml write fails
    let out = kuda(&["--config", "gone/kuda.toml", "init"], &dir);
    assert_eq!(out.status.code(), Some(1));
    std::fs::remove_dir_all(&dir).ok();
}

// ---------- status edge cases ----------

#[test]
fn status_edge_cases() {
    let dir = tmp("status");
    // no .kuda at all -> exit 2
    let out = kuda(&["status"], &dir);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("run `kuda sync`"));
    // .kuda/modules exists but no index.yaml yet -> digest skipped
    seed_py_repo(&dir);
    mk_at(&dir, ".kuda/modules/note.txt", "not a card");
    let out = kuda(&["status"], &dir);
    assert!(out.status.success());
    // a card that won't parse -> skipped
    mk_at(&dir, ".kuda/modules/broken.yaml", "::not a card::");
    // hand-written index without tree_digest -> digest check skipped
    mk_at(&dir, ".kuda/index.yaml", "name: x\nroot: .\npurpose: p\nmodules: []\ncommands: []\nprovenance:\n  files: []\n  hashes: []\n  commit_sha: c\n  generated_at: \"0\"\n  generator: g\n");
    let out = kuda(&["status"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    // garbage index.yaml -> parse fails, skipped silently
    mk_at(&dir, ".kuda/index.yaml", ":::");
    let out = kuda(&["status"], &dir);
    assert!(out.status.success());
    std::fs::remove_dir_all(&dir).ok();
}

// ---------- sync: incremental / upgrades / failures ----------

#[test]
fn sync_incremental_and_deletions() {
    let dir = tmp("incr");
    git_init(&dir);
    seed_py_repo(&dir);
    mk_at(
        &dir,
        "other/pyproject.toml",
        "[project]\nname = \"other\"\n",
    );
    mk_at(&dir, "other/o.py", "def o():\n    pass\n");
    assert!(kuda(&["sync"], &dir).status.success());
    let card1 = std::fs::read_to_string(dir.join(".kuda/modules/app.yaml")).unwrap();
    // no-op re-sync: same bytes -> same card (hash-skip path)
    assert!(kuda(&["sync"], &dir).status.success());
    assert_eq!(
        std::fs::read_to_string(dir.join(".kuda/modules/app.yaml")).unwrap(),
        card1
    );
    // change a file -> module re-summarized
    mk_at(
        &dir,
        "main.py",
        "import requests\n\ndef serve():\n    pass\n\ndef extra():\n    pass\n",
    );
    assert!(kuda(&["sync"], &dir).status.success());
    assert!(std::fs::read_to_string(dir.join(".kuda/modules/app.yaml"))
        .unwrap()
        .contains("extra"));
    // delete a file -> status marks the module drifted (blob_hash
    // misses -> empty string), then sync re-indexes via old_hashes
    std::fs::remove_file(dir.join("main.py")).unwrap();
    let out = kuda(&["status"], &dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("drifted  app"), "{stdout}");
    assert!(kuda(&["sync"], &dir).status.success());
    // delete a whole module -> card + store rows swept
    std::fs::remove_dir_all(dir.join("other")).unwrap();
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!dir.join(".kuda/modules/other.yaml").exists());
    let out = kuda(&["find", "o"], &dir);
    // stale generator stamp -> re-summarize even when bytes match
    mk_at(&dir, "x.py", "def x():\n    pass\n");
    assert!(kuda(&["sync"], &dir).status.success());
    let card = std::fs::read_to_string(dir.join(".kuda/modules/app.yaml")).unwrap();
    let stale = card.replace("kuda-template/0.1.2", "ancient/0.0");
    std::fs::write(dir.join(".kuda/modules/app.yaml"), stale).unwrap();
    assert!(kuda(&["sync"], &dir).status.success());
    assert!(std::fs::read_to_string(dir.join(".kuda/modules/app.yaml"))
        .unwrap()
        .contains("kuda-template/0.1.2"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn sync_full_and_upgrade_seed() {
    let dir = tmp("full");
    seed_py_repo(&dir);
    assert!(kuda(&["sync"], &dir).status.success());
    // lose the sqlite store -> upgrade path seeds file hashes from
    // the committed cards' provenance
    std::fs::remove_file(dir.join(".kuda/index.sqlite")).unwrap();
    // also drop a stale, unparseable card -> seeded rows skip it
    mk_at(&dir, ".kuda/modules/garbage.yaml", ":::");
    mk_at(&dir, ".kuda/modules/unreadable.yaml", "name: x\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            dir.join(".kuda/modules/unreadable.yaml"),
            std::fs::Permissions::from_mode(0o000),
        )
        .unwrap();
    }
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    // --full rebuilds regardless
    let out = kuda(&["sync", "--full"], &dir);
    assert!(out.status.success());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn sync_store_and_cards_failures() {
    let dir = tmp("syncfail");
    seed_py_repo(&dir);
    // store path pointing at a directory -> SqliteStore::open fails
    mk_at(&dir, "kuda.toml", "[store]\npath = \".kuda\"\n");
    std::fs::create_dir_all(dir.join(".kuda")).unwrap();
    let out = kuda(&["sync"], &dir);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("store"));
    // cards dir unmakeable (.kuda is a file) -> mkdir fails
    let dir2 = tmp("syncfail2");
    seed_py_repo(&dir2);
    std::fs::write(dir2.join(".kuda"), "x").unwrap();
    let out = kuda(&["sync"], &dir2);
    assert_eq!(out.status.code(), Some(1));
    // unknown extract kind -> warning, still indexes the rest
    let dir3 = tmp("syncfail3");
    seed_py_repo(&dir3);
    mk_at(
        &dir3,
        "kuda.toml",
        "[extract]\nkinds = [\"manifest\", \"bogus\", \"lsp\"]\n",
    );
    let out = kuda(&["sync"], &dir3);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown kind bogus"));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&dir2).ok();
    std::fs::remove_dir_all(&dir3).ok();
}

#[test]
fn sync_workspace_mode() {
    let a = tmp("wsp-a");
    let b = tmp("wsp-b");
    seed_py_repo(&a);
    seed_py_repo(&b);
    // workspace.yaml with a junk entry (non-string) + real roots
    mk_at(
        &a,
        ".kuda/workspace.yaml",
        &format!("roots:\n  - {}\n  - 42\n  - {}\n", a.display(), b.display()),
    );
    let out = kuda(&["--workspace", "sync"], &a);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(a.join(".kuda/index.yaml").exists());
    assert!(b.join(".kuda/index.yaml").exists());
    // invalid workspace.yaml -> roots [] -> falls back to cwd
    mk_at(&a, ".kuda/workspace.yaml", "\t");
    let out = kuda(&["--workspace", "sync"], &a);
    assert!(out.status.success());
    // yaml without roots key -> empty -> cwd
    mk_at(&a, ".kuda/workspace.yaml", "other: 1\n");
    let out = kuda(&["--workspace", "sync"], &a);
    assert!(out.status.success());
    std::fs::remove_dir_all(&a).ok();
    std::fs::remove_dir_all(&b).ok();
}

// ---------- sturn WAL ----------

#[test]
fn sync_sturn_wal_marks_modules() {
    let dir = tmp("wal");
    git_init(&dir);
    seed_py_repo(&dir);
    assert!(kuda(&["sync"], &dir).status.success());
    // WAL event commits on a sturn ref
    let wal_commit = |body: &str| {
        std::fs::write(dir.join("event.json"), body).unwrap();
        git(&dir, &["add", "event.json"]);
        git(&dir, &["commit", "-qm", "wal"]);
    };
    wal_commit("{\"op\":{\"path\":\"main.py\",\"glob\":\"docs/**\"}}");
    git(&dir, &["rm", "-q", "event.json"]); // removes worktree file too
    git(&dir, &["commit", "-qm", "drop"]); // no event.json -> show fails
    wal_commit("not json at all"); // unparseable -> skipped
    wal_commit("{\"no_op\":true}"); // missing op -> skipped
    wal_commit("{\"op\":{\"from\":\"a.py\",\"to\":\"b.py\",\"glob\":\"*\"}}");
    let head = git(&dir, &["rev-parse", "HEAD"]);
    git(&dir, &["update-ref", "refs/sturn/wal/test", &head]);
    let out = kuda(&["sync"], &dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(stderr.contains("sturn wal:"), "{stderr}");
    // a second WAL round: since-head stored -> `<since>..<ref>` range
    wal_commit("{\"op\":{\"path\":\"main.py\"}}");
    let head = git(&dir, &["rev-parse", "HEAD"]);
    git(&dir, &["update-ref", "refs/sturn/wal/test", &head]);
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::remove_dir_all(&dir).ok();
}

// ---------- acp summarize + file cards ----------

/// ndjson ACP agent stub for `summarize.kind = "acp"`.
const ACP_STUB: &str = r#"
import sys, json, os
mode = os.environ.get("KUDA_STUB_MODE", "ok")
prose = os.environ.get("KUDA_STUB_PROSE", "purpose: agent module prose\nnotes:\n- from agent")
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
    except Exception:
        continue
    m = msg.get("method"); mid = msg.get("id")
    if m == "initialize":
        print(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"protocolVersion":1}}), flush=True)
    elif m == "session/new":
        print(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"sessionId":"s1"}}), flush=True)
    elif m == "session/prompt":
        if mode == "error":
            print(json.dumps({"jsonrpc":"2.0","id":mid,"error":{"code":-1,"message":"boom"}}), flush=True)
            continue
        print(json.dumps({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":prose}}}}), flush=True)
        print(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"stopReason":"end_turn"}}), flush=True)
"#;

fn write_acp_stub(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("agent.py");
    std::fs::write(&p, ACP_STUB).unwrap();
    p
}

#[test]
fn sync_acp_module_and_file_cards() {
    let dir = tmp("acp");
    let agent = write_acp_stub(&dir);
    mk_at(&dir, "pyproject.toml", "[project]\nname = \"app\"\n");
    mk_at(&dir, "main.py", "def serve():\n    pass\n");
    mk_at(&dir, "util.go", "package app\n\nfunc Go() {}\n");
    mk_at(&dir, "lib.rs", "pub fn rust_fn() {}\n");
    mk_at(&dir, "app.ts", "export function tsFn() {}\n");
    mk_at(&dir, "widget.tsx", "export const W = () => null;\n");
    mk_at(&dir, "README.md", "# app\nreal readme purpose\n");
    // slug collision: a__b.py and a/b.py map to the same card name
    mk_at(&dir, "a__b.py", "def coll1():\n    pass\n");
    mk_at(&dir, "a/b.py", "def coll2():\n    pass\n");
    // >200KB file: skipped as not prose-material
    mk_at(&dir, "big.py", &format!("# {}\n", "x".repeat(210_000)));
    // >14KB file: exercises the prompt char budget + "truncated"
    mk_at(
        &dir,
        "medium.py",
        &(0..400)
            .map(|i| format!("def medium_fn_{i}():\n    return {i}\n"))
            .collect::<String>(),
    );
    // unreadable source file -> skipped quietly
    mk_at(&dir, "locked.py", "def locked():\n    pass\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            dir.join("locked.py"),
            std::fs::Permissions::from_mode(0o000),
        )
        .unwrap();
    }
    mk_at(
        &dir,
        "kuda.toml",
        &format!(
            "[extract]\nkinds = [\"manifest\", \"treesitter\"]\n\
             [summarize]\nkind = \"acp\"\nacp_cmd = \"python3 {}\"\nfile_cards = true\n",
            agent.display()
        ),
    );
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("slug collision"));
    // agent prose landed in the module card
    let card = std::fs::read_to_string(dir.join(".kuda/modules/app.yaml")).unwrap();
    assert!(card.contains("agent module prose"), "{card}");
    assert!(card.contains("from agent"), "{card}");
    // L1.5 file cards exist for source files
    assert!(dir.join(".kuda/files/app/main.py.yaml").exists());
    assert!(dir.join(".kuda/files/app/lib.rs.yaml").exists());
    // the >200KB file got no card
    assert!(!dir.join(".kuda/files/app/big.py.yaml").exists());
    // one of the colliding slugs won, the other was skipped
    assert_eq!(
        std::fs::read_dir(dir.join(".kuda/files/app"))
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains("a__b.py")
            })
            .count(),
        1
    );
    // re-sync: file cards skip on matching hash (cheap path)
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    // corrupt one file card -> parse miss re-prompts it
    std::fs::write(dir.join(".kuda/files/app/main.py.yaml"), "::::").unwrap();
    // modify a file -> its card regenerates
    mk_at(
        &dir,
        "main.py",
        "def serve():\n    pass\n\ndef more():\n    pass\n",
    );
    let out = kuda(&["sync"], &dir);
    assert!(out.status.success());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            dir.join("locked.py"),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
    }
    // agent errors on session/prompt -> file card logged + skipped
    let out = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(["sync", "--full"])
        .current_dir(&dir)
        .env("KUDA_STUB_MODE", "error")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stderr).contains("acp file"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn sync_acp_agent_failure_template_fallback() {
    let dir = tmp("acpfail");
    let agent = write_acp_stub(&dir);
    mk_at(&dir, "pyproject.toml", "[project]\nname = \"app\"\n");
    mk_at(&dir, "main.py", "def serve():\n    pass\n");
    mk_at(
        &dir,
        "kuda.toml",
        &format!(
            "[summarize]\nkind = \"acp\"\nacp_cmd = \"python3 {}\"\n",
            agent.display()
        ),
    );
    let out = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(["sync"])
        .current_dir(&dir)
        .env("KUDA_STUB_MODE", "error")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(stderr.contains("template fallback"), "{stderr}");
    // manifest-only pseudo-module skips the agent entirely (acp on,
    // but nothing to ground on)
    let dir2 = tmp("acpmono");
    let agent2 = write_acp_stub(&dir2);
    mk_at(&dir2, "pyproject.toml", "[project]\nname = \"only-man\"\n");
    mk_at(
        &dir2,
        "sub/pyproject.toml",
        "[project]\nname = \"sub\"\ndependencies=[\"x\"]\n",
    );
    mk_at(&dir2, "sub/s.py", "def s():\n    pass\n");
    mk_at(
        &dir2,
        "kuda.toml",
        &format!(
            "[summarize]\nkind = \"acp\"\nacp_cmd = \"python3 {}\"\n",
            agent2.display()
        ),
    );
    let out = kuda(&["sync"], &dir2);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir2.join(".kuda/modules/sub.yaml").exists());
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&dir2).ok();
}

// ---------- mcp / acp stdio servers ----------

#[test]
fn mcp_full_session() {
    let dir = tmp("mcp");
    seed_py_repo(&dir);
    assert!(kuda(&["sync"], &dir).status.success());
    let input = concat!(
        "\n", // empty line skipped
        "not json at all\n", // unparseable skipped
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/ready\"}\n", // no id -> ignored
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"repo_card\",\"arguments\":{}}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"module_card\",\"arguments\":{\"module\":\"app\"}}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"tools/call\",\"params\":{\"name\":\"module_card\",\"arguments\":{\"module\":\"gone\"}}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"tools/call\",\"params\":{\"name\":\"find\",\"arguments\":{\"name\":\"serve\"}}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"tools/call\",\"params\":{\"name\":\"dependents\",\"arguments\":{\"name\":\"requests\"}}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"tools/call\",\"params\":{\"name\":\"bogus\",\"arguments\":{}}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"unknown/method\"}\n",
    );
    let out = kuda_stdin(&["mcp"], &dir, input);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let reply = |id: u32| -> String {
        stdout
            .lines()
            .find(|l| l.contains(&format!("\"id\":{id}")))
            .unwrap_or_else(|| panic!("no reply for id {id}: {stdout}"))
            .to_owned()
    };
    assert!(reply(1).contains("serverInfo"), "{stdout}");
    assert!(reply(2).contains("repo_card"), "{stdout}");
    assert!(reply(2).contains("module_card"), "{stdout}");
    assert!(reply(3).contains("modules:"), "{stdout}");
    assert!(reply(4).contains("app"));
    assert!(reply(5).contains("error"), "{stdout}");
    assert!(reply(6).contains("main.py"), "{stdout}");
    assert!(reply(7).contains("requests"), "{stdout}");
    assert!(reply(8).contains("unknown tool"), "{stdout}");
    // unknown top-level method -> notification ignored, no reply w/ id 9
    std::fs::remove_dir_all(&dir).ok();
}

/// mcp tools when the store can't open -> error text, not a crash.
#[test]
fn mcp_store_error_path() {
    let dir = tmp("mcperr");
    // store.path is a directory -> SqliteStore::open fails
    mk_at(&dir, "kuda.toml", "[store]\npath = \".kuda\"\n");
    std::fs::create_dir_all(dir.join(".kuda")).unwrap();
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"find\",\"arguments\":{\"name\":\"x\"}}}\n";
    let out = kuda_stdin(&["mcp"], &dir, input);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("error"));
    // mcp against a dir with no index at all -> repo_card errs
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"repo_card\",\"arguments\":{}}}\n";
    let out = kuda_stdin(&["mcp"], &dir, input);
    assert!(String::from_utf8_lossy(&out.stdout).contains("error"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn acp_full_session() {
    let dir = tmp("acpstdio"); // distinct tag — `tmp` dirs collide by name
    seed_py_repo(&dir);
    assert!(kuda(&["sync"], &dir).status.success());
    let input = concat!(
        "\n",
        "{bad json\n",
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"session/new\",\"params\":{}}\n",
        // prompt: text + non-text blocks; verbs find/deps/card/unknown
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"session/prompt\",\"params\":{\"sessionId\":\"s\",\"prompt\":[{\"type\":\"text\",\"text\":\"find serve\"},{\"type\":\"image\"}]}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"session/prompt\",\"params\":{\"prompt\":[{\"type\":\"text\",\"text\":\"deps requests\"}]}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"session/prompt\",\"params\":{\"prompt\":[{\"type\":\"text\",\"text\":\"card app\"}]}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"session/prompt\",\"params\":{\"prompt\":[{\"type\":\"text\",\"text\":\"card gone\"}]}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"session/prompt\",\"params\":{\"prompt\":[{\"type\":\"text\",\"text\":\"map\"}]}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"session/cancel\",\"params\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"authenticated\",\"params\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":10,\"method\":\"mystery/method\",\"params\":{}}\n",
    );
    let out = kuda_stdin(&["acp"], &dir, input);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let reply = |id: u32| -> String {
        stdout
            .lines()
            .find(|l| l.contains(&format!("\"id\":{id}")))
            .unwrap_or_else(|| panic!("no reply for id {id}: {stdout}"))
            .to_owned()
    };
    assert!(reply(1).contains("agentInfo"), "{stdout}");
    assert!(reply(2).contains("kuda-default"), "{stdout}");
    assert!(stdout.contains("session/update"), "{stdout}");
    assert!(stdout.contains("main.py"), "{stdout}");
    assert!(stdout.contains("requests"), "{stdout}");
    // 'card gone' -> error text in the chunk
    assert!(stdout.contains("error:"), "{stdout}");
    // unknown method -> -32601
    assert!(reply(10).contains("-32601"), "{stdout}");
    std::fs::remove_dir_all(&dir).ok();
}

/// acp_answer with a broken store -> "error:" chunk text.
#[test]
fn acp_store_error_path() {
    let dir = tmp("acperr");
    mk_at(&dir, "kuda.toml", "[store]\npath = \".kuda\"\n");
    std::fs::create_dir_all(dir.join(".kuda")).unwrap();
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"session/prompt\",\"params\":{\"prompt\":[{\"type\":\"text\",\"text\":\"find x\"}]}}\n";
    let out = kuda_stdin(&["acp"], &dir, input);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("error:"));
    // 'deps' verb hits the same broken store
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"session/prompt\",\"params\":{\"prompt\":[{\"type\":\"text\",\"text\":\"deps x\"}]}}\n";
    let out = kuda_stdin(&["acp"], &dir, input);
    assert!(String::from_utf8_lossy(&out.stdout).contains("error:"));
    std::fs::remove_dir_all(&dir).ok();
}

// ---------- detect_commands breadth ----------

#[test]
fn sync_detect_commands_all_sources() {
    let dir = tmp("cmds");
    // rust + go + npm + python + make + just + task
    mk_at(
        &dir,
        "Cargo.toml",
        "[package]\nname = \"ruster\"\ndescription = \"rust pkg desc\"\n",
    );
    mk_at(
        &dir,
        "src/lib.rs",
        "//! crate doc purpose\n\npub fn f() {}\n",
    );
    mk_at(
        &dir,
        "goapp/go.mod",
        "module github.com/x/goapp\n\nrequire (\n\tdep v1.0\n)\n",
    );
    mk_at(&dir, "goapp/main.go", "package main\n\nfunc main() {}\n");
    mk_at(
        &dir,
        "web/package.json",
        "{\"name\":\"web\",\"description\":\"web desc\",\"scripts\":{\"test\":\"vitest\",\"build\":\"tsc\",\"custom\":\"x\"}}",
    );
    mk_at(&dir, "web/index.ts", "export function f() {}\n");
    mk_at(&dir, "badjson/package.json", "{oops");
    mk_at(&dir, "noscripts/package.json", "{\"name\":\"noscripts\"}");
    mk_at(&dir, "py/pyproject.toml", "[project]\nname = \"pymod\"\n");
    mk_at(&dir, "py/tests/test_x.py", "def test_x():\n    pass\n");
    mk_at(&dir, "Makefile", "build:\n\tcc\ntest: build\n\tt\nVAR := x\n%pat: %in\n\tp\n# comment\nno-colon-line\nbuild:\n\tagain\n");
    mk_at(&dir, "goapp/Makefile", "gen:\n\tgo generate\n");
    mk_at(&dir, "justfile", "x:\n\techo\n");
    mk_at(&dir, "Taskfile.yml", "version: '3'\n");
    // unreadable dir + unreadable package.json -> skipped quietly
    mk_at(&dir, "locked/x.py", "def x():\n    pass\n");
    mk_at(&dir, "unreadable/package.json", "{\"name\":\"u\"}");
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let index = std::fs::read_to_string(dir.join(".kuda/index.yaml")).unwrap();
    // commands are capped at 8 — the npm/pytest pushes still run
    // but land past the cap; check what fits.
    for want in [
        "make build",
        "make -C goapp gen",
        "just --list",
        "task --list",
        "cargo check",
        "go test ./...",
    ] {
        assert!(index.contains(want), "missing {want} in:\n{index}");
    }
    // purpose fallback: Cargo.toml description wins for ruster
    let card = std::fs::read_to_string(dir.join(".kuda/modules/ruster.yaml")).unwrap();
    assert!(card.contains("rust pkg desc"), "{card}");
    // crate doc comment for the module without description
    let card = std::fs::read_to_string(dir.join(".kuda/modules/web.yaml")).unwrap();
    assert!(card.contains("web desc"), "{card}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.join("locked"), std::fs::Permissions::from_mode(0o000))
            .unwrap();
        std::fs::set_permissions(
            dir.join("unreadable/package.json"),
            std::fs::Permissions::from_mode(0o000),
        )
        .unwrap();
    }
    let _ = kuda(&["sync"], &dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.join("locked"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        std::fs::set_permissions(
            dir.join("unreadable/package.json"),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
    }
    // a second fixture where npm + pytest fit under the cap
    let dir2 = tmp("cmds2");
    mk_at(
        &dir2,
        "web/package.json",
        "{\"name\":\"only\",\"scripts\":{\"test\":\"vitest\",\"zzz\":\"x\"}}",
    );
    mk_at(&dir2, "web/index.ts", "export function f() {}\n");
    mk_at(&dir2, "py/pyproject.toml", "[project]\nname = \"pymod\"\n");
    mk_at(&dir2, "py/tests/test_x.py", "def test_x():\n    pass\n");
    let out = kuda(&["sync"], &dir2);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let index = std::fs::read_to_string(dir2.join(".kuda/index.yaml")).unwrap();
    assert!(index.contains("npm run test"), "{index}");
    assert!(index.contains("(cd web && npm run"), "{index}");
    assert!(index.contains("pytest"), "{index}");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&dir2).ok();
}

/// module_purpose fallbacks: crate `//!` doc + module README.
#[test]
fn sync_purpose_fallback_chain() {
    let dir = tmp("purpose");
    // lib.rs //! doc (no manifest description)
    mk_at(&dir, "a/Cargo.toml", "[package]\nname = \"withdoc\"\n");
    mk_at(
        &dir,
        "a/src/lib.rs",
        "//! from crate docs\n\npub fn f() {}\n",
    );
    // main.rs //! doc
    mk_at(&dir, "b/Cargo.toml", "[package]\nname = \"withmain\"\n");
    mk_at(&dir, "b/src/main.rs", "//! from main docs\nfn main() {}\n");
    // module README first non-heading line
    mk_at(&dir, "c/package.json", "{\"name\":\"withreadme\"}");
    mk_at(&dir, "c/README.md", "# Title\n\nreadme purpose line\n");
    mk_at(&dir, "c/index.js", "function f() {}\n");
    // README with only headings -> falls to root README
    mk_at(&dir, "d/package.json", "{\"name\":\"headonly\"}");
    mk_at(&dir, "d/README.md", "# only a heading\n");
    mk_at(&dir, "d/index.js", "function f() {}\n");
    mk_at(&dir, "README.md", "# repo\nroot readme purpose\n");
    // no purpose source anywhere
    mk_at(&dir, "e/package.json", "{\"name\":\"nopurpose\"}");
    mk_at(&dir, "e/index.js", "function f() {}\n");
    // src/lib.rs with real code first line -> crate_doc None
    mk_at(&dir, "f/Cargo.toml", "[package]\nname = \"nodoc\"\n");
    mk_at(&dir, "f/src/lib.rs", "pub fn f() {}\n");
    // src/lib.rs all comments -> crate_doc falls through to None
    mk_at(&dir, "h/Cargo.toml", "[package]\nname = \"onlycomments\"\n");
    mk_at(&dir, "h/src/lib.rs", "// just a comment\n// another\n");
    // go.mod manifest -> manifest_purpose None arm
    mk_at(&dir, "g/go.mod", "module x\n");
    mk_at(&dir, "g/x.go", "package x\n");
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let read =
        |m: &str| std::fs::read_to_string(dir.join(format!(".kuda/modules/{m}.yaml"))).unwrap();
    assert!(read("withdoc").contains("from crate docs"));
    assert!(read("withmain").contains("from main docs"));
    assert!(read("withreadme").contains("readme purpose line"));
    assert!(read("headonly").contains("root readme purpose"));
    // every fallback fails -> root README is the last resort
    assert!(read("nopurpose").contains("root readme purpose"));
    std::fs::remove_dir_all(&dir).ok();
}

/// `verify` list cap: >10 files sorts entry points first.
#[test]
fn sync_verify_cap_prefers_entry_points() {
    let dir = tmp("verifycap");
    mk_at(&dir, "pyproject.toml", "[project]\nname = \"bigmod\"\n");
    for i in 0..12 {
        mk_at(&dir, &format!("f{i:02}.py"), "def f():\n    pass\n");
    }
    mk_at(&dir, "__init__.py", "def init():\n    pass\n");
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let card = std::fs::read_to_string(dir.join(".kuda/modules/bigmod.yaml")).unwrap();
    let verify_count = card
        .lines()
        .skip_while(|l| !l.starts_with("verify:"))
        .skip(1)
        .take_while(|l| l.starts_with('-'))
        .count();
    assert_eq!(verify_count, 10, "{card}");
    assert!(card.contains("__init__.py"), "{card}");
    std::fs::remove_dir_all(&dir).ok();
}

/// watch: fs change triggers an incremental sync.
#[test]
fn watch_syncs_on_change() {
    let dir = tmp("watch");
    mk_at(&dir, "pyproject.toml", "[project]\nname = \"app\"\n");
    mk_at(&dir, "a.py", "def a():\n    pass\n");
    let mut child = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(["watch"])
        .current_dir(&dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    // give the watcher a beat to register, then trigger an event
    std::thread::sleep(std::time::Duration::from_millis(800));
    mk_at(&dir, "b.py", "def b():\n    pass\n");
    // also write inside .kuda — must be filtered out
    std::fs::create_dir_all(dir.join(".kuda")).ok();
    std::fs::write(dir.join(".kuda/noise.txt"), "x").ok();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !dir.join(".kuda/modules/app.yaml").exists() {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("watch never synced: {:?}", child.wait_with_output());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    let _ = child.kill();
    let _ = child.wait();
    std::fs::remove_dir_all(&dir).ok();
}

/// PATH with no git -> every git spawn fails quietly.
#[test]
fn sync_without_git_on_path() {
    let dir = tmp("nogitpath");
    seed_py_repo(&dir);
    let out = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(["sync"])
        .current_dir(&dir)
        .env("PATH", "/nonexistent-xyz")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// stdout write failing with non-BrokenPipe errors just stops.
#[cfg(unix)]
#[test]
fn write_error_not_broken_pipe() {
    let dir = tmp("devfull");
    seed_py_repo(&dir);
    assert!(kuda(&["sync"], &dir).status.success());
    let full = std::fs::File::options()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kuda"))
        .args(["map"])
        .current_dir(&dir)
        .stdout(std::process::Stdio::from(full))
        .output()
        .unwrap();
    // emit() swallows non-BrokenPipe errors; process still exits 0
    assert!(out.status.success());
    std::fs::remove_dir_all(&dir).ok();
}

/// `kuda map` on the real repo keeps dogfooding honest.
#[test]
fn init_then_status_without_git() {
    let dir = tmp("nogit");
    seed_py_repo(&dir);
    assert!(kuda(&["sync"], &dir).status.success());
    // not a git repo -> head_sha empty; sturn for-each-ref fails quietly
    let out = kuda(&["status"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Coverage builds stub out `Cmd::Watch` (the daemon loop can't be
/// observed once the process must be killed) — the arm still runs.
#[cfg(coverage)]
#[test]
fn watch_arm_dispatches() {
    let dir = tmp("watcharm");
    let out = kuda(&["watch"], &dir);
    assert!(out.status.success());
    std::fs::remove_dir_all(&dir).ok();
}

/// A package whose own sources import its dist name by the import
/// spelling (`import self_pkg` inside self-pkg) must not produce a
/// `self-pkg --Imports--> self-pkg` edge — filtered at index time.
#[test]
fn dependents_skips_self_import_edges() {
    let dir = tmp("selfedge");
    mk_at(
        &dir,
        "shared/pyproject.toml",
        "[project]\nname = \"self-pkg\"\n",
    );
    mk_at(
        &dir,
        "shared/self_pkg/__init__.py",
        "import self_pkg\n\ndef helper():\n    pass\n",
    );
    mk_at(
        &dir,
        "user/pyproject.toml",
        "[project]\nname = \"user\"\ndependencies = [\"self-pkg\"]\n",
    );
    mk_at(
        &dir,
        "user/main.py",
        "import self_pkg\n\ndef run():\n    pass\n",
    );
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = kuda(&["dependents", "self-pkg"], &dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("user"), "{stdout}");
    // no self-edge may surface — nothing may *come from* self-pkg
    assert!(!stdout.contains("self-pkg --"), "{stdout}");
    std::fs::remove_dir_all(&dir).ok();
}

/// ACP-written cards must stamp provenance with the adapter that
/// produced the prose — `kuda-acp/…`, not `kuda-template/…`.
#[test]
fn sync_acp_stamps_kuda_acp_generator() {
    let dir = tmp("acp-gen");
    let agent = write_acp_stub(&dir);
    mk_at(&dir, "pyproject.toml", "[project]\nname = \"app\"\n");
    mk_at(&dir, "main.py", "def serve():\n    pass\n");
    mk_at(
        &dir,
        "kuda.toml",
        &format!(
            "[summarize]\nkind = \"acp\"\nacp_cmd = \"python3 {}\"\n",
            agent.display()
        ),
    );
    let out = kuda(&["sync"], &dir);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let card = std::fs::read_to_string(dir.join(".kuda/modules/app.yaml")).unwrap();
    assert!(card.contains("generator: kuda-acp/"), "{card}");
    assert!(!card.contains("kuda-template"), "{card}");
    assert!(card.contains("agent module prose"), "{card}");
    std::fs::remove_dir_all(&dir).ok();
}
