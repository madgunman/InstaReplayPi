//! Loopback HTTP API smoke — run with engine subprocess or via mvp_accept-full.sh.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

fn engine_bin() -> std::path::PathBuf {
    std::env::var("CARGO_BIN_EXE_replay-engine")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("target")
                .join(profile)
                .join("replay-engine")
        })
}

fn curl_get(path: &str) -> Option<String> {
    let out = Command::new("curl")
        .args(["-sfS", "--max-time", "5", &format!("http://127.0.0.1:8080{path}")])
        .output()
        .ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok()
    } else {
        None
    }
}

fn curl_post(path: &str, body: &str) -> Option<String> {
    let out = Command::new("curl")
        .args([
            "-sfS",
            "--max-time",
            "30",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            body,
            &format!("http://127.0.0.1:8080{path}"),
        ])
        .output()
        .ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok()
    } else {
        String::from_utf8(out.stderr).ok()
    }
}

fn wait_health(max_secs: u32) -> bool {
    for _ in 0..max_secs {
        if curl_get("/api/health").is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    false
}

struct EngineProc {
    child: Child,
}

impl Drop for EngineProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_engine() -> EngineProc {
    let bin = engine_bin();
    assert!(bin.exists(), "replay-engine binary not found at {}", bin.display());
    let child = Command::new(&bin)
        .args(["--test", "--no-ui"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn replay-engine");
    EngineProc { child }
}

#[test]
fn http_health_and_start_live_smoke() {
    if Command::new("curl").output().is_err() {
        eprintln!("skip http_api: curl not installed");
        return;
    }

    let _engine = spawn_engine();
    assert!(wait_health(45), "HTTP /api/health did not respond");

    let health = curl_get("/api/health").expect("health");
    assert!(health.contains("\"ok\""), "health: {health}");

    let start = curl_post(
        "/api/start-live",
        r#"{"device_id":"test","display_id":0,"fullscreen":false,"width":1280,"height":720,"fps":30,"pixel_format":"auto"}"#,
    )
    .expect("start-live");
    assert!(
        start.contains("\"ok\":true") || start.contains("\"ok\": true"),
        "start-live failed: {start}"
    );

    std::thread::sleep(Duration::from_secs(2));

    let status = curl_get("/api/status").expect("status");
    assert!(
        status.contains("bufferSecondsAvailable") || status.contains("buffer_seconds"),
        "status: {status}"
    );
}
