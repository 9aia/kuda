//! Sqlite impl of `IndexStore` — single file, zero server,
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
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
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
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
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
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    fn modules(&self) -> Result<Vec<String>, String> {
        let conn = self.lock()?;
        let mut st = conn
            .prepare("SELECT DISTINCT module FROM symbols ORDER BY module")
            .map_err(|e| e.to_string())?;
        let rows = st.query_map([], |r| r.get(0)).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
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
            .query_row("SELECT v FROM meta WHERE k = ?1", params![key], |r| r.get(0))
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(e.to_string()),
            })
    }
}
