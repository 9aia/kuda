//! kuda — a hippocampus for your codebase. Composition root:
//! parses kuda.toml, wires adapters behind ports, runs subcommands.

use clap::{Parser, Subcommand};
use kuda_config::ConfigSource;
use kuda_config_toml::TomlSource;
use kuda_core::{Config, LangDef, Provenance, RepoCard, Symbol};
use kuda_vcs::Vcs;
use kuda_extract::Extractor;
use kuda_store::IndexStore;
use kuda_summarize::Summarizer;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};


const EXAMPLE_TOML: &str = include_str!("../../../examples/kuda.toml.example");
const LANG_TOMLS: &[&str] = &[
    include_str!("../../../data/langs/rust.toml"),
    include_str!("../../../data/langs/typescript.toml"),
    include_str!("../../../data/langs/python.toml"),
    include_str!("../../../data/langs/go.toml"),
];

#[derive(Parser)]
#[command(
    name = "kuda",
    version,
    about = "A hippocampus for your codebase — repo memory: index, query, verify"
)]
struct Cli {
    /// Config path (else $KUDA_CONFIG, ./kuda.toml,
    /// ~/.config/kuda/kuda.toml).
    #[arg(long, global = true)]
    config: Option<String>,

    /// Workspace mode — all roots in .kuda/workspace.yaml.
    #[arg(long, global = true)]
    workspace: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Detect languages -> kuda.toml + .kuda/ scaffold.
    Init,
    /// Incremental index (hash -> extract -> summarize -> store).
    Sync {
        /// Rebuild everything, ignoring content-hash freshness.
        #[arg(long)]
        full: bool,
    },
    /// Provenance vs HEAD: per-card drift report.
    Status,
    /// Render the L0 repo card (the always-load blob).
    Map,
    /// Module card + freshness.
    Card { module: String },
    /// Symbol -> file:line.
    Find { symbol: String },
    /// Reverse dep lookup — impact analysis.
    Dependents { name: String },
    /// Code + docs/comments mentions — the doc-sweep query.
    Refs {
        name: String,
        /// Only docs/comments/markdown mentions.
        #[arg(long)]
        docs: bool,
    },
    /// Serve the index over MCP (stdio).
    Mcp,
    /// Speak ACP — Agent Client Protocol surface for any agent
    /// client (zed, cline, cursor).
    Acp,
    /// Extractor/store/config health.
    Doctor,
    /// Emit CLI reference markdown (docs pipeline).
    #[command(hide = true, name = "gen-cli-docs")]
    GenCliDocs,
}

fn main() {
    let cli = Cli::parse();
    let code = match &cli.cmd {
        Cmd::Init => init(&cli),
        Cmd::Sync { full } => sync(&cli, *full),
        Cmd::Status => status(&cli),
        Cmd::Map => map(&cli),
        Cmd::Card { module } => card(&cli, module),
        Cmd::Find { symbol } => find(&cli, symbol),
        Cmd::Dependents { name } => dependents(&cli, name),
        Cmd::Refs { name, docs } => refs(&cli, name, *docs),
        Cmd::Doctor => doctor(&cli),
        Cmd::Mcp => mcp(&cli),
        Cmd::Acp => acp(&cli),
        Cmd::GenCliDocs => {
            println!("{}", clap_markdown::help_markdown::<Cli>());
            0
        }
    };
    std::process::exit(code);
}

fn root(cli: &Cli) -> PathBuf {
    let p = cli
        .config
        .as_deref()
        .map(Path::new)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| ".".into());
    std::fs::canonicalize(&p).unwrap_or(p)
}

fn load_config(cli: &Cli) -> Config {
    TomlSource {
        path: cli.config.clone(),
    }
    .load()
    .unwrap_or_else(|e| {
        eprintln!("config: {e} — defaults in use");
        Config::default()
    })
}

fn langs() -> Vec<LangDef> {
    LANG_TOMLS
        .iter()
        .filter_map(|t| toml::from_str(t).ok())
        .collect()
}

/// git blob hash — provenance key. In-process sha1
/// ("blob <len>\0" + bytes): identical values to
/// `git hash-object`, no subprocess per file.
fn hash_files(root: &Path, files: &[PathBuf]) -> Vec<String> {
    files.iter().map(|f| blob_hash(&root.join(f))).collect()
}

