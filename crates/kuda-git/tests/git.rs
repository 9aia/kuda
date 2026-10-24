use kuda_git::GitVcs;
use kuda_vcs::Vcs;
use std::process::Command;

fn repo() -> (std::path::PathBuf, String) {
    let d = std::env::temp_dir().join(format!("kuda-repo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    Command::new("git")
        .args(["init", "-b", "main"])
        .arg(&d)
        .output()
        .unwrap();
    std::fs::write(d.join("f"), "x").unwrap();
    for a in [
        vec!["add", "."],
        vec![
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            "c",
        ],
    ] {
        Command::new("git")
            .args(&a)
            .current_dir(&d)
            .output()
            .unwrap();
    }
    let sha = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&d)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    (d, sha)
}

#[test]
fn sync_has_commit_worktree_tag() {
    let (r, sha) = repo();
    let v = GitVcs;
    assert_eq!(v.head_sha(&r).as_deref(), Some(sha.as_str()));
    assert!(v.has_commit(&r, &sha));
    assert!(!v.has_commit(&r, &"0".repeat(40)));
    assert!(!v.tag_exists(&r, "t1"));
    assert!(v.tag(&r, "t1"));
    assert!(v.tag_exists(&r, "t1"));
    let wt = std::env::temp_dir().join(format!("kuda-wt-{}", std::process::id()));
    assert!(v.worktree_add(&r, &wt, &sha));
    assert!(wt.join("f").exists());
    assert!(v.worktree_remove(&r, &wt));
    assert!(!wt.exists());
    // failure paths: no remote, non-repo, bogus sha — all false/None
    assert!(!v.sync(&r));
    assert!(!v.push_tag(&r, "t1"));
    let notrepo = std::env::temp_dir().join(format!("kuda-norepo-{}", std::process::id()));
    std::fs::create_dir_all(&notrepo).unwrap();
    assert_eq!(v.head_sha(&notrepo), None);
    assert!(!v.worktree_add(&notrepo, &notrepo.join("w"), &sha));
    assert!(!v.worktree_remove(&r, &notrepo.join("gone")));
    assert!(v.worktree_prune(&r));
    // git missing from PATH -> spawn fails -> None
    let orig = std::env::var("PATH").unwrap_or_default();
    let empty = std::env::temp_dir().join(format!("kuda-nopath-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    unsafe {
        std::env::set_var("PATH", &empty);
    }
    assert_eq!(v.head_sha(&r), None);
    assert!(!v.has_commit(&r, &sha));
    unsafe {
        std::env::set_var("PATH", orig);
    }
    let _ = std::fs::remove_dir_all(&r);
    let _ = std::fs::remove_dir_all(&notrepo);
    let _ = std::fs::remove_dir_all(&empty);
}
