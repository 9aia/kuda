//! Sqlite adapter of `IndexStore` — single file, zero server,
//! rebuildable. Lives at `.kuda/index.sqlite` (gitignored).

use kuda_core::{DepEdge, EdgeKind, Symbol, SymbolKind};
use kuda_store::IndexStore;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// `Connection` is `!Sync` — the mutex keeps the store behind the
/// port's `Send + Sync` bound. Single-writer anyway.
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS symbols(
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    file TEXT NOT NULL,
    line INTEGER NOT NULL,
    module TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_module ON symbols(module);
CREATE TABLE IF NOT EXISTS edges(
    from_mod TEXT NOT NULL,
    to_mod TEXT NOT NULL,
    kind TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_mod);
CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS files(
    path TEXT PRIMARY KEY,
    hash TEXT NOT NULL,
    module TEXT NOT NULL
);
";

fn sk_str(k: SymbolKind) -> &'static str {
    match k {
        SymbolKind::Function => "function",
        SymbolKind::Type => "type",
        SymbolKind::Trait => "trait",
        SymbolKind::Const => "const",
        SymbolKind::Module => "module",
        SymbolKind::Macro => "macro",
        SymbolKind::Other => "other",
    }
}

fn sk_from(s: &str) -> SymbolKind {
    match s {
        "function" => SymbolKind::Function,
        "type" => SymbolKind::Type,
        "trait" => SymbolKind::Trait,
        "const" => SymbolKind::Const,
        "module" => SymbolKind::Module,
        "macro" => SymbolKind::Macro,
        _ => SymbolKind::Other,
    }
}

fn ek_str(k: EdgeKind) -> &'static str {
    match k {
        EdgeKind::Imports => "imports",
        EdgeKind::Calls => "calls",
        EdgeKind::PackageDep => "package-dep",
        EdgeKind::WorkspaceMember => "workspace-member",
    }
}

fn ek_from(s: &str) -> EdgeKind {
    match s {
        "imports" => EdgeKind::Imports,
        "calls" => EdgeKind::Calls,
        "workspace-member" => EdgeKind::WorkspaceMember,
        _ => EdgeKind::PackageDep,
    }
}

impl SqliteStore {
    /// Open (creating) the store at `path`; parents are made.
    pub fn open(path: &Path) -> Result<Self, String> {
        #[cfg(not(coverage))]
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        // parent() is None only for "/"-like paths — never a store path
        #[cfg(coverage)]
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new("")))
            .map_err(|e| e.to_string())?;
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn.lock().map_err(|e| e.to_string())
    }
}