fn blob_hash(path: &Path) -> String {
    use sha1::Digest;
    let Ok(bytes) = std::fs::read(path) else {
        return String::new();
    };
    let mut h = sha1::Sha1::new();
    h.update(format!("blob {}\0", bytes.len()).as_bytes());
    h.update(&bytes);
    h.finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn unix_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

fn init(cli: &Cli) -> i32 {
    let dir = root(cli);
    let toml_path = dir.join("kuda.toml");
    if toml_path.exists() {
        eprintln!("kuda.toml already exists — not overwriting");
        return 1;
    }
    if let Err(e) = std::fs::write(&toml_path, EXAMPLE_TOML) {
        eprintln!("write {}: {e}", toml_path.display());
        return 1;
    }
    for d in [".kuda", ".kuda/modules"] {
        if let Err(e) = std::fs::create_dir_all(dir.join(d)) {
            eprintln!("mkdir {d}: {e}");
            return 1;
        }
    }
    println!("wrote {} + .kuda/ — run `kuda sync`", toml_path.display());
    0
}

/// Workspace roots from `.kuda/workspace.yaml` (`roots: [...]`),
/// resolved relative to the file's dir.
fn workspace_roots(dir: &Path) -> Vec<PathBuf> {
    let p = dir.join(".kuda/workspace.yaml");
    let Ok(text) = std::fs::read_to_string(&p) else {
        return vec![];
    };
    let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(&text) else {
        return vec![];
    };
    v["roots"]
        .as_sequence()
        .map(|s| {
            s.iter()
                .filter_map(|r| r.as_str())
                .map(|r| {
                    let p = dir.join(r);
                    std::fs::canonicalize(&p).unwrap_or(p)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The pipeline: extract -> per-module summarize -> cards + store.
fn sync(cli: &Cli, _full: bool) -> i32 {
    let dir = root(cli);
    let cfg = load_config(cli);
    let roots = if cli.workspace {
        workspace_roots(&dir)
    } else {
        vec![dir.clone()]
    };
    let mut code = 0;
    for r in if roots.is_empty() { vec![dir.clone()] } else { roots } {
        println!("sync {}", r.display());
        code |= sync_one(&r, &cfg, cli);
    }
    code
}

fn sync_one(dir: &Path, cfg: &Config, _cli: &Cli) -> i32 {
    let langs = langs();
    let manifest_list: Vec<String> = langs
        .iter()
        .flat_map(|l| l.manifests.iter().cloned())
        .collect();
    let manifest_names: std::collections::HashSet<String> =
        manifest_list.iter().cloned().collect();
    let src_exts: std::collections::HashSet<String> = langs
        .iter()
        .flat_map(|l| {
            l.extensions
                .iter()
                .map(|e| e.trim_start_matches('.').to_owned())
        })
        .collect();

    // --- manifest pass first (cheap): package names decide module
    // identity — walk groups by manifest dir -> pkg name, matching
    // the Module symbols the manifest extractor emits.
    let manifest_ext = kuda_manifest::ManifestExtractor {
        excludes: cfg.extract.exclude.clone(),
    }
    .extract(dir, &langs)
    .unwrap_or_default();
    // manifest parent dir (repo-relative) -> package name
    let mut pkg_by_dir: std::collections::HashMap<PathBuf, String> =
        std::collections::HashMap::new();
    for s in &manifest_ext.symbols {
        if let Some(d) = s.file.parent() {
            pkg_by_dir.insert(d.to_path_buf(), s.module.clone());
        }
    }

    // --- walk once: group manifests + source files by module ---
    // Cheap (no parse): drives both the incremental diff and the
    // per-module file lists.
    let mut mod_files: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    {
        let mut stack = vec![dir.to_path_buf()];
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
                        .map(|n| kuda_core::dir_excluded(n, &cfg.extract.exclude))
                        .unwrap_or(true)
                        && !p.join(".git").is_file()
                    {
                        stack.push(p);
                    }
                    continue;
                }
                let name = e.file_name().to_string_lossy().into_owned();
                let is_manifest = manifest_names.contains(name.as_str());
                let is_src = p
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| src_exts.contains(x))
                    .unwrap_or(false);
                if !is_manifest && !is_src {
                    continue;
                }
                let rel = p.strip_prefix(dir).unwrap_or(&p).to_path_buf();
                let module = kuda_treesitter::manifest_dir_of(dir, &p, &manifest_list)
                    .map(|d| {
                        let rel_dir = d.strip_prefix(dir).unwrap_or(&d).to_path_buf();
                        pkg_by_dir.get(&rel_dir).cloned().unwrap_or_else(|| {
                            d.file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_else(|| "_root".into())
                        })
                    })
                    .unwrap_or_else(|| "_root".into());
                mod_files.entry(module).or_default().push(rel);
            }
        }
        for v in mod_files.values_mut() {
            v.sort();
        }
    }
    // in-process blob hashes for every walked file
    let filehash: std::collections::HashMap<PathBuf, String> = mod_files
        .values()
        .flatten()
        .map(|f| (f.clone(), blob_hash(&dir.join(f))))
        .collect();

    let cards_dir = dir.join(&cfg.cards.dir);
    if let Err(e) = std::fs::create_dir_all(cards_dir.join("modules")) {
        eprintln!("mkdir {}: {e}", cards_dir.display());
        return 1;
    }
    let mut store = match kuda_store_sqlite::SqliteStore::open(&dir.join(&cfg.store.path)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("store {}: {e}", cfg.store.path.display());
            return 1;
        }
    };
    // upgrade path: no file-hash rows yet -> seed them from the
    // committed cards' provenance so nothing re-parses (or
    // re-prompts an agent) on the first post-upgrade sync.
    let mut old_hashes = store.file_hashes().unwrap_or_default();
    if old_hashes.is_empty() {
        if let Ok(rd) = std::fs::read_dir(cards_dir.join("modules")) {
            for e in rd.flatten() {
                let Ok(text) = std::fs::read_to_string(e.path()) else {
                    continue;
                };
                let Ok(card) = kuda_cards::parse_module_card(&text) else {
                    continue;
                };
                let rows: Vec<(String, String)> = card
                    .provenance
                    .files
                    .iter()
                    .zip(&card.provenance.hashes)
                    .map(|(f, h)| (f.display().to_string(), h.clone()))
                    .collect();
                let _ = store.record_files(&card.name, &rows);
            }
        }
        old_hashes = store.file_hashes().unwrap_or_default();
    }

    // diff: which modules changed?
    let mut changed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (module, files) in &mod_files {
        for f in files {
            match old_hashes.get(&f.display().to_string()) {
                Some((h, _)) if h == &filehash[f] => {}
                _ => {
                    changed.insert(module.clone());
                    break;
                }
            }
        }
        // no card on disk -> force (store may outlive .kuda/modules)
        let card_path = cards_dir
            .join("modules")
            .join(format!("{}.yaml", safe(module)));
        if !card_path.exists() {
            changed.insert(module.clone());
        }
    }
    for (path, (_, module)) in &old_hashes {
        if !filehash.contains_key(&PathBuf::from(path)) {
            changed.insert(module.clone()); // deleted file
        }
    }
    let parse_set: std::collections::HashSet<PathBuf> = changed
        .iter()
        .flat_map(|m| mod_files.get(m).cloned().unwrap_or_default())
        .collect();

    let mut extractors: Vec<Box<dyn Extractor>> = Vec::new();
    for k in &cfg.extract.kinds {
        match k.as_str() {
            "manifest" => extractors.push(Box::new(kuda_manifest::ManifestExtractor {
                excludes: cfg.extract.exclude.clone(),
            })),
            "treesitter" => extractors.push(Box::new(kuda_treesitter::TreesitterExtractor {
                excludes: cfg.extract.exclude.clone(),
                pkg_names: pkg_by_dir.clone(),
            })),
            "lsp" => extractors.push(Box::new(kuda_lsp::LspExtractor {
                server_cmd: cfg.extract.lsp_server.clone(),
            })),
            other => eprintln!("extract: unknown kind {other} (adapter crate?)"),
        }
    }
    let mut ext = kuda_extract::Extraction::default();
    for x in &extractors {
        // manifest is cheap (few files) — full pass keeps module
        // symbols + edges complete; treesitter honors the filter.
        let filtered = x.kind() == "treesitter" && !parse_set.is_empty();
        let res = if filtered {
            x.extract_filtered(&dir, &langs, &parse_set)
        } else {
            x.extract(&dir, &langs)
        };
        match res {
            Ok(e) => {
                ext.symbols.extend(e.symbols);
                ext.edges.extend(e.edges);
                ext.files.extend(e.files);
            }
            Err(e) => eprintln!("extract {}: {e}", x.kind()),
        }
    }
    let sha = kuda_git::GitVcs
        .head_sha(dir)
        .unwrap_or_else(|| "unknown".into());
    let now = unix_now();

    // group symbols + edges by module (filtered pass: changed only)
    let mut by_mod: BTreeMap<String, Vec<kuda_core::Symbol>> = BTreeMap::new();
    for s in &ext.symbols {
        by_mod.entry(s.module.clone()).or_default().push(s.clone());
    }

    // summarizer adapter: acp spawns an agent; anything else is template
    let acp = cfg
        .summarize
        .acp_cmd
        .clone()
        .map(|cmd| kuda_acp::AcpSummarizer::new(cmd, dir.to_path_buf()));
    let use_acp = cfg.summarize.kind == "acp" && acp.is_some();
    let summ = kuda_template::TemplateSummarizer;
    let mut module_names = Vec::new();
    // modules that vanished from the walk entirely -> drop card +
    // store rows
    let present: std::collections::HashSet<&String> = mod_files.keys().collect();
    if let Ok(old_mods) = store.modules() {
        for m in old_mods {
            if !present.contains(&m) {
                let _ = store.replace_module(&m, &[], &[]);
                let _ = store.record_files(&m, &[]);
                let stale = cards_dir.join("modules").join(format!("{}.yaml", safe(&m)));
                let _ = std::fs::remove_file(stale);
            }
        }
    }
    for (module, mfiles) in &mod_files {
        if !changed.contains(module) {
            module_names.push(module.clone());
            continue; // store + card already agree with the bytes
        }
        let prov = Provenance {
            hashes: mfiles
                .iter()
                .map(|f| filehash.get(f).cloned().unwrap_or_default())
                .collect(),
            files: mfiles.clone(),
            commit_sha: sha.clone(),
            generated_at: now.clone(),
            generator: "kuda-template/0.1.0".into(),
        };
        let empty: Vec<kuda_core::Symbol> = Vec::new();
        let syms: &[kuda_core::Symbol] =
            by_mod.get(module).map(|v| v.as_slice()).unwrap_or(&empty);
        let mod_edges: Vec<_> = ext
            .edges
            .iter()
            .filter(|e| e.from == *module)
            .cloned()
            .collect();
        // L1.5 file cards — per-source-file chunked summaries.
        // Per-file hash-skip makes the steady state free; only
        // changed files spend an agent prompt.
        if use_acp && cfg.summarize.file_cards {
            if let Some(acp) = acp.as_ref() {
                sync_file_cards(
                    dir,
                    module,
                    syms,
                    &mfiles,
                    &prov.hashes,
                    &manifest_names,
                    acp,
                    &cards_dir.join("files"),
                    &sha,
                    &now,
                );
            }
        }
        // module `path` for the card: its manifest if present,
        // else first input file
        let path = mfiles
            .iter()
            .find(|f| {
                f.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| manifest_names.contains(n))
                    .unwrap_or(false)
            })
            .cloned()
            .or_else(|| mfiles.first().cloned())
            .unwrap_or_else(|| PathBuf::from(module));
        // manifest-only pseudo-modules (root Cargo.toml, docs/
        // package.json) have no source to ground on — the agent
        // would return empty prose. Skip the prompt entirely.
        let only_manifests = !mfiles.is_empty()
            && mfiles.iter().all(|f| {
                f.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| manifest_names.contains(n))
                    .unwrap_or(false)
            });
        let card_res = if use_acp && !only_manifests {
            acp.as_ref()
                .unwrap()
                .summarize(
                    module,
                    &path,
                    &kuda_extract::Extraction {
                        symbols: syms.to_vec(),
                        edges: mod_edges.clone(),
                        files: mfiles.clone(),
                    },
                    prov.clone(),
                )
                .or_else(|e| {
                    eprintln!("acp {module}: {e} — template fallback");
                    summ.summarize(
                        module,
                        &path,
                        &kuda_extract::Extraction {
                            symbols: syms.to_vec(),
                            edges: mod_edges.clone(),
                            files: mfiles.clone(),
                        },
                        prov.clone(),
                    )
                })
        } else {
            summ.summarize(
                module,
                &path,
                &kuda_extract::Extraction {
                    symbols: syms.to_vec(),
                    edges: mod_edges.clone(),
                    files: mfiles.clone(),
                },
                prov.clone(),
            )
        };
        let mut card = match card_res {
            Ok(c) => c,
            Err(e) => {
                eprintln!("summarize {module}: {e}");
                continue;
            }
        };
        // purpose: manifest description -> crate docs -> README
        if card.purpose == card.name {
            if let Some(d) = module_purpose(dir, &dir.join(&path)) {
                card.purpose = d;
            }
        }
        let out = cards_dir.join("modules").join(format!("{}.yaml", safe(&card.name)));
        match kuda_cards::render_module_card(&card) {
            Ok(y) => {
                if let Err(e) = std::fs::write(&out, y) {
                    eprintln!("write {}: {e}", out.display());
                }
            }
            Err(e) => eprintln!("render {module}: {e}"),
        }
        if let Err(e) = store.replace_module(module, syms, &mod_edges) {
            eprintln!("store {module}: {e}");
        }
        let rows: Vec<(String, String)> = mfiles
            .iter()
            .zip(&prov.hashes)
            .map(|(f, h)| (f.display().to_string(), h.clone()))
            .collect();
        if let Err(e) = store.record_files(module, &rows) {
            eprintln!("record_files {module}: {e}");
        }
        module_names.push(card.name);
    }
    module_names.sort();

    // L0 repo card
    let name = cfg
        .repo
        .name
        .clone()
        .or_else(|| dir.file_name().map(|f| f.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "repo".into());
    let prov = Provenance {
        files: ext.files.clone(),
        hashes: hash_files(&dir, &ext.files),
        commit_sha: sha.clone(),
        generated_at: now.clone(),
        generator: "kuda/0.1.0".into(),
    };
    let repo_card = RepoCard {
        name,
        root: dir.to_path_buf(),
        purpose: readme_purpose(&dir.join("README.md")).unwrap_or_default(),
        modules: module_names.clone(),
        commands: detect_commands(&dir),
        provenance: prov,
    };
    match kuda_cards::render_repo_card(&repo_card) {
        Ok(y) => {
            if let Err(e) = std::fs::write(cards_dir.join("index.yaml"), y) {
                eprintln!("write index.yaml: {e}");
            }
        }
        Err(e) => eprintln!("render index: {e}"),
    }
    let _ = store.set_meta("commit_sha", &sha);
    let _ = store.set_meta("generated_at", &now);
    let _ = std::fs::write(
        cards_dir.join("meta.json"),
        format!("{{\"commit_sha\":\"{sha}\",\"generated_at\":{now}}}\n"),
    );
    println!("indexed {} module(s) at {sha:.7}", module_names.len());
    0
}

