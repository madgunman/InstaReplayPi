//! Local HTTP API for Pi touch UI (127.0.0.1:8080).

use std::net::SocketAddr;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use replay_core::config::AppConfig;
use replay_core::fsm::ReplayState;
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;
use tracing::info;

use crate::control_api::ControlApi;

#[derive(Clone)]
pub struct HttpState {
    pub api: ControlApi,
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    ready: bool,
    version: &'static str,
}

#[derive(Serialize)]
struct ActionResponse {
    ok: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    error: String,
}

#[derive(Deserialize)]
struct StartLiveBody {
    device_id: String,
    width: u32,
    height: u32,
    fps: u32,
    #[serde(default = "default_pixel_format")]
    pixel_format: String,
    #[serde(default)]
    display_id: u32,
    #[serde(default = "default_fullscreen")]
    fullscreen: bool,
}

fn default_pixel_format() -> String {
    "auto".into()
}

fn default_fullscreen() -> bool {
    true
}

#[derive(Deserialize, Default)]
struct ReplayLastBody {
    #[serde(default)]
    seconds: u32,
}

pub async fn serve(bind_addr: &str, api: ControlApi) -> anyhow::Result<()> {
    let addr: SocketAddr = bind_addr.parse()?;
    let state = HttpState { api };

    let assets_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/touch");
    let serve_assets = ServeDir::new(assets_dir);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/status", get(status))
        .route("/api/diagnostics", get(diagnostics))
        .route("/api/config", get(get_config))
        .route("/api/devices", get(devices))
        .route("/api/formats/{device_id}", get(formats))
        .route("/api/displays", get(displays))
        .route("/api/mark", post(mark))
        .route("/api/replay", post(replay))
        .route("/api/replay-last", post(replay_last))
        .route("/api/return-live", post(return_live))
        .route("/api/clear-mark", post(clear_mark))
        .route("/api/start-live", post(start_live))
        .route("/api/stop", post(stop))
        .fallback_service(serve_assets)
        .with_state(state);

    info!(%addr, "Touch HTTP server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(st): State<HttpState>) -> Json<HealthResponse> {
    let ready = st.api.engine_ready().await;
    Json(HealthResponse {
        ok: true,
        ready,
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn status(State(st): State<HttpState>) -> Json<serde_json::Value> {
    let snap = st.api.status().await;
    Json(status_json(&snap))
}

async fn diagnostics(State(st): State<HttpState>) -> Json<serde_json::Value> {
    let d = st.api.diagnostics().await;
    Json(serde_json::to_value(d).unwrap_or_default())
}

async fn get_config(State(st): State<HttpState>) -> Json<AppConfig> {
    Json(st.api.get_config().await)
}

async fn devices(State(st): State<HttpState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "devices": st.api.list_devices_json() }))
}

async fn formats(
    axum::extract::Path(device_id): axum::extract::Path<String>,
    State(st): State<HttpState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "formats": st.api.list_formats(&device_id) }))
}

async fn displays(State(st): State<HttpState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "displays": st.api.list_displays() }))
}

async fn mark(State(st): State<HttpState>) -> impl IntoResponse {
    action_result(st.api.mark().await.map(|_| ()))
}

async fn replay(State(st): State<HttpState>) -> impl IntoResponse {
    action_result(st.api.replay().await)
}

async fn replay_last(
    State(st): State<HttpState>,
    Json(body): Json<ReplayLastBody>,
) -> impl IntoResponse {
    action_result(st.api.replay_last(body.seconds).await)
}

async fn return_live(State(st): State<HttpState>) -> impl IntoResponse {
    action_result(st.api.return_live().await)
}

async fn clear_mark(State(st): State<HttpState>) -> impl IntoResponse {
    action_result(st.api.clear_mark().await)
}

async fn start_live(
    State(st): State<HttpState>,
    Json(body): Json<StartLiveBody>,
) -> impl IntoResponse {
    action_result(
        st.api
            .start_live(
                body.device_id,
                body.width,
                body.height,
                body.fps,
                body.pixel_format,
                body.display_id,
                body.fullscreen,
            )
            .await,
    )
}

async fn stop(State(st): State<HttpState>) -> impl IntoResponse {
    action_result(st.api.stop().await)
}

fn action_result(r: anyhow::Result<()>) -> (StatusCode, Json<ActionResponse>) {
    match r {
        Ok(()) => (
            StatusCode::OK,
            Json(ActionResponse {
                ok: true,
                error: String::new(),
            }),
        ),
        Err(e) => (
            StatusCode::OK,
            Json(ActionResponse {
                ok: false,
                error: e.to_string(),
            }),
        ),
    }
}

fn status_json(snap: &crate::controller::StatusSnapshot) -> serde_json::Value {
    serde_json::json!({
        "state": snap.state.as_str(),
        "inputFps": snap.input_fps,
        "droppedFrames": snap.dropped_frames,
        "bufferSecondsAvailable": snap.buffer_seconds_available,
        "diskWarning": snap.disk_warning,
        "lastError": snap.last_error,
        "bufferReady": snap.buffer_ready,
        "bufferError": snap.buffer_error,
        "markTimestampNs": snap.mark_timestamp_ns,
        "engineConnected": true,
    })
}

/// Gate helpers matching touch UI (1.5s min buffer for replay).
pub fn can_mark(snap: &crate::controller::StatusSnapshot) -> bool {
    snap.buffer_ready
        && matches!(snap.state, ReplayState::Live | ReplayState::Marked)
}

pub fn can_replay(snap: &crate::controller::StatusSnapshot) -> bool {
    snap.buffer_seconds_available >= replay_core::buffer::MIN_REPLAY_BUFFER_SECS
        && matches!(
            snap.state,
            ReplayState::Live | ReplayState::Marked | ReplayState::Replaying
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::StatusSnapshot;
    use replay_core::fsm::ReplayState;

    fn snap(state: ReplayState, buffer_ready: bool, secs: f64) -> StatusSnapshot {
        StatusSnapshot {
            state,
            input_fps: 30.0,
            dropped_frames: 0,
            buffer_seconds_available: secs,
            disk_warning: false,
            last_error: String::new(),
            buffer_ready,
            buffer_error: false,
            mark_timestamp_ns: 0,
            sequence: 0,
        }
    }

    #[test]
    fn gates_require_buffer_ready_for_mark() {
        assert!(!can_mark(&snap(ReplayState::Live, false, 2.0)));
        assert!(can_mark(&snap(ReplayState::Live, true, 2.0)));
    }

    #[test]
    fn gates_require_min_secs_for_replay() {
        assert!(!can_replay(&snap(ReplayState::Live, true, 0.5)));
        assert!(can_replay(&snap(ReplayState::Live, true, 2.0)));
    }
}
