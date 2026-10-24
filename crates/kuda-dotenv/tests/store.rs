use kuda_dotenv::DotenvSecrets;
use kuda_secret::Secrets;

fn tmp(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("kuda-env-{}-{tag}", std::process::id()))
}

#[test]
fn set_get_update() {
    let f = tmp("a");
    let s = DotenvSecrets { path: f.clone() };
    s.set("A", "1").unwrap();
    s.set("B", "2").unwrap();
    s.set("A", "3").unwrap(); // update in place
    assert_eq!(s.get("A").as_deref(), Some("3"));
    assert_eq!(s.get("B").as_deref(), Some("2"));
    assert_eq!(s.get("MISSING"), None);
    let text = std::fs::read_to_string(&f).unwrap();
    assert_eq!(text.matches("A=").count(), 1);
    let _ = std::fs::remove_file(&f);
}

#[test]
fn get_skips_malformed_and_empty() {
    let f = tmp("c");
    // no `=` line, empty value, then the hit
    std::fs::write(&f, "NOSEP\nEMPTY=\nGOOD=val\n").unwrap();
    let s = DotenvSecrets { path: f.clone() };
    assert_eq!(s.get("EMPTY"), None);
    assert_eq!(s.get("GOOD").as_deref(), Some("val"));
    let _ = std::fs::remove_file(&f);
}

/// A parentless path skips the mkdir entirely — the bare write
/// then fails on its own.
#[test]
fn set_no_parent_path() {
    let s = DotenvSecrets {
        path: std::path::PathBuf::from("/"),
    };
    assert!(s.set("K", "v").is_err());
}

#[test]
fn set_error_when_parent_is_file() {
    let dir = std::env::temp_dir().join(format!("kuda-env-{}-err", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("afile");
    std::fs::write(&f, "x").unwrap();
    let s = DotenvSecrets { path: f.join("secrets") };
    assert!(s.set("K", "v").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn perms_600() {
    use std::os::unix::fs::PermissionsExt;
    let f = tmp("b");
    let s = DotenvSecrets { path: f.clone() };
    s.set("K", "v").unwrap();
    assert_eq!(
        std::fs::metadata(&f).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let _ = std::fs::remove_file(&f);
}