/// L1.5 sync — one card per source file. Chunk with text-splitter
/// (code-aware via tree-sitter where a grammar exists), prompt the
/// shared ACP session, write `.kuda/files/<module>/<slug>.yaml`.
/// Per-file provenance hash gates: unchanged files cost nothing.
fn sync_file_cards(
    dir: &Path,
    module: &str,
    syms: &[Symbol],
    mfiles: &[PathBuf],
    hashes: &[String],
    manifest_names: &std::collections::HashSet<String>,
    acp: &kuda_acp::AcpSummarizer,
    files_dir: &Path,
    sha: &str,
    now: &str,
) {
    use text_splitter::{ChunkConfig, CodeSplitter, MarkdownSplitter, TextSplitter};
    const CHUNK: usize = 1_500; // chars per chunk
    const BUDGET: usize = 14_000; // total source chars per prompt
    const SKIP_BYTES: u64 = 200_000; // files bigger than this: not prose-material
    for (rel, hash) in mfiles.iter().zip(hashes) {
        if rel
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| manifest_names.contains(n))
            .unwrap_or(false)
        {
            continue; // manifests are card inputs, not file cards
        }
        let slug = rel.display().to_string().replace('/', "__");
        let out = files_dir
            .join(safe(module))
            .join(format!("{slug}.yaml"));
        if out.exists() {
            if let Ok(old) = std::fs::read_to_string(&out)
                .map_err(|e| e.to_string())
                .and_then(|t| kuda_cards::parse_file_card(&t))
            {
                if old.provenance.hashes.first() == Some(hash) {
                    continue;
                }
            }
        }
        let abs = dir.join(rel);
        let Ok(text) = std::fs::read_to_string(&abs) else {
            continue;
        };
        if abs.metadata().map(|m| m.len()).unwrap_or(0) > SKIP_BYTES {
            continue;
        }
        let cfg = ChunkConfig::new(CHUNK);
        let chunks: Vec<String> = match rel.extension().and_then(|e| e.to_str()) {
            Some("rs") => CodeSplitter::new(tree_sitter_rust::LANGUAGE, cfg)
                .map(|s| s.chunk_indices(&text).map(|(_, c)| c.to_string()).collect())
                .unwrap_or_else(|_| vec![text.clone()]),
            Some("ts") | Some("tsx") => {
                CodeSplitter::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT, cfg)
                    .map(|s| s.chunk_indices(&text).map(|(_, c)| c.to_string()).collect())
                    .unwrap_or_else(|_| vec![text.clone()])
            }
            Some("py") => CodeSplitter::new(tree_sitter_python::LANGUAGE, cfg)
                .map(|s| s.chunk_indices(&text).map(|(_, c)| c.to_string()).collect())
                .unwrap_or_else(|_| vec![text.clone()]),
            Some("md") | Some("mdx") => MarkdownSplitter::new(cfg)
                .chunk_indices(&text)
                .map(|(_, c)| c.to_string())
                .collect(),
            _ => TextSplitter::new(cfg)
                .chunk_indices(&text)
                .map(|(_, c)| c.to_string())
                .collect(),
        };
        // budget the prompt: take chunks until the char cap
        let mut body = String::new();
        let mut used = 0usize;
        for c in &chunks {
            if body.len() + c.len() > BUDGET {
                break;
            }
            used += 1;
            body.push_str("\n---\n");
            body.push_str(c);
        }
        let sym_names: Vec<String> = syms
            .iter()
            .filter(|s| &s.file == rel)
            .map(|s| s.name.clone())
            .take(30)
            .collect();
        let prompt = format!(
            "Summarize this source file for the repo memory index.\n\
             File: {}\nModule: {module}\nSymbols defined: {}\n\n\
             Contents ({used} of {} semantic chunks{}):\n{body}\n\n\
             Reply with ONLY `purpose:` followed by 1-3 grounded \
             sentences — this text is re-verified against the file.",
            rel.display(),
            sym_names.join(", "),
            chunks.len(),
            if used < chunks.len() { ", truncated" } else { "" },
        );
        let prose = match acp.complete(&prompt) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("acp file {}: {e}", rel.display());
                continue;
            }
        };
        // purpose = first non-empty line, `purpose:` stripped
        let purpose = prose
            .lines()
            .find_map(|l| {
                let t = l.trim();
                t.strip_prefix("purpose:")
                    .or_else(|| t.strip_prefix("Purpose:"))
                    .map(|p| p.trim().to_owned())
                    .filter(|p| !p.is_empty())
                    .or_else(|| (!t.is_empty()).then(|| t.to_owned()))
            })
            .unwrap_or_else(|| rel.display().to_string());
        let card = kuda_core::FileCard {
            path: rel.clone(),
            module: module.to_string(),
            purpose,
            symbols: sym_names,
            provenance: Provenance {
                files: vec![rel.clone()],
                hashes: vec![hash.clone()],
                commit_sha: sha.to_string(),
                generated_at: now.to_string(),
                generator: "kuda-acp/0.1.0".into(),
            },
        };
        if let Ok(y) = kuda_cards::render_file_card(&card) {
            if let Some(parent) = out.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&out, y) {
                eprintln!("write {}: {e}", out.display());
            }
        }
    }
}

