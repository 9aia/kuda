//! Records adapter — non-code record corpora -> record symbols +
//! reference edges. Domain knowledge lives in `DomainDef` data
//! (data/domains/*): the regexes are the format. Frontmatter
//! records (`id = "T-001"`) and checklist records (`- [ ] T-001`)
//! are the same mechanism — a per-line capture-group-1 scan.

use kuda_core::{DepEdge, DomainDef, EdgeKind, LangDef, Symbol, SymbolKind};
use kuda_extract::{Extraction, Extractor};
use std::collections::HashSet;
use std::path::Path;

/// `excludes`: extra dir names from `[extract] exclude`.
/// `domains`: descriptors loaded from `data/domains/*` — a corpus
/// that doesn't exist on disk is a silent skip, not an error.
pub struct RecordsExtractor {
    pub excludes: Vec<String>,
    pub domains: Vec<DomainDef>,
}

impl Extractor for RecordsExtractor {
    fn kind(&self) -> &'static str {
        "records"
    }

    fn extract(&self, root: &Path, _langs: &[LangDef]) -> Result<Extraction, String> {
        let mut out = Extraction::default();
        for def in &self.domains {
            let compiled = compile(def)?;
            for base in &def.roots {
                walk_corpus(root, base, def, &compiled, &self.excludes, &mut out);
            }
        }
        Ok(out)
    }
}

struct Compiled {
    records: Vec<regex::Regex>,
    refs: Option<regex::Regex>,
}

fn compile(def: &DomainDef) -> Result<Compiled, String> {
    let records = def
        .record_patterns
        .iter()
        .map(|p| {
            regex::Regex::new(p)
                .map_err(|e| format!("domain {}: bad record_pattern {p:?}: {e}", def.name))
        })
        .collect::<Result<_, _>>()?;
    let refs = def
        .ref_pattern
        .as_ref()
        .map(|p| {
            regex::Regex::new(p)
                .map_err(|e| format!("domain {}: bad ref_pattern {p:?}: {e}", def.name))
        })
        .transpose()?;
    Ok(Compiled { records, refs })
}

/// Recursive walk of one declared corpus root. Unlike the shared
/// repo walk, a `.git`-file worktree marker does NOT stop the
/// descent — declaring the root is the intent (myco's `project/`
/// is exactly such a worktree).
fn walk_corpus(
    root: &Path,
    base: &Path,
    def: &DomainDef,
    compiled: &Compiled,
    excludes: &[String],
    out: &mut Extraction,
) {
    let mut stack = vec![root.join(base)];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if !e
                    .file_name()
                    .to_str()
                    .map(|n| kuda_core::dir_excluded(n, excludes))
                    .unwrap_or(true)
                {
                    stack.push(p);
                }
                continue;
            }
            let ext_ok = p
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| {
                    def.extensions
                        .iter()
                        .any(|de| de.trim_start_matches('.') == x)
                })
                .unwrap_or(false);
            if !ext_ok {
                continue;
            }
            let rel = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
            let Ok(text) = std::fs::read_to_string(&p) else {
                continue;
            };
            scan_file(&rel, &text, def, compiled, out);
        }
    }
}

/// One file -> record symbols + reference edges. Record scope:
/// single-record files take edges from the whole text; multi-record
/// files (checklists) scope each record's refs to its own line —
/// otherwise every task would edge to every mentioned id.
fn scan_file(rel: &Path, text: &str, def: &DomainDef, compiled: &Compiled, out: &mut Extraction) {
    // (id, 1-based line) — first match wins per id per file.
    // Fenced code blocks are skipped: `# comment` inside a toml
    // sample is not an H1 record.
    let mut seen: HashSet<String> = HashSet::new();
    let mut records: Vec<(String, u32)> = Vec::new();
    let mut fenced = false;
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        for re in &compiled.records {
            if let Some(c) = re.captures(line) {
                if let Some(m) = c.get(1) {
                    let id = m.as_str().to_owned();
                    if seen.insert(id.clone()) {
                        records.push((id, i as u32 + 1));
                    }
                }
                break; // one pattern per line is enough
            }
        }
    }
    if records.is_empty() {
        return;
    }
    for (id, line) in &records {
        out.symbols.push(Symbol {
            name: id.clone(),
            kind: SymbolKind::Record,
            file: rel.to_path_buf(),
            line: *line,
            module: def.module.clone(),
        });
    }
    if let Some(re) = &compiled.refs {
        let multi = records.len() > 1;
        for (id, line) in &records {
            // scope: own line for multi-record files, whole text else
            let hay: &str = if multi {
                text.lines().nth(*line as usize - 1).unwrap_or("")
            } else {
                text
            };
            for c in re.captures_iter(hay) {
                // capture group 1 if the pattern has one (e.g.
                // "`(ident)`" -> ident), else the whole match
                let to = c.get(1).or(c.get(0)).unwrap().as_str().to_owned();
                out.edges.push(DepEdge {
                    from: id.clone(),
                    to,
                    kind: EdgeKind::References,
                });
            }
        }
    }
    out.files.push(rel.to_path_buf());
}

