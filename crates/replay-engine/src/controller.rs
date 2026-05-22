use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result};
use replay_core::buffer::{ChunkIndex, MIN_REPLAY_BUFFER_SECS};
use replay_core::config::AppConfig;
use replay_core::fsm::{ReplayEvent, ReplayFsm, ReplayState};
use serde::Serialize;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::chunk_index_actor::ChunkIndexHandle;
use crate::program_output::should_use_headless;
use crate::runtime::GstreamerRuntime;
use crate::storage_monitor::StorageMonitor;
use crate::video_stats::VideoStats;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    pub input_fps: f64,
    pub dropped_frames: u64,
    pub buffer_seconds_available: f64,
    pub disk_warning: bool,
    pub buffer_error: bool,
    pub current_state: String,
    pub last_error: String,
    pub replay_trigger_delay_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusSnapshot {
    #[serde(rename = "state")]
    pub state: ReplayState,
    pub input_fps: f64,
    pub dropped_frames: u64,
    pub buffer_seconds_available: f64,
    pub disk_warning: bool,
    pub last_error: String,
    pub buffer_ready: bool,
    pub buffer_error: bool,
    pub mark_timestamp_ns: i64,
    pub sequence: u64,
}

pub struct EngineController {
    test_mode: bool,
    config: Arc<RwLock<AppConfig>>,
    fsm: Arc<RwLock<ReplayFsm>>,
    runtime: Arc<GstreamerRuntime>,
    chunk_index: Arc<Mutex<Option<ChunkIndexHandle>>>,
    mark_timestamp_ns_cache: Arc<AtomicI64>,
    video_stats: Arc<Mutex<Option<Arc<VideoStats>>>>,
    status_tx: broadcast::Sender<StatusSnapshot>,
    diagnostics: Arc<RwLock<Diagnostics>>,
    sequence: AtomicU64,
    storage_monitor: Arc<Mutex<Option<StorageMonitor>>>,
    /// Updated from the GStreamer bus when fragments close (lock-free for gRPC).
    buffer_secs_cache: Arc<AtomicU64>,
    replay_finished_tx: mpsc::Sender<()>,
    signal_lost_tx: mpsc::Sender<()>,
    signal_restored_tx: mpsc::Sender<()>,
}

/// Receivers for pipeline events — consumed by `spawn_event_handlers`.
pub struct EngineEventReceivers {
    pub replay_finished: mpsc::Receiver<()>,
    pub signal_lost: mpsc::Receiver<()>,
    pub signal_restored: mpsc::Receiver<()>,
}

impl EngineController {
    pub fn new(config: AppConfig, test_mode: bool) -> (Self, EngineEventReceivers) {
        let (status_tx, _) = broadcast::channel(64);
        let (replay_finished_tx, replay_finished_rx) = mpsc::channel();
        let (signal_lost_tx, signal_lost_rx) = mpsc::channel();
        let (signal_restored_tx, signal_restored_rx) = mpsc::channel();

        let controller = Self {
            test_mode,
            config: Arc::new(RwLock::new(config)),
            fsm: Arc::new(RwLock::new(ReplayFsm::new())),
            runtime: Arc::new(GstreamerRuntime::spawn(should_use_headless(test_mode))),
            chunk_index: Arc::new(Mutex::new(None)),
            mark_timestamp_ns_cache: Arc::new(AtomicI64::new(0)),
            video_stats: Arc::new(Mutex::new(None)),
            status_tx,
            diagnostics: Arc::new(RwLock::new(Diagnostics {
                input_fps: 0.0,
                dropped_frames: 0,
                buffer_seconds_available: 0.0,
                disk_warning: false,
                buffer_error: false,
                current_state: ReplayState::Starting.as_str().into(),
                last_error: String::new(),
                replay_trigger_delay_ms: 0.0,
            })),
            sequence: AtomicU64::new(0),
            storage_monitor: Arc::new(Mutex::new(None)),
            buffer_secs_cache: Arc::new(AtomicU64::new(0)),
            replay_finished_tx,
            signal_lost_tx,
            signal_restored_tx,
        };
        let receivers = EngineEventReceivers {
            replay_finished: replay_finished_rx,
            signal_lost: signal_lost_rx,
            signal_restored: signal_restored_rx,
        };
        (controller, receivers)
    }