/// Purpose fallback chain: manifest `description` -> crate `//!`
/// doc comment -> module README first line.
fn module_purpose(root: &Path, manifest: &Path) -> Option<String> {
    manifest_purpose(manifest)
        .or_else(|| crate_doc_purpose(&manifest.parent()?.join("src/lib.rs")))
        .or_else(|| crate_doc_purpose(&manifest.parent()?.join("src/main.rs")))
        .or_else(|| readme_purpose(&manifest.parent()?.join("README.md")))
        .or_else(|| readme_purpose(&root.join("README.md")))
}

/// First `//!` doc line — the crate's self-description.
fn crate_doc_purpose(lib: &Path) -> Option<String> {
    let text = std::fs::read_to_string(lib).ok()?;
    for line in text.lines() {
        let l = line.trim_start();
        if let Some(d) = l.strip_prefix("//!") {
            let d = d.trim();
            if !d.is_empty() {
                return Some(d.to_owned());
            }
        } else if !l.is_empty() && !l.starts_with("//") {
            return None; // first real code line — no doc comment
        }
    }
    None
}

/// First non-heading, non-empty README line.
fn readme_purpose(readme: &Path) -> Option<String> {
    let text = std::fs::read_to_string(readme).ok()?;
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('!') && !l.starts_with('['))
        .map(str::to_owned)
}

