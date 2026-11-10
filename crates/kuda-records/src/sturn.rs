//! `sturn-wal` domain source — the first non-tree corpus. sturn
//! keeps its coordination log in git refs: one commit per event on
//! `refs/sturn/wal/<ws>` (compaction archives at
//! `refs/sturn/archive/<ws>`), each commit's tree holding a single
//! `event.json`. Each ref is indexed as a virtual JSONL file — one
//! event per line, named by the ref — so the ordinary
//! record/edge scan applies unchanged, same-line edge scoping
//! included. No refs, not a repo, no git binary: silent skip, same
//! as an absent corpus dir.

use crate::{scan_file, Compiled};
use kuda_core::DomainDef;
use kuda_extract::Extraction;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// One `git` call under `root` — stdout on success, `None` on any
/// failure. Ref-corpus reads are best-effort by contract: a repo
/// without sturn refs isn't an extraction error.
fn git(root: &Path, args: &[String]) -> Option<String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Refs under `prefix` + their head shas — the source's input set.
/// Sync folds these into the module's "files" with the head sha as
/// the content hash: a moved ref diffs like a write, a deleted ref
/// like a removal — incremental sync needs nothing new.
pub fn inputs(root: &Path, prefix: &str) -> Vec<(PathBuf, String)> {
    let Some(out) = git(
        root,
        &[
            "for-each-ref".into(),
            "--format=%(refname) %(objectname)".into(),
            prefix.into(),
        ],
    ) else {
        return Vec::new();
    };
    out.lines()
        .filter_map(|l| {
            let (r, sha) = l.trim().split_once(' ')?;
            (!r.is_empty() && !sha.is_empty()).then(|| (PathBuf::from(r), sha.to_owned()))
        })
        .collect()
}

