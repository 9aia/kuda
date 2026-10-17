//! kuda — a hippocampus for your codebase. Composition root:
//! parses kuda.toml, wires impls behind ports, runs subcommands.

use clap::{Parser, Subcommand};
use kuda_config_toml::TomlSource;
use std::path::Path;

const EXAMPLE_TOML: &str = include_str!("../../../examples/kuda.toml.example");

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
        /// Include docs/comments/markdown mentions.
        #[arg(long)]
        docs: bool,
    },
    /// Serve the index over MCP (stdio).
    Mcp,
    /// Extractor/store/config health.
    Doctor,
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.cmd {
        Cmd::Init => init(&cli),
        Cmd::Doctor => doctor(&cli),
        _ => {
            eprintln!("kuda: not yet implemented — see project/TODO.md");
            2
        }
    };
    std::process::exit(code);
}

fn init(cli: &Cli) -> i32 {
    let dir = cli
        .config
        .as_deref()
        .map(Path::new)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| ".".into());
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
    println!(
        "wrote {} + .kuda/ — run `kuda sync` when extractors land",
        toml_path.display()
    );
    0
}

fn doctor(cli: &Cli) -> i32 {
    println!("kuda doctor");
    let src = TomlSource {
        path: cli.config.clone(),
    };
    match kuda_config::ConfigSource::load(&src) {
        Ok(c) => {
            println!("  config   ok  (extract: {:?}, summarize: {}, store: {})",
                c.extract.kinds, c.summarize.kind, c.store.kind);
        }
        Err(e) => println!("  config   --  {e}"),
    }
    println!("  impls    manifest, template, sqlite (treesitter/lsp/llm/mcp pending)");
    0
}