/// Pull `description` from a manifest (Cargo.toml/package.json).
fn manifest_purpose(manifest: &Path) -> Option<String> {
    let text = std::fs::read_to_string(manifest).ok()?;
    if manifest.file_name()? == "Cargo.toml" {
        let t = text.parse::<toml::Table>().ok()?;
        t.get("package")?
            .get("description")?
            .as_str()
            .map(str::to_owned)
    } else if manifest.file_name()? == "package.json" {
        let v = serde_json::from_str::<serde_json::Value>(&text).ok()?;
        v.get("description")?.as_str().map(str::to_owned)
    } else {
        None
    }
}

fn safe(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn detect_commands(dir: &Path) -> Vec<String> {
    let mut cmds = Vec::new();
    if dir.join("Cargo.toml").exists() {
        cmds.extend(["cargo check", "cargo test --workspace"].map(str::to_owned));
    }
    if dir.join("package.json").exists() {
        cmds.push("npm test".into());
    }
    if dir.join("go.mod").exists() {
        cmds.push("go test ./...".into());
    }
    cmds
}

/// Drift report: per-card hashes vs current content + sha vs HEAD.
fn status(cli: &Cli) -> i32 {
    let dir = root(cli);
    let moddir = dir.join(".kuda/modules");
    let Ok(entries) = std::fs::read_dir(&moddir) else {
        eprintln!("no .kuda/modules — run `kuda sync` first");
        return 2;
    };
    let head = kuda_git::GitVcs.head_sha(&dir).unwrap_or_default();
    let (mut fresh, mut drifted) = (0, 0);
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "yaml").unwrap_or(false) {
            let Ok(card) =
                kuda_cards::parse_module_card(&std::fs::read_to_string(&p).unwrap_or_default())
            else {
                continue;
            };
            let hashes = hash_files(&dir, &card.provenance.files);
            let stale = card
                .provenance
                .hashes
                .iter()
                .zip(&hashes)
                .any(|(a, b)| a != b)
                || card.provenance.files.len() != hashes.len();
            if stale {
                drifted += 1;
                println!("drifted  {}", card.name);
            } else {
                fresh += 1;
            }
        }
    }
    println!("{fresh} fresh, {drifted} drifted (HEAD {})", &head[..head.len().min(7)]);
    0
}

