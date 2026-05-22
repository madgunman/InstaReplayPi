//! Headless engine smoke is run via `scripts/mvp_accept-full.sh` (--test --no-ui).

#[test]
fn mvp_accept_scripts_exist() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(root.join("scripts/mvp_accept-full.sh").exists());
}