    /// Bridge GStreamer thread events into async FSM updates (call once from `main`).
    pub fn spawn_event_handlers(
        controller: Arc<Self>,
        receivers: EngineEventReceivers,
        handle: tokio::runtime::Handle,
    ) {
        let c_replay = controller.clone();
        let h_replay = handle.clone();
        std::thread::spawn(move || {
            let mut rx = receivers.replay_finished;
            while rx.recv().is_ok() {
                let c = c_replay.clone();
                h_replay.spawn(async move {
                    c.on_replay_finished().await;
                });
            }
        });
        let c_lost = controller.clone();
        let h_lost = handle.clone();
        std::thread::spawn(move || {
            let mut rx = receivers.signal_lost;
            while rx.recv().is_ok() {
                let c = c_lost.clone();
                h_lost.spawn(async move {
                    c.on_pipeline_signal_lost().await;
                });
            }
        });
        let c_ok = controller.clone();
        let h_ok = handle.clone();
        std::thread::spawn(move || {
            let mut rx = receivers.signal_restored;
            while rx.recv().is_ok() {
                let c = c_ok.clone();
                h_ok.spawn(async move {
                    c.on_pipeline_signal_restored().await;
                });
            }
        });
    }

    pub fn subscribe_status(&self) -> broadcast::Receiver<StatusSnapshot> {
        self.status_tx.subscribe()
    }

    pub async fn config(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    pub async fn set_config(&self, config: AppConfig) -> Result<()> {
        *self.config.write().await = config.clone();
        config.save().map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    }

    pub async fn start_live(
        &self,
        device_id: String,
        width: u32,
        height: u32,
        fps: u32,
        pixel_format: String,
        display_id: u32,
        fullscreen: bool,
    ) -> Result<()> {
        let mut cfg = self.config.write().await;
        cfg.input.device_id = device_id;
        cfg.input.resolution = format!("{width}x{height}");
        cfg.input.fps = fps;
        cfg.input.pixel_format = pixel_format;
        cfg.output.display_id = display_id;
        cfg.output.fullscreen = fullscreen;
        if let Err(e) = cfg.save() {
            warn!(error = %e, "Config save skipped (install /etc/instant-replay/config.toml on Pi)");
        }

        if cfg.storage.auto_clean_on_start {
            let path = cfg.storage.buffer_path.clone();
            if let Err(e) = std::fs::remove_dir_all(&path) {
                warn!(path = %path.display(), error = %e, "auto_clean remove_dir_all failed");
            }
            if let Err(e) = std::fs::create_dir_all(&path) {
                warn!(path = %path.display(), error = %e, "auto_clean create_dir_all failed");
            }
        }
        let config = cfg.clone();
        drop(cfg);
        let buffer_path = config.storage.buffer_path.clone();

        self.buffer_secs_cache.store(0, Ordering::Relaxed);
        let runtime = self.runtime.clone();
        let replay_finished_tx = Some(self.replay_finished_tx.clone());
        let signal_lost_tx = Some(self.signal_lost_tx.clone());
        let signal_restored_tx = Some(self.signal_restored_tx.clone());
        let buffer_secs_cache = self.buffer_secs_cache.clone();
        let start_config = config.clone();
        let handles = tokio::task::spawn_blocking(move || {
            runtime.start_live(
                start_config,
                width,
                height,
                fps,
                replay_finished_tx,
                signal_lost_tx,
                signal_restored_tx,
                buffer_secs_cache,
            )
        })
        .await
        .map_err(|e| anyhow::anyhow!("start_live task: {e}"))??;
        *self.chunk_index.lock().unwrap() = Some(handles.chunk_index.clone());
        self.mark_timestamp_ns_cache.store(0, Ordering::Relaxed);
        *self.video_stats.lock().unwrap() = Some(handles.video_stats);
        *self.storage_monitor.lock().unwrap() = Some(StorageMonitor::spawn(buffer_path));

        self.fsm_apply_blocking(ReplayEvent::InputReady).await?;

        self.update_buffer_metrics().await;
        self.publish_status().await;
        info!("Live capture started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let runtime = self.runtime.clone();
        tokio::task::spawn_blocking(move || runtime.stop())
            .await
            .map_err(|e| anyhow::anyhow!("stop task: {e}"))??;
        *self.chunk_index.lock().unwrap() = None;
        self.mark_timestamp_ns_cache.store(0, Ordering::Relaxed);
        *self.video_stats.lock().unwrap() = None;
        Ok(())
    }

    pub fn signal_lost_notify(&self) {
        let _ = self.signal_lost_tx.send(());
    }

    pub fn signal_restored_notify(&self) {
        let _ = self.signal_restored_tx.send(());
    }

    fn chunk_handle(&self) -> Result<ChunkIndexHandle> {
        self.chunk_index
            .lock()
            .map_err(|_| anyhow::anyhow!("chunk index lock poisoned"))?
            .clone()
            .context("capture not running")
    }

    async fn chunk_index_mut<R, F>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut ChunkIndex) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        self.chunk_handle()?.with_index(f).await
    }

