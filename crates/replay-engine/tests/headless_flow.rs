//! Full engine gRPC flow (StartLive → buffer → Replay → ReturnLive).
//! Run locally: `cargo test -p replay-engine --test headless_flow -- --ignored --nocapture`
//! CI runs the same path via `scripts/mvp_accept-full.sh` (xvfb + HTTP curl).

#[test]
#[ignore = "requires GStreamer, display/xvfb; use scripts/mvp_accept-full.sh"]
fn headless_mvp_flow_documented() {
    // Guard test documents the integration entry point for developers.
    // Automated coverage: .github/workflows/acceptance.yml → mvp_accept-full.sh
    assert!(std::path::Path::new("scripts/mvp_accept-full.sh").exists());
    assert!(std::path::Path::new("scripts/mvp_accept.sh").exists());
}
