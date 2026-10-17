use kuda_config::ConfigSource;
use kuda_config_toml::TomlSource;

fn tmp(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("kuda-cfg-{}-{tag}.toml", std::process::id()))
}

const CFG: &str = r#"
[extract]
kinds = ["manifest", "treesitter"]
[summarize]
kind = "template"
[store]
kind = "sqlite"
"#;

#[test]
fn loads_via_env_var() {
    let f = tmp("a");
    std::fs::write(&f, CFG).unwrap();
    unsafe {
        std::env::set_var("KUDA_CONFIG", &f);
    }
    let c = TomlSource { path: None }.load().unwrap();
    assert_eq!(c.summarize.kind, "template");
    assert_eq!(c.extract.kinds[0], "manifest");
    unsafe {
        std::env::remove_var("KUDA_CONFIG");
    }
    let _ = std::fs::remove_file(&f);
}

#[test]
fn explicit_path_and_missing_error() {
    let f = tmp("b");
    std::fs::write(&f, CFG).unwrap();
    let c = TomlSource {
        path: Some(f.to_string_lossy().into()),
    }
    .load()
    .unwrap();
    assert_eq!(c.store.kind, "sqlite");
    assert!(TomlSource {
        path: Some("/nonexistent/x".into())
    }
    .load()
    .is_err());
    let _ = std::fs::remove_file(&f);
}