impl IndexStore for SqliteStore {
    fn replace_module(
        &mut self,
        module: &str,
        syms: &[Symbol],
        edges: &[DepEdge],
    ) -> Result<(), String> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM symbols WHERE module = ?1", params![module])
            .and_then(|_| tx.execute("DELETE FROM edges WHERE from_mod = ?1", params![module]))
            .map_err(|e| e.to_string())?;
        for s in syms {
            tx.execute(
                "INSERT INTO symbols(name,kind,file,line,module) VALUES(?1,?2,?3,?4,?5)",
                params![
                    s.name,
                    sk_str(s.kind),
                    s.file.to_string_lossy(),
                    s.line,
                    s.module
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        for e in edges {
            tx.execute(
                "INSERT INTO edges(from_mod,to_mod,kind) VALUES(?1,?2,?3)",
                params![e.from, e.to, ek_str(e.kind)],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    fn file_hashes(&self) -> Result<std::collections::HashMap<String, (String, String)>, String> {
        let conn = self.lock()?;
        let mut st = conn
            .prepare("SELECT path,hash,module FROM files")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = std::collections::HashMap::new();
        for r in rows {
            let (p, h, m) = r.map_err(|e| e.to_string())?;
            out.insert(p, (h, m));
        }
        Ok(out)
    }

    fn record_files(&mut self, module: &str, files: &[(String, String)]) -> Result<(), String> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM files WHERE module = ?1", params![module])
            .map_err(|e| e.to_string())?;
        for (p, h) in files {
            tx.execute(
                "INSERT INTO files(path,hash,module) VALUES(?1,?2,?3)",
                params![p, h, module],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    fn find(&self, name: &str) -> Result<Vec<Symbol>, String> {
        let conn = self.lock()?;
        let mut st = conn
            .prepare("SELECT name,kind,file,line,module FROM symbols WHERE name = ?1")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map(params![name], |r| {
                Ok(Symbol {
                    name: r.get(0)?,
                    kind: sk_from(&r.get::<_, String>(1)?),
                    file: r.get::<_, String>(2)?.into(),
                    line: r.get(3)?,
                    module: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    fn search(&self, pattern: &str) -> Result<Vec<Symbol>, String> {
        let conn = self.lock()?;
        let mut st = conn
            .prepare("SELECT name,kind,file,line,module FROM symbols WHERE name LIKE ?1 ESCAPE '\\' ORDER BY name")
            .map_err(|e| e.to_string())?;
        // escape LIKE metachars so user text stays literal
        let esc = pattern
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let rows = st
            .query_map(params![format!("%{esc}%")], |r| {
                Ok(Symbol {
                    name: r.get(0)?,
                    kind: sk_from(&r.get::<_, String>(1)?),
                    file: r.get::<_, String>(2)?.into(),
                    line: r.get(3)?,
                    module: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    fn dependents(&self, name: &str) -> Result<Vec<DepEdge>, String> {
        let conn = self.lock()?;
        let mut st = conn
            .prepare("SELECT from_mod,to_mod,kind FROM edges WHERE to_mod = ?1")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map(params![name], |r| {
                Ok(DepEdge {
                    from: r.get(0)?,
                    to: r.get(1)?,
                    kind: ek_from(&r.get::<_, String>(2)?),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    fn edge_count(&self) -> Result<usize, String> {
        self.lock()?
            .query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }

    fn modules(&self) -> Result<Vec<String>, String> {
        let conn = self.lock()?;
        let mut st = conn
            .prepare("SELECT DISTINCT module FROM symbols ORDER BY module")
            .map_err(|e| e.to_string())?;
        let rows = st.query_map([], |r| r.get(0)).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    fn set_meta(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.lock()?
            .execute(
                "INSERT INTO meta(k,v) VALUES(?1,?2) ON CONFLICT(k) DO UPDATE SET v=?2",
                params![key, value],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn meta(&self, key: &str) -> Result<Option<String>, String> {
        self.lock()?
            .query_row("SELECT v FROM meta WHERE k = ?1", params![key], |r| {
                r.get(0)
            })
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(e.to_string()),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kuda-sql-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sym(name: &str, kind: SymbolKind, module: &str) -> Symbol {
        Symbol {
            name: name.into(),
            kind,
            file: "f".into(),
            line: 1,
            module: module.into(),
        }
    }

    #[test]
    fn all_kinds_roundtrip() {
        let mut s = SqliteStore::open(&tmp("k").join("i.db")).unwrap();
        let syms: Vec<Symbol> = [
            ("f", SymbolKind::Function),
            ("t", SymbolKind::Type),
            ("tr", SymbolKind::Trait),
            ("c", SymbolKind::Const),
            ("m", SymbolKind::Module),
            ("mac", SymbolKind::Macro),
            ("o", SymbolKind::Other),
        ]
        .iter()
        .map(|(n, k)| sym(n, *k, "mod"))
        .collect();
        let edges: Vec<DepEdge> = [
            EdgeKind::Imports,
            EdgeKind::Calls,
            EdgeKind::PackageDep,
            EdgeKind::WorkspaceMember,
        ]
        .iter()
        .enumerate()
        .map(|(i, k)| DepEdge {
            from: "mod".into(),
            to: format!("t{i}"),
            kind: *k,
        })
        .collect();
        s.replace_module("mod", &syms, &edges).unwrap();
        for (n, k) in [
            ("f", SymbolKind::Function),
            ("t", SymbolKind::Type),
            ("tr", SymbolKind::Trait),
            ("c", SymbolKind::Const),
            ("m", SymbolKind::Module),
            ("mac", SymbolKind::Macro),
            ("o", SymbolKind::Other),
        ] {
            assert_eq!(s.find(n).unwrap()[0].kind, k, "{n}");
        }
        // corrupt kind strings exercise the fallback arms
        {
            let conn = s.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO symbols(name,kind,file,line,module) VALUES('x','bogus','f',1,'mod')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO edges(from_mod,to_mod,kind) VALUES('mod','zz','bogus-kind')",
                [],
            )
            .unwrap();
        }
        assert_eq!(s.find("x").unwrap()[0].kind, SymbolKind::Other);
        assert_eq!(s.dependents("zz").unwrap()[0].kind, EdgeKind::PackageDep);
    }

    #[test]
    fn open_error_paths() {
        let root = tmp("e");
        // parent is a file -> create_dir_all fails
        let file = root.join("afile");
        std::fs::write(&file, "x").unwrap();
        assert!(SqliteStore::open(&file.join("db.sqlite")).is_err());
        // path is a directory -> Connection::open fails
        std::fs::create_dir_all(root.join("dpath")).unwrap();
        assert!(SqliteStore::open(&root.join("dpath")).is_err());
        // garbage bytes -> schema batch fails on a non-sqlite file
        let bad = root.join("bad.db");
        std::fs::write(&bad, b"this is not sqlite at all not even close").unwrap();
        assert!(SqliteStore::open(&bad).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn query_error_paths() {
        let root = tmp("q");
        let db = root.join("i.db");
        let mut s = SqliteStore::open(&db).unwrap();
        s.replace_module("m", &[sym("a", SymbolKind::Function, "m")], &[])
            .unwrap();
        s.record_files("m", &[("a".into(), "h".into())]).unwrap();
        // drop tables under the store's feet -> every read/write errs
        {
            let conn = s.conn.lock().unwrap();
            conn.execute_batch("DROP TABLE symbols; DROP TABLE edges; DROP TABLE files; DROP TABLE meta;")
                .unwrap();
        }
        assert!(s.find("a").is_err());
        assert!(s.search("a").is_err());
        assert!(s.dependents("a").is_err());
        assert!(s.edge_count().is_err());
        assert!(s.modules().is_err());
        assert!(s.file_hashes().is_err());
        assert!(s.meta("k").is_err());
        assert!(s.set_meta("k", "v").is_err());
        assert!(s.replace_module("m", &[], &[]).is_err());
        assert!(s.record_files("m", &[]).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn search_escapes_like_metachars() {
        let mut s = SqliteStore::open(&tmp("l").join("i.db")).unwrap();
        s.replace_module(
            "m",
            &[
                sym("100%", SymbolKind::Function, "m"),
                sym("a_b", SymbolKind::Function, "m"),
                sym("axb", SymbolKind::Function, "m"),
            ],
            &[],
        )
        .unwrap();
        // literal % and _ must not act as wildcards
        assert_eq!(s.search("100%").unwrap().len(), 1);
        assert_eq!(s.search("a_b").unwrap().len(), 1);
        assert_eq!(s.search("a\\b").unwrap().len(), 0);
    }
}
