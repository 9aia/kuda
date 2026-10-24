use kuda_config::ConfigSource;
use kuda_config_toml::TomlSource;

fn tmp(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("kuda-cfg-{}-{tag}.toml", std::process::id()))
}

/// env + cwd mutation is process-global — serialize the tests
/// that touch them.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

#[test]
fn parse_error_and_dotfile_fallback() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // malformed toml -> parse error surfaces the path
    let f = tmp("c");
    std::fs::write(&f, "not [valid toml").unwrap();
    let e = TomlSource {
        path: Some(f.to_string_lossy().into()),
    }
    .load()
    .unwrap_err();
    assert!(e.contains("parse"), "{e}");
    let _ = std::fs::remove_file(&f);

    // default search: ./kuda.toml picked up from cwd
    let dir = std::env::temp_dir().join(format!("kuda-cfg-{}-cwd", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("kuda.toml"), CFG).unwrap();
    let prev = std::env::current_dir().unwrap();
    let saved_cfg = std::env::var("KUDA_CONFIG").ok();
    unsafe {
        std::env::remove_var("KUDA_CONFIG");
    }
    std::env::set_current_dir(&dir).unwrap();
    let c = TomlSource { path: None }.load().unwrap();
    std::env::set_current_dir(prev).unwrap();
    if let Some(v) = saved_cfg {
        unsafe {
            std::env::set_var("KUDA_CONFIG", v);
        }
    }
    assert_eq!(c.store.kind, "sqlite");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn unreadable_config_is_a_read_error() {
    use std::os::unix::fs::PermissionsExt;
    let f = tmp("d");
    std::fs::write(&f, CFG).unwrap();
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o000)).unwrap();
    let e = TomlSource {
        path: Some(f.to_string_lossy().into()),
    }
    .load()
    .unwrap_err();
    assert!(e.contains("read"), "{e}");
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
    let _ = std::fs::remove_file(&f);
}

#[test]
fn home_fallback_when_unset() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // no explicit path, no KUDA_CONFIG, no HOME -> the HOME arm of
    // the default search still produces a candidate path.
    let saved_home = std::env::var("HOME").ok();
    let saved_cfg = std::env::var("KUDA_CONFIG").ok();
    unsafe {
        std::env::remove_var("HOME");
        std::env::remove_var("KUDA_CONFIG");
    }
    // cwd has no kuda.toml here -> "no kuda.toml found"
    let dir = std::env::temp_dir().join(format!("kuda-cfg-{}-nohome", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(&dir).unwrap();
    let e = TomlSource { path: None }.load().unwrap_err();
    std::env::set_current_dir(prev).unwrap();
    assert!(e.contains("no kuda.toml"), "{e}");
    unsafe {
        if let Some(v) = saved_home {
            std::env::set_var("HOME", v);
        }
        if let Some(v) = saved_cfg {
            std::env::set_var("KUDA_CONFIG", v);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