#[cfg(test)]
mod tests {
    use super::*;
    use kuda_extract::Extractor;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("kuda-rec-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn myco() -> DomainDef {
        DomainDef {
            name: "myco".into(),
            roots: vec!["project".into()],
            extensions: vec![".md".into()],
            module: "project".into(),
            purpose: None,
            record_patterns: vec![
                "(?:id|item)\\s*=\\s*\"([A-Z]+-[0-9]+)\"".into(),
                "-\\s*\\[[ xX]\\]\\s*([A-Z]+-[0-9]+)".into(),
            ],
            ref_pattern: Some("\\b[A-Z]+-[0-9]+\\b".into()),
        }
    }

    #[test]
    fn extracts_frontmatter_and_checklist_records() {
        let root = tmp("scan");
        let mk = |p: &str, text: &str| {
            let f = root.join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(&f, text).unwrap();
        };
        mk(
            "project/adrs/0001-adr.md",
            "+++\nid = \"ADR-0001\"\n+++\n\nBody mentions T-002 and ADR-0001.\n",
        );
        mk(
            "project/TODO.md",
            "# TODO\n\n- [ ] T-001 first task\n- [ ] T-002 blocked on T-001\n- [x] T-003 done\n",
        );
        mk(
            "project/handoffs/T-009.md",
            "+++\nitem = \"T-009\"\n+++\nsee T-002\n",
        );
        // non-record files and wrong extensions ignored
        mk("project/index.md", "# index\n");
        mk("project/meta.toml", "name = \"x\"\n");
        // outside the corpus root entirely
        mk("other/T-099.md", "- [ ] T-099 nope\n");
        // corpus root that is a git worktree still walks
        mk("project/.git", "gitdir: ../.git/worktrees/project");

        let x = RecordsExtractor {
            excludes: vec![],
            domains: vec![myco()],
        };
        let out = x.extract(&root, &[]).unwrap();
        assert_eq!(x.kind(), "records");

        let names: Vec<&str> = out.symbols.iter().map(|s| s.name.as_str()).collect();
        for want in ["ADR-0001", "T-001", "T-002", "T-003", "T-009"] {
            assert!(names.contains(&want), "missing {want}: {names:?}");
        }
        assert!(!names.contains(&"T-099"));
        assert!(out.symbols.iter().all(|s| s.kind == SymbolKind::Record));
        assert!(out.symbols.iter().all(|s| s.module == "project"));
        // record pins the line its id first appears on
        let t1 = out.symbols.iter().find(|s| s.name == "T-001").unwrap();
        assert_eq!(t1.line, 3);

        // single-record file: whole-text edge scope
        assert!(out
            .edges
            .iter()
            .any(|e| e.from == "ADR-0001" && e.to == "T-002"));
        // self-edge emitted here; sync's retain drops it downstream
        // multi-record file: same-line scope only
        assert!(out
            .edges
            .iter()
            .any(|e| e.from == "T-002" && e.to == "T-001"));
        assert!(!out
            .edges
            .iter()
            .any(|e| e.from == "T-001" && e.to == "T-002"));
        assert!(out.edges.iter().all(|e| e.kind == EdgeKind::References));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ref_pattern_prefers_capture_group_one() {
        let root = tmp("grp");
        let f = root.join("project/page.md");
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, "# Page One\n\ncall `kuda-records` for this\n").unwrap();
        let mut def = myco();
        def.record_patterns = vec!["^#\\s+(.+)$".into()];
        def.ref_pattern = Some("`([a-z][a-z0-9_-]+)`".into());
        let x = RecordsExtractor {
            excludes: vec![],
            domains: vec![def],
        };
        let out = x.extract(&root, &[]).unwrap();
        // edge target is the group's contents, not the backticked match
        assert!(out
            .edges
            .iter()
            .any(|e| e.from == "Page One" && e.to == "kuda-records"));
        assert!(!out.edges.iter().any(|e| e.to.contains('`')));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn absent_root_and_bad_regex() {
        let root = tmp("absent");
        let x = RecordsExtractor {
            excludes: vec![],
            domains: vec![myco()],
        };
        let out = x.extract(&root, &[]).unwrap(); // silent skip
        assert!(out.symbols.is_empty() && out.edges.is_empty());

        let mut bad = myco();
        bad.record_patterns = vec!["([".into()];
        let x = RecordsExtractor {
            excludes: vec![],
            domains: vec![bad],
        };
        assert!(x.extract(&root, &[]).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