/// Commits on a ref, oldest first — the event sequence.
fn commits(root: &Path, refname: &str) -> Vec<String> {
    git(
        root,
        &["rev-list".into(), "--reverse".into(), refname.into()],
    )
    .map(|s| {
        s.lines()
            .map(|l| l.trim().to_owned())
            .filter(|l| !l.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

/// A commit's `event.json` — `None` for non-event commits (the ref
/// namespace is sturn's, but a foreign commit isn't a WAL event:
/// skipped, not warned).
fn event(root: &Path, sha: &str) -> Option<serde_json::Value> {
    let text = git(
        root,
        &["cat-file".into(), "-p".into(), format!("{sha}:event.json")],
    )?;
    serde_json::from_str(&text).ok()
}

/// Index every ref under `prefix`: the chain's parseable events
/// become one virtual JSONL text scanned by [`scan_file`], so
/// `Symbol.file` = the ref name and `Symbol.line` = the event's
/// offset in the chain. Two fields are injected before scanning —
/// `seq` (1-based entry offset) and `record`, the queryable id
/// `<actor>:<op>@<ws>#<id8>` (`id8` = event blob/commit hash —
/// stable across compaction, unlike `seq`). An empty `id` (events
/// written before id assignment) falls back to the commit sha.
pub fn extract(
    root: &Path,
    prefix: &str,
    def: &DomainDef,
    compiled: &Compiled,
    out: &mut Extraction,
) {
    for (refname, _head) in inputs(root, prefix) {
        let r = refname.display().to_string();
        let mut lines = Vec::new();
        for sha in commits(root, &r) {
            let Some(mut ev) = event(root, &sha) else {
                continue;
            };
            let Some(o) = ev.as_object_mut() else {
                continue;
            };
            let field = |k: &str| o.get(k).and_then(|v| v.as_str()).unwrap_or("").to_owned();
            let (actor, ws, id) = (field("actor"), field("ws"), field("id"));
            let id = if id.is_empty() { sha.clone() } else { id };
            let op = o
                .get("op")
                .and_then(|op| op.get("op"))
                .and_then(|v| v.as_str())
                .unwrap_or("event")
                .to_owned();
            let seq = lines.len() + 1;
            let record = format!(
                "{actor}:{op}@{ws}#{}",
                id.chars().take(8).collect::<String>()
            );
            o.insert("id".into(), serde_json::json!(id));
            o.insert("seq".into(), serde_json::json!(seq));
            o.insert("record".into(), serde_json::json!(record));
            lines.push(ev.to_string());
        }
        if lines.is_empty() {
            continue;
        }
        scan_file(&refname, &lines.join("\n"), def, compiled, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecordsExtractor;
    use kuda_core::{EdgeKind, Source, SymbolKind};
    use kuda_extract::Extractor;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kuda-wal-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sh(root: &Path, args: &[&str]) -> String {
        let o = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(o.status.success(), "git {args:?}: {:?}", o.stderr);
        String::from_utf8_lossy(&o.stdout).trim().to_owned()
    }

    /// Build a repo with a `refs/sturn/wal/main` event chain — same
    /// shape sturn writes: blob + tree(event.json) + commit + ref.
    fn wal_repo(tag: &str) -> PathBuf {
        let root = tmp(tag);
        sh(&root, &["init", "-q"]);
        sh(&root, &["config", "user.email", "t@t"]);
        sh(&root, &["config", "user.name", "t"]);
        let mut parent: Option<String> = None;
        for ev in [
            r#"{"id":"","ws":"main","actor":"luis","ts":1,"op":{"op":"claim","glob":"crates/*","ttl_secs":60}}"#,
            r#"{"id":"","ws":"main","actor":"devin","ts":2,"op":{"op":"write","path":"src/a.rs","blob":"b1"}}"#,
        ] {
            let f = root.join("ev.json");
            std::fs::write(&f, ev).unwrap();
            let blob = sh(&root, &["hash-object", "-w", "ev.json"]);
            // mktree needs stdin — write via a scratch index instead
            sh(
                &root,
                &[
                    "update-index",
                    "--add",
                    "--cacheinfo",
                    &format!("100644,{blob},event.json"),
                ],
            );
            let tree = sh(&root, &["write-tree"]);
            sh(&root, &["read-tree", "--empty"]);
            let mut args = vec!["commit-tree", &tree, "-m", "sturn:main:event"];
            if let Some(p) = &parent {
                args.extend(["-p", p]);
            }
            let commit = sh(&root, &args);
            sh(&root, &["update-ref", "refs/sturn/wal/main", &commit]);
            parent = Some(commit);
        }
        let _ = std::fs::remove_file(root.join("ev.json"));
        root
    }

    fn sturn_def() -> DomainDef {
        DomainDef {
            name: "sturn".into(),
            roots: vec![],
            extensions: vec![],
            source: Some(Source::SturnWal {
                ref_prefix: "refs/sturn".into(),
            }),
            module: "sturn-wal".into(),
            purpose: None,
            record_patterns: vec!["\"record\":\\s*\"([^\"]+)\"".into()],
            ref_pattern: Some("\"(?:path|glob|actor|ws)\":\\s*\"([^\"]+)\"".into()),
            schema_json: None,
            file_records: false,
        }
    }

    #[test]
    fn wal_events_become_records_and_edges() {
        let root = wal_repo("scan");
        let x = RecordsExtractor {
            excludes: vec![],
            domains: vec![sturn_def()],
        };
        let out = x.extract(&root, &[]).unwrap();
        let names: Vec<&str> = out.symbols.iter().map(|s| s.name.as_str()).collect();
        // one record per event — `luis:claim@main#<id8>` shape
        assert_eq!(out.symbols.len(), 2, "{names:?}");
        assert!(names.iter().any(|n| n.starts_with("luis:claim@main#")));
        assert!(names.iter().any(|n| n.starts_with("devin:write@main#")));
        assert!(out.symbols.iter().all(|s| s.kind == SymbolKind::Record
            && s.file == Path::new("refs/sturn/wal/main")
            && s.module == "sturn-wal"));
        // same-line edge scoping: the claim event edges to its glob,
        // the write event to its path — no cross-wiring
        let claim = names
            .iter()
            .find(|n| n.starts_with("luis:"))
            .unwrap()
            .to_string();
        let write = names
            .iter()
            .find(|n| n.starts_with("devin:"))
            .unwrap()
            .to_string();
        assert!(out
            .edges
            .iter()
            .any(|e| e.from == claim && e.to == "crates/*" && e.kind == EdgeKind::References));
        assert!(out
            .edges
            .iter()
            .any(|e| e.from == write && e.to == "src/a.rs"));
        assert!(!out
            .edges
            .iter()
            .any(|e| e.from == claim && e.to == "src/a.rs"));

        // source inputs: the ref + its head sha
        let inputs = crate::source_inputs(&root, &sturn_def());
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].0, Path::new("refs/sturn/wal/main"));
        assert_eq!(inputs[0].1.len(), 40);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn no_refs_is_silent_skip() {
        let root = tmp("bare");
        sh(&root, &["init", "-q"]);
        let x = RecordsExtractor {
            excludes: vec![],
            domains: vec![sturn_def()],
        };
        let out = x.extract(&root, &[]).unwrap();
        assert!(out.symbols.is_empty() && out.edges.is_empty());
        assert!(crate::source_inputs(&root, &sturn_def()).is_empty());
        // and on a non-repo entirely — no .git at all
        let plain = tmp("plain");
        let out = x.extract(&plain, &[]).unwrap();
        assert!(out.symbols.is_empty());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&plain);
    }
}
