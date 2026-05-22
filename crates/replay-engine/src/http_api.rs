//! Loopback-only HTTP control API for acceptance scripts and local diagnostics.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use replay_core::config::AppConfig;
use replay_core::types::{DisplayInfo, VideoDevice, VideoFormat};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::info;

use crate::control_api::ControlApi;
use crate::controller::{Diagnostics, StatusSnapshot};

#[derive(Clone)]
struct HttpState {
    api: ControlApi,
}

#[derive(Serialize)]
struct OkResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    ready: bool,
}

#[derive(Serialize, Deserialize)]
struct StartLiveBody {
    device_id: String,
    width: u32,
    height: u32,
    fps: u32,
    pixel_format: String,
    display_id: u32,
    fullscreen: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct ReplayLastBody {
    seconds: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusJson {
    state: String,
    input_fps: f64,
    dropped_frames: u64,
    buffer_seconds_available: f64,
    disk_warning: bool,
    last_error: String,
    buffer_ready: bool,
    buffer_error: bool,
    mark_timestamp_ns: i64,
    sequence: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsJson {
    input_fps: f64,
    dropped_frames: u64,
    buffer_seconds_available: f64,
    disk_warning: bool,
    buffer_error: bool,
    current_state: String,
    last_error: String,
    replay_trigger_delay_ms: f64,
}

#[derive(Serialize)]
struct DisplaysResponse {
    displays: Vec<DisplayInfo>,
}

#[derive(Serialize)]
struct DevicesResponse {
    devices: Vec<VideoDevice>,
}

#[derive(Serialize)]
struct FormatsResponse {
    formats: Vec<VideoFormat>,
}

fn ok_empty() -> Json<OkResponse> {
    Json(OkResponse {
        ok: true,
        error: None,
    })
}

fn err_response(msg: impl Into<String>) -> (StatusCode, Json<OkResponse>) {
    (
        StatusCode::OK,
        Json(OkResponse {
            ok: false,
            error: Some(msg.into()),
        }),
    )
}

fn status_to_json(s: StatusSnapshot) -> StatusJson {
    StatusJson {
        state: s.state.as_str().to_string(),
        input_fps: s.input_fps,
        dropped_frames: s.dropped_frames,
        buffer_seconds_available: s.buffer_seconds_available,
        disk_warning: s.disk_warning,
        last_error: s.last_error,
        buffer_ready: s.buffer_ready,
        buffer_error: s.buffer_error,
        mark_timestamp_ns: s.mark_timestamp_ns,
        sequence: s.sequence,
    }
}

fn diag_to_json(d: Diagnostics) -> DiagnosticsJson {
    DiagnosticsJson {
        input_fps: d.input_fps,
        dropped_frames: d.dropped_frames,
        buffer_seconds_available: d.buffer_seconds_available,
        disk_warning: d.disk_warning,
        buffer_error: d.buffer_error,
        current_state: d.current_state,
        last_error: d.last_error,
        replay_trigger_delay_ms: d.replay_trigger_delay_ms,
    }
}

async fn health(State(st): State<Arc<HttpState>>) -> Json<HealthResponse> {
    let ready = st.api.engine_ready().await;
    Json(HealthResponse { ok: true, ready })
}

async fn status(State(st): State<Arc<HttpState>>) -> Json<StatusJson> {
    let snap = st.api.status().await;
    Json(status_to_json(snap))
}

async fn diagnostics(State(st): State<Arc<HttpState>>) -> Json<DiagnosticsJson> {
    let d = st.api.diagnostics().await;
    Json(diag_to_json(d))
}

async fn devices(State(st): State<Arc<HttpState>>) -> Json<DevicesResponse> {
    Json(DevicesResponse {
        devices: st.api.list_devices_json(),
    })
}

async fn displays(State(st): State<Arc<HttpState>>) -> Json<DisplaysResponse> {
    Json(DisplaysResponse {
        displays: st.api.list_displays(),
    })
}

async fn formats(
    State(st): State<Arc<HttpState>>,
    Path(device_id): Path<String>,
) -> Json<FormatsResponse> {
    Json(FormatsResponse {
        formats: st.api.list_formats(&device_id),
    })
}

async fn config_get(State(st): State<Arc<HttpState>>) -> Json<AppConfig> {
    Json(st.api.get_config().await)
}

async fn start_live(
    State(st): State<Arc<HttpState>>,
    Json(body): Json<StartLiveBody>,
) -> Result<Json<OkResponse>, (StatusCode, Json<OkResponse>)> {
    match st
        .api
        .start_live(
            body.device_id,
            body.width,
            body.height,
            body.fps,
            body.pixel_format,
            body.display_id,
            body.fullscreen,
        )
        .await
    {
        Ok(()) => Ok(ok_empty()),
        Err(e) => Err(err_response(e.to_string())),
    }
}

async fn stop(State(st): State<Arc<HttpState>>) -> Result<Json<OkResponse>, (StatusCode, Json<OkResponse>)> {
    match st.api.stop().await {
        Ok(()) => Ok(ok_empty()),
        Err(e) => Err(err_response(e.to_string())),
    }
}

async fn mark(State(st): State<Arc<HttpState>>) -> Result<Json<OkResponse>, (StatusCode, Json<OkResponse>)> {
    match st.api.mark().await {
        Ok(_) => Ok(ok_empty()),
        Err(e) => Err(err_response(e.to_string())),
    }
}

async fn clear_mark(
    State(st): State<Arc<HttpState>>,
) -> Result<Json<OkResponse>, (StatusCode, Json<OkResponse>)> {
    match st.api.clear_mark().await {
        Ok(()) => Ok(ok_empty()),
        Err(e) => Err(err_response(e.to_string())),
    }
}

async fn replay(State(st): State<Arc<HttpState>>) -> Result<Json<OkResponse>, (StatusCode, Json<OkResponse>)> {
    match st.api.replay().await {
        Ok(()) => Ok(ok_empty()),
        Err(e) => Err(err_response(e.to_string())),
    }
}

async fn replay_last(
    State(st): State<Arc<HttpState>>,
    Json(body): Json<ReplayLastBody>,
) -> Result<Json<OkResponse>, (StatusCode, Json<OkResponse>)> {
    let secs = body.seconds.unwrap_or(0);
    match st.api.replay_last(secs).await {
        Ok(()) => Ok(ok_empty()),
        Err(e) => Err(err_response(e.to_string())),
    }
}

async fn return_live(
    State(st): State<Arc<HttpState>>,
) -> Result<Json<OkResponse>, (StatusCode, Json<OkResponse>)> {
    match st.api.return_live().await {
        Ok(()) => Ok(ok_empty()),
        Err(e) => Err(err_response(e.to_string())),
    }
}

fn router(state: Arc<HttpState>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/status", get(status))
        .route("/api/diagnostics", get(diagnostics))
        .route("/api/devices", get(devices))
        .route("/api/displays", get(displays))
        .route("/api/formats/{device_id}", get(formats))
        .route("/api/config", get(config_get))
        .route("/api/start-live", post(start_live))
        .route("/api/stop", post(stop))
        .route("/api/mark", post(mark))
        .route("/api/clear-mark", post(clear_mark))
        .route("/api/replay", post(replay))
        .route("/api/replay-last", post(replay_last))
        .route("/api/return-live", post(return_live))
        .with_state(state)
}

/// Bind address for the loopback HTTP API (127.0.0.1 only).
pub fn bind_addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

pub fn http_port_from_env() -> u16 {
    std::env::var("INSTANT_REPLAY_HTTP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080)
}

pub fn http_enabled_by_default(test: bool, appliance: bool, explicit: bool) -> bool {
    if std::env::var("INSTANT_REPLAY_HTTP")
        .ok()
        .as_deref()
        .map(|v| v == "0" || v.eq_ignore_ascii_case("false"))
        .unwrap_or(false)
    {
        return false;
    }
    explicit || test || appliance
}

/// Spawn the HTTP server on the current Tokio runtime.
pub async fn spawn(api: ControlApi, port: u16) -> anyhow::Result<()> {
    let state = Arc::new(HttpState { api });
    let app = router(state);
    let addr = bind_addr(port);
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "Loopback HTTP API listening (acceptance / diagnostics)");
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!(error = %e, "HTTP API server exited");
        }
    });
    Ok(())
}