fn map(cli: &Cli) -> i32 {
    cat_file(&root(cli).join(".kuda/index.yaml"))
}

fn card(cli: &Cli, module: &str) -> i32 {
    cat_file(&root(cli).join(format!(".kuda/modules/{}.yaml", safe(module))))
}

fn cat_file(p: &Path) -> i32 {
    match std::fs::read_to_string(p) {
        Ok(t) => {
            print!("{t}");
            0
        }
        Err(e) => {
            eprintln!("{}: {e}", p.display());
            1
        }
    }
}

fn find(cli: &Cli, symbol: &str) -> i32 {
    let dir = root(cli);
    let cfg = load_config(cli);
    let roots = if cli.workspace {
        let r = workspace_roots(&dir);
        if r.is_empty() { vec![dir.clone()] } else { r }
    } else {
        vec![dir.clone()]
    };
    let mut hits = Vec::new();
    for r in &roots {
        if let Ok(store) = kuda_store_sqlite::SqliteStore::open(&r.join(&cfg.store.path)) {
            // exact first; on miss, fuzzy substring tier (marked)
            let mut syms = kuda_query::find(&store, symbol).unwrap_or_default();
            if syms.is_empty() {
                syms = kuda_query::search(&store, symbol).unwrap_or_default();
                for s in syms.iter_mut() {
                    s.module.push_str(" (fuzzy)");
                }
            }
            {
                // tag hits with their repo when spanning roots
                for s in syms.iter_mut() {
                    if roots.len() > 1 {
                        s.module = format!("{}:{}", repo_tag(r), s.module);
                    }
                }
                hits.append(&mut syms);
            }
        }
    }
    if hits.is_empty() {
        println!("no symbol `{symbol}` — run `kuda sync`?");
        return 1;
    }
    for s in hits {
        println!("{}:{} {:?} {}", s.file.display(), s.line, s.kind, s.module);
    }
    0
}

fn repo_tag(root: &Path) -> String {
    root.file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".into())
}

