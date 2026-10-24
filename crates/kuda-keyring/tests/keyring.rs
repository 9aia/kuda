use kuda_keyring::KeyringSecrets;
use kuda_secret::Secrets;

/// One test doing the whole PATH-stubbed flow — env mutation is
/// process-global, keep it serialized inside a single test.
#[test]
fn service_lookup_and_stubbed_secret_tool_roundtrip() {
    assert_eq!(KeyringSecrets::default().service, "kuda");
    {
        let s = KeyringSecrets { service: "kuda-test".into() };
        assert_eq!(s.get("kuda-test-definitely-missing"), None);
    }
    let bin = std::env::temp_dir().join(format!("kuda-keyring-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bin);
    std::fs::create_dir_all(&bin).unwrap();
    let stub = bin.join("secret-tool");
    std::fs::write(
        &stub,
        "#!/bin/sh\ncase \"$1\" in\n\
         \x20 lookup) [ \"$KUDA_STUB_MODE\" = fail ] && exit 1; \
         [ \"$KUDA_STUB_MODE\" = empty ] && exit 0; echo \"s3cret\";;\n\
         \x20 store) cat >/dev/null; [ \"$KUDA_STUB_MODE\" = fail ] && exit 1; exit 0;;\n\
         esac\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let orig_path = std::env::var("PATH").unwrap_or_default();
    unsafe {
        std::env::set_var("PATH", format!("{}:{orig_path}", bin.display()));
    }
    let s = KeyringSecrets {
        service: "kuda-test".into(),
    };
    // lookup hit + store success
    assert_eq!(s.get("k").as_deref(), Some("s3cret"));
    s.set("k", "v").unwrap();
    // store failure -> io error
    unsafe {
        std::env::set_var("KUDA_STUB_MODE", "fail");
    }
    assert!(s.set("k", "v").is_err());
    assert_eq!(s.get("k"), None);
    // empty stdout -> None
    unsafe {
        std::env::set_var("KUDA_STUB_MODE", "empty");
    }
    assert_eq!(s.get("k"), None);
    // secret-tool missing entirely -> None / spawn error
    unsafe {
        std::env::remove_var("KUDA_STUB_MODE");
        std::env::set_var("PATH", &bin);
    }
    std::fs::remove_file(&stub).unwrap();
    assert_eq!(s.get("k"), None);
    assert!(s.set("k", "v").is_err());
    unsafe {
        std::env::set_var("PATH", orig_path);
    }
    let _ = std::fs::remove_dir_all(&bin);
}