    /// FSM updates via blocking_write — avoids starving async handlers when `publish_status` holds a read lock.
    async fn fsm_apply_blocking(&self, event: ReplayEvent) -> Result<ReplayState> {
        let fsm = self.fsm.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = fsm.blocking_write();
            guard
                .apply(event)
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .await
        .map_err(|e| anyhow::anyhow!("fsm task: {e}"))?
    }

    async fn fsm_state_blocking(&self) -> ReplayState {
        let fsm = self.fsm.clone();
        tokio::task::spawn_blocking(move || fsm.blocking_read().state())
            .await
            .unwrap_or(ReplayState::ErrorRecovery)
    }

    fn buffer_seconds_available(&self) -> f64 {
        f64::from_bits(self.buffer_secs_cache.load(Ordering::Relaxed))
    }

    fn buffer_ready(&self) -> bool {
        self.buffer_seconds_available() > 0.0
    }

    pub fn capture_running(&self) -> bool {
        self.chunk_index
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    pub async fn engine_ready(&self) -> bool {
        if !self.capture_running() {
            return false;
        }
        let state = self.fsm_state_blocking().await;
        if self.diagnostics.read().await.buffer_error {
            return false;
        }
        matches!(
            state,
            ReplayState::Live | ReplayState::Marked | ReplayState::Replaying
        )
    }

    pub async fn mark(&self) -> Result<i64> {
        let handle = self.chunk_handle()?;
        if !self.buffer_ready() {
            anyhow::bail!("buffer not ready — wait for recording before marking");
        }
        let buffer_mark = handle
            .snapshot_mark()
            .map_err(|msg| anyhow::anyhow!("{msg}"))?;
        handle.post_set_mark(buffer_mark.clone());

        let mark_ns = buffer_mark.unix_ms as i64 * 1_000_000;
        self.mark_timestamp_ns_cache
            .store(mark_ns, Ordering::Relaxed);

        self.fsm_apply_blocking(ReplayEvent::Mark).await?;

        info!(
            chunk_id = %buffer_mark.chunk_id,
            offset_ms = buffer_mark.offset_ms,
            "Mark set on buffer timeline"
        );
        Ok(mark_ns)
    }

    pub async fn clear_mark(&self) -> Result<()> {
        let _ = self
            .chunk_index_mut(|idx| {
                idx.clear_mark();
                idx.persist_index_background();
                Ok(())
            })
            .await;
        self.mark_timestamp_ns_cache.store(0, Ordering::Relaxed);
        self.fsm_apply_blocking(ReplayEvent::ClearMark).await?;
        self.publish_status().await;
        Ok(())
    }

    pub async fn replay_from_mark_or_last(&self) -> Result<()> {
        let cfg = self.config.read().await.clone();
        let mode = cfg.replay.mode.as_str();

        let (segments, offset_ms) = if mode == "last" {
            (
                self.get_segments_last(cfg.replay.default_replay_seconds)
                    .await?,
                0,
            )
        } else {
            let has_mark = self.mark_timestamp_ns_cache.load(Ordering::Relaxed) > 0
                || self.fsm_state_blocking().await == ReplayState::Marked;
            if has_mark {
                self.get_segments_and_offset_from_mark().await?
            } else {
                (
                    self.get_segments_last(cfg.replay.default_replay_seconds)
                        .await?,
                    0,
                )
            }
        };

        self.run_replay(segments, cfg.replay.speed, offset_ms).await
    }

    pub async fn replay_last(&self, seconds: u32) -> Result<()> {
        let cfg = self.config.read().await.clone();
        let secs = if seconds == 0 {
            cfg.replay.default_replay_seconds
        } else {
            seconds
        };
        let segments = self.get_segments_last(secs).await?;
        self.run_replay(segments, cfg.replay.speed, 0).await
    }

    async fn run_replay(
        &self,
        segments: Vec<std::path::PathBuf>,
        rate: f64,
        first_offset_ms: u64,
    ) -> Result<()> {
        if segments.is_empty() {
            anyhow::bail!("no segments to replay");
        }
        let buffer_secs = self.buffer_seconds_available();
        if buffer_secs < MIN_REPLAY_BUFFER_SECS {
            anyhow::bail!(
                "buffer has {buffer_secs:.1}s available, need at least {MIN_REPLAY_BUFFER_SECS:.1}s before replay"
            );
        }

        let prior_state = self.fsm_state_blocking().await;
        let trigger_start = Instant::now();

        let event = if self.mark_timestamp_ns_cache.load(Ordering::Relaxed) > 0 {
            ReplayEvent::Replay
        } else {
            ReplayEvent::ReplayLast
        };
        self.fsm_apply_blocking(event).await?;

        let runtime = self.runtime.clone();
        let replay_result = tokio::task::spawn_blocking(move || {
            runtime.replay_segments(segments, rate, first_offset_ms)
        })
        .await
        .map_err(|e| anyhow::anyhow!("replay_segments task: {e}"));

        match replay_result {
            Ok(Ok(())) => {
                let delay = trigger_start.elapsed().as_secs_f64() * 1000.0;
                self.diagnostics.write().await.replay_trigger_delay_ms = delay;
                self.publish_status().await;
                Ok(())
            }
            Ok(Err(e)) | Err(e) => {
                let msg = e.to_string();
                warn!(error = %msg, prior_state = ?prior_state, "Replay failed — rolling back FSM");
                let runtime = self.runtime.clone();
                let _ = tokio::task::spawn_blocking(move || runtime.return_live()).await;
                self.rollback_fsm_to(prior_state).await;
                {
                    let mut diag = self.diagnostics.write().await;
                    diag.last_error = msg.clone();
                }
                self.publish_status().await;
                Err(e)
            }
        }
    }

    async fn rollback_fsm_to(&self, state: ReplayState) {
        let fsm = self.fsm.clone();
        let _ = tokio::task::spawn_blocking(move || {
            fsm.blocking_write().force_state(state);
        })
        .await;
    }

    pub async fn return_live(&self) -> Result<()> {
        let runtime = self.runtime.clone();
        tokio::task::spawn_blocking(move || runtime.return_live())
            .await
            .map_err(|e| anyhow::anyhow!("return_live task: {e}"))??;
        let _ = self.fsm_apply_blocking(ReplayEvent::ReturnLive).await;
        self.publish_status().await;
        info!("Returned to live");
        Ok(())
    }

    pub async fn signal_lost(&self) {
        self.on_pipeline_signal_lost().await;
    }

    pub async fn signal_restored(&self) {
        self.on_pipeline_signal_restored().await;
    }

    async fn on_pipeline_signal_lost(&self) {
        let runtime = self.runtime.clone();
        let _ = tokio::task::spawn_blocking(move || runtime.notify_signal_lost())
            .await;
        let _ = self.fsm_apply_blocking(ReplayEvent::InputLost).await;
        self.diagnostics.write().await.last_error = "Input signal lost".into();
        self.publish_status().await;
    }

    async fn on_pipeline_signal_restored(&self) {
        let runtime = self.runtime.clone();
        let _ = tokio::task::spawn_blocking(move || runtime.notify_signal_restored())
            .await;
        if self.fsm_state_blocking().await == ReplayState::NoSignal {
            let _ = self.fsm_apply_blocking(ReplayEvent::InputReady).await;
        }
        self.diagnostics.write().await.last_error.clear();
        self.publish_status().await;
    }

    async fn get_segments_last(&self, seconds: u32) -> Result<Vec<std::path::PathBuf>> {
        let segs = self
            .chunk_index_mut(move |idx| Ok(idx.segments_last_seconds(seconds)))
            .await?;
        if segs.is_empty() {
            warn!("No buffer segments for replay last {seconds}s");
        }
        Ok(segs)
    }

    async fn get_segments_and_offset_from_mark(
        &self,
    ) -> Result<(Vec<std::path::PathBuf>, u64)> {
        let (segs, offset_ms) = self
            .chunk_index_mut(|idx| {
                let offset_ms = idx.mark().map(|m| m.offset_ms).unwrap_or(0);
                Ok((idx.segments_from_buffer_mark(), offset_ms))
            })
            .await?;
        if segs.is_empty() {
            warn!("No buffer segments from mark");
        }
        Ok((segs, offset_ms))
    }

    async fn update_buffer_metrics(&self) {
        let secs = f64::from_bits(self.buffer_secs_cache.load(Ordering::Relaxed));
        let mut diag = self.diagnostics.write().await;
        diag.buffer_seconds_available = secs;
        if let Some(stats) = self.video_stats.lock().unwrap().as_ref() {
            let (fps, drops) = stats.snapshot();
            diag.input_fps = fps;
            diag.dropped_frames = drops;
        }
        if let Some(mon) = self.storage_monitor.lock().unwrap().as_ref() {
            diag.disk_warning = mon.disk_warning();
            diag.buffer_error = mon.disk_full();
            if mon.disk_full() {
                diag.last_error = "Buffer disk full — recording stopped".into();
            } else if mon.slow_disk() && diag.last_error.is_empty() {
                diag.last_error = "Slow disk writes — check storage".into();
            }
        }
    }

    async fn check_storage_alarms(&self) {
        let disk_full = self
            .storage_monitor
            .lock()
            .unwrap()
            .as_ref()
            .map(|m| m.disk_full())
            .unwrap_or(false);
        if disk_full {
            let state = self.fsm_state_blocking().await;
            if matches!(state, ReplayState::Live | ReplayState::Marked) {
                warn!("Disk full — buffer stopped");
                let _ = self.fsm_apply_blocking(ReplayEvent::InputLost).await;
            }
        }
    }

    pub async fn get_diagnostics(&self) -> Diagnostics {
        let mut diag = self.diagnostics.read().await.clone();
        diag.buffer_seconds_available = self.buffer_seconds_available();
        diag.current_state = self.fsm_state_blocking().await.as_str().into();
        diag
    }

    pub fn mark_timestamp_ns(&self) -> i64 {
        self.mark_timestamp_ns_cache.load(Ordering::Relaxed)
    }

    pub async fn status_snapshot(&self) -> StatusSnapshot {
        self.update_buffer_metrics().await;
        let state = self.fsm_state_blocking().await;
        let mark_timestamp_ns = if state == ReplayState::Marked {
            self.mark_timestamp_ns_cache.load(Ordering::Relaxed)
        } else {
            0
        };
        let diag = self.diagnostics.read().await;
        StatusSnapshot {
            state,
            input_fps: diag.input_fps,
            dropped_frames: diag.dropped_frames,
            buffer_seconds_available: diag.buffer_seconds_available,
            disk_warning: diag.disk_warning,
            last_error: diag.last_error.clone(),
            buffer_ready: diag.buffer_seconds_available > 0.0,
            buffer_error: diag.buffer_error,
            mark_timestamp_ns,
            sequence: self.sequence.load(Ordering::Relaxed),
        }
    }

    pub async fn publish_status(&self) {
        self.update_buffer_metrics().await;
        self.check_storage_alarms().await;
        let state = self.fsm_state_blocking().await;
        let mark_timestamp_ns = if state == ReplayState::Marked {
            self.mark_timestamp_ns_cache.load(Ordering::Relaxed)
        } else {
            0
        };
        let mut diag = self.diagnostics.write().await;
        diag.current_state = state.as_str().into();
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let snap = StatusSnapshot {
            state,
            input_fps: diag.input_fps,
            dropped_frames: diag.dropped_frames,
            buffer_seconds_available: diag.buffer_seconds_available,
            disk_warning: diag.disk_warning,
            last_error: diag.last_error.clone(),
            buffer_ready: diag.buffer_seconds_available > 0.0,
            buffer_error: diag.buffer_error,
            mark_timestamp_ns,
            sequence: seq,
        };
        drop(diag);
        let _ = self.status_tx.send(snap);
    }

    pub async fn fsm_state(&self) -> ReplayState {
        self.fsm_state_blocking().await
    }

    pub fn test_mode(&self) -> bool {
        self.test_mode
    }

    pub fn list_displays(&self) -> Vec<replay_core::types::DisplayInfo> {
        crate::displays::list_displays(self.runtime.program_output())
    }

    pub async fn clean_buffer(&self) -> Result<()> {
        let path = self.config.read().await.storage.buffer_path.clone();
        ChunkIndex::new(path.clone(), 20, 1).clean_all();
        Ok(())
    }

    async fn on_replay_finished(&self) {
        if self.fsm_state_blocking().await != ReplayState::Replaying {
            return;
        }
        let runtime = self.runtime.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || runtime.return_live())
            .await
            .map_err(|e| anyhow::anyhow!("return_live task: {e}"))
            .and_then(|r| r)
        {
            warn!(error = %e, "return_live after replay EOS failed");
        }
        if self.fsm_state_blocking().await == ReplayState::Replaying {
            let _ = self.fsm_apply_blocking(ReplayEvent::ReplayFinished).await;
            let _ = self.fsm_apply_blocking(ReplayEvent::Recover).await;
        }
        self.publish_status().await;
        info!("Replay finished (EOS) — live restored");
    }
}