fn dependents(cli: &Cli, name: &str) -> i32 {
    let dir = root(cli);
    let cfg = load_config(cli);
    let roots = if cli.workspace {
        let r = workspace_roots(&dir);
        if r.is_empty() { vec![dir.clone()] } else { r }
    } else {
        vec![dir.clone()]
    };
    let mut edges = Vec::new();
    for r in &roots {
        if let Ok(store) = kuda_store_sqlite::SqliteStore::open(&r.join(&cfg.store.path)) {
            if let Ok(mut e) = kuda_query::dependents(&store, name) {
                edges.append(&mut e);
            }
        }
    }
    match Ok::<Vec<kuda_core::DepEdge>, String>(edges) {
        Ok(edges) if edges.is_empty() => {
            println!("nothing depends on `{name}`");
            0
        }
        Ok(edges) => {
            for e in edges {
                println!("{} --{:?}--> {}", e.from, e.kind, e.to);
            }
            0
        }
        Err(e) => {
            eprintln!("dependents: {e}");
            1
        }
    }
}

/// Textual mention sweep — the doc-update use case. With
/// `--workspace`, sweeps every root; with `extract.lsp_server`
/// set and `lsp` in kinds, tries semantic references first.
fn refs(cli: &Cli, name: &str, docs_only: bool) -> i32 {
    let dir = root(cli);
    let cfg = load_config(cli);
    let roots = if cli.workspace {
        let r = workspace_roots(&dir);
        if r.is_empty() { vec![dir.clone()] } else { r }
    } else {
        vec![dir.clone()]
    };
    // semantic tier first (only when not docs-only)
    if !docs_only && cfg.extract.kinds.iter().any(|k| k == "lsp") {
        if let Some(cmd) = cfg.extract.lsp_server.clone() {
            let mut any = false;
            for r in &roots {
                match kuda_lsp::references(&cmd, r, name) {
                    Ok(locs) if !locs.is_empty() => {
                        any = true;
                        for (f, l) in locs {
                            let rel = f.strip_prefix(r).unwrap_or(&f);
                            println!("{}:{}{}", rel.display(), l,
                                if roots.len() > 1 { format!("  [{}]", repo_tag(r)) } else { String::new() });
                        }
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("lsp refs: {e}"),
                }
            }
            if any {
                return 0;
            }
        }
    }
    let mut total = 0;
    for dir in &roots {
        total += refs_in(dir, name, docs_only, roots.len() > 1);
    }
    if total == 0 {
        println!("no mentions of `{name}`");
    }
    0
}

fn refs_in(dir: &Path, name: &str, docs_only: bool, tag: bool) -> usize {
    let exts: &[&str] = if docs_only {
        &["md", "mdx", "rst", "txt"]
    } else {
        &["md", "mdx", "rs", "py", "ts", "tsx", "go", "toml", "yaml", "yml", "sh", "txt"]
    };
    let mut hits = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                // search uses hard-noise excludes only — NOT the
                // [extract] exclude list: fixtures/testdata are
                // legitimate places to grep for symbol uses.
                if !e
                    .file_name()
                    .to_str()
                    .map(|n| kuda_core::dir_excluded(n, &[]))
                    .unwrap_or(true)
                {
                    stack.push(p);
                }
                continue;
            }
            let ext_ok = p
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| exts.contains(&x))
                .unwrap_or(false);
            if !ext_ok {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&p) {
                for (i, line) in text.lines().enumerate() {
                    if line.contains(name) {
                        hits += 1;
                        let rel = p.strip_prefix(dir).unwrap_or(&p);
                        let t = if tag {
                            format!("  [{}]", repo_tag(dir))
                        } else {
                            String::new()
                        };
                        println!("{}:{}: {}{t}", rel.display(), i + 1, line.trim());
                    }
                }
            }
        }
    }
    hits
}

