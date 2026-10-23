use kuda_keyring::KeyringSecrets;
use kuda_secret::Secrets;

#[test]
fn default_service_and_missing_lookup() {
    assert_eq!(KeyringSecrets::default().service, "kuda");
    let s = KeyringSecrets { service: "kuda-test".into() };
    assert_eq!(s.get("kuda-test-definitely-missing"), None);
}