/// Minimal MCP stdio server — the index as agent tools.
/// Line-delimited JSON-RPC 2.0; tools: repo_card, module_card,
/// find, dependents, status, refs.
fn mcp(cli: &Cli) -> i32 {
    use serde_json::{json, Value};
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let dir = root(cli);
    let cfg = load_config(cli);
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let method = req["method"].as_str().unwrap_or("");
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let reply = |result: Value| {
            println!("{}", json!({"jsonrpc":"2.0","id":id,"result":result}));
        };
        let tool_text = |text: String| {
            json!({"content":[{"type":"text","text":text}]})
        };
        match method {
            "initialize" => reply(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "kuda", "version": "0.1.0"}
            })),
            "tools/list" => reply(json!({"tools":[
                {"name":"repo_card","description":"L0 repo card — always-load summary","inputSchema":{"type":"object","properties":{}}},
                {"name":"module_card","description":"L1 module card + freshness","inputSchema":{"type":"object","properties":{"module":{"type":"string"}},"required":["module"]}},
                {"name":"find","description":"symbol -> file:line","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}},
                {"name":"dependents","description":"reverse dep lookup (impact analysis)","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}},
                {"name":"status","description":"provenance drift vs HEAD","inputSchema":{"type":"object","properties":{}}},
                {"name":"refs","description":"code+docs mentions","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"docs":{"type":"boolean"}},"required":["name"]}},
            ]})),
            "tools/call" => {
                let name = req["params"]["name"].as_str().unwrap_or("");
                let args = &req["params"]["arguments"];
                let arg_s = |k: &str| args[k].as_str().unwrap_or("").to_owned();
                let text = match name {
                    "repo_card" => std::fs::read_to_string(dir.join(".kuda/index.yaml"))
                        .unwrap_or_else(|e| format!("error: {e}")),
                    "module_card" => std::fs::read_to_string(
                        dir.join(format!(".kuda/modules/{}.yaml", safe(&arg_s("module")))),
                    )
                    .unwrap_or_else(|e| format!("error: {e}")),
                    "find" | "dependents" => {
                        let n = arg_s("name");
                        match kuda_store_sqlite::SqliteStore::open(&dir.join(&cfg.store.path)) {
                            Ok(store) => {
                                if name == "find" {
                                    kuda_query::find(&store, &n)
                                        .map(|v| {
                                            v.iter()
                                                .map(|s| {
                                                    format!(
                                                        "{}:{} {:?} {}",
                                                        s.file.display(),
                                                        s.line,
                                                        s.kind,
                                                        s.module
                                                    )
                                                })
                                                .collect::<Vec<_>>()
                                                .join("\n")
                                        })
                                        .unwrap_or_else(|e| format!("error: {e}"))
                                } else {
                                    kuda_query::dependents(&store, &n)
                                        .map(|v| {
                                            v.iter()
                                                .map(|e| {
                                                    format!(
                                                        "{} --{:?}--> {}",
                                                        e.from, e.kind, e.to
                                                    )
                                                })
                                                .collect::<Vec<_>>()
                                                .join("\n")
                                        })
                                        .unwrap_or_else(|e| format!("error: {e}"))
                                }
                            }
                            Err(e) => format!("error: {e}"),
                        }
                    }
                    _ => format!("unknown tool {name}"),
                };
                reply(tool_text(text));
            }
            _ => {} // notifications — ignore
        }
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
    0
}

/// Minimal ACP server — same index, different wire protocol.
/// Prompts are tiny query strings: `find X`, `dependents X`,
/// `card X`, `map`, `status`; anything else echoes the L0 card.
fn acp(cli: &Cli) -> i32 {
    use serde_json::{json, Value};
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let dir = root(cli);
    let cfg = load_config(cli);
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let method = req["method"].as_str().unwrap_or("");
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        match method {
            "initialize" => println!("{}", json!({
                "jsonrpc":"2.0","id":id,
                "result":{"protocolVersion":1,
                    "agentCapabilities":{"loadSession":false,
                        "promptCapabilities":{"image":false,"audio":false,"embeddedContext":true}},
                    "agentInfo":{"name":"kuda","version":"0.1.0","title":"kuda — repo memory"}}
            })),
            "session/new" => println!("{}", json!({
                "jsonrpc":"2.0","id":id,
                "result":{"sessionId":"kuda-default"}
            })),
            "session/prompt" => {
                let sid = req["params"]["sessionId"].as_str().unwrap_or("kuda-default");
                let prompt: String = req["params"]["prompt"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|b| b["text"].as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let text = acp_answer(&dir, &cfg, &prompt);
                println!("{}", json!({"jsonrpc":"2.0","method":"session/update",
                    "params":{"sessionId":sid,"update":{"sessionUpdate":"agent_message_chunk",
                        "content":{"type":"text","text":text}}}}));
                println!("{}", json!({"jsonrpc":"2.0","id":id,
                    "result":{"stopReason":"end_turn"}}));
            }
            "session/cancel" | "authenticated" => {}
            _ => println!("{}", json!({"jsonrpc":"2.0","id":id,
                "error":{"code":-32601,"message":"method not found"}})),
        }
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
    0
}

/// Turn a natural-ish prompt into an index query.
fn acp_answer(dir: &Path, cfg: &Config, prompt: &str) -> String {
    let p = prompt.trim();
    let (verb, arg) = p.split_once(' ').map(|(v, a)| (v, a.trim())).unwrap_or((p, ""));
    match verb.to_ascii_lowercase().as_str() {
        "find" => kuda_store_sqlite::SqliteStore::open(&dir.join(&cfg.store.path))
            .and_then(|s| kuda_query::find(&s, arg))
            .map(|v| {
                v.iter()
                    .map(|s| format!("{}:{} {:?} {}", s.file.display(), s.line, s.kind, s.module))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|e| format!("error: {e}")),
        "dependents" | "deps" => kuda_store_sqlite::SqliteStore::open(&dir.join(&cfg.store.path))
            .and_then(|s| kuda_query::dependents(&s, arg))
            .map(|v| {
                v.iter()
                    .map(|e| format!("{} --{:?}--> {}", e.from, e.kind, e.to))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|e| format!("error: {e}")),
        "card" => std::fs::read_to_string(dir.join(format!(".kuda/modules/{}.yaml", safe(arg))))
            .unwrap_or_else(|e| format!("error: {e}")),
        "map" | "status" | _ => std::fs::read_to_string(dir.join(".kuda/index.yaml"))
            .unwrap_or_else(|e| format!("error: {e}")),
    }
}

fn doctor(cli: &Cli) -> i32 {
    println!("kuda doctor");
    match (TomlSource {
        path: cli.config.clone(),
    })
    .load()
    {
        Ok(c) => println!(
            "  config   ok  (extract: {:?}, summarize: {}, store: {})",
            c.extract.kinds, c.summarize.kind, c.store.kind
        ),
        Err(e) => println!("  config   --  {e}"),
    }
    println!("  adapters    manifest, treesitter, lsp, template, sqlite (llm pending)");
    0
}
