use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use replay_core::buffer::ChunkIndex;
use replay_core::config::AppConfig;
use tracing::{error, info, warn};

use crate::chunk_index_actor::{ChunkIndexHandle, ChunkIndexWorker};
use crate::video_stats::{self, VideoStats};

pub struct CapturePipeline {
    pipeline: gst::Pipeline,
    chunk_worker: ChunkIndexWorker,
    video_stats: Arc<VideoStats>,
    replay_bin: Option<gst::Bin>,
    replay_active: Arc<AtomicBool>,
    replay_finished_tx: Option<Sender<()>>,
    signal_lost_tx: Option<Sender<()>>,
    signal_restored_tx: Option<Sender<()>>,
    had_signal_error: bool,
    #[allow(dead_code)]
    bus_watch: Option<gst::bus::BusWatchGuard>,
}

impl CapturePipeline {
    pub fn build(
        config: &AppConfig,
        window_handle: usize,
        headless: bool,
        replay_finished_tx: Option<Sender<()>>,
        signal_lost_tx: Option<Sender<()>>,
        signal_restored_tx: Option<Sender<()>>,
        buffer_secs_cache: Arc<AtomicU64>,
    ) -> Result<Self> {
        gst::init().context("GStreamer init")?;

        let buffer_path = config.storage.buffer_path.clone();
        std::fs::create_dir_all(&buffer_path)?;

        let chunk_worker = ChunkIndexWorker::spawn(
            ChunkIndex::new(
                buffer_path.clone(),
                config.replay.buffer_seconds,
                config.replay.chunk_seconds,
            ),
            buffer_secs_cache.clone(),
        );
        let video_stats = Arc::new(VideoStats::default());

        let (width, height) = config.parse_resolution().unwrap_or((1920, 1080));
        let fps = config.input.fps.max(1);

        let pipeline_desc = build_pipeline_description(
            &config.input.device_id,
            width,
            height,
            fps,
            &config.input.pixel_format,
            config.output.show_status_overlay,
            headless,
            &buffer_path,
            config.replay.chunk_seconds,
            config.replay.buffer_seconds,
        );

        info!(pipeline = %pipeline_desc, "Building capture pipeline");

        let element = gst::parse::launch(&pipeline_desc).map_err(|e| {
            anyhow::anyhow!("parse capture pipeline: {e}\npipeline={pipeline_desc}")
        })?;
        let pipeline = element
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("expected gst::Pipeline"))?;

        if !headless {
            super::program_sink::attach_window_handle(&pipeline, "program_sink", window_handle)?;
        }

        // Ensure replay pad exists on selector (unlinked until replay starts).
        if let Some(sel) = pipeline.by_name("outsel") {
            let _ = sel.request_pad_simple("sink_1");
        }

        let bus = pipeline.bus().context("pipeline bus")?;
        let index_for_bus = chunk_worker.handle();
        let replay_finished_tx_bus = replay_finished_tx.clone();
        let signal_lost_tx_bus = signal_lost_tx.clone();
        let signal_restored_tx_bus = signal_restored_tx.clone();
        let replay_active = Arc::new(AtomicBool::new(false));
        let replay_active_bus = replay_active.clone();

        let bus_watch = Some(bus.add_watch(move |_, msg| {
            use gst::MessageView;

            if let MessageView::Element(elem) = msg.view() {
                if let Some(s) = elem.structure() {
                    crate::chunk_registry::handle_element_message(s, &index_for_bus);
                }
            }

            match msg.view() {
                MessageView::Eos(_) => {
                    if replay_active_bus.load(Ordering::SeqCst) {
                        if let Some(ref replay_tx) = replay_finished_tx_bus.as_ref() {
                            replay_active_bus.store(false, Ordering::SeqCst);
                            info!("Replay EOS — returning to live");
                            let _ = replay_tx.send(());
                        }
                    }
                }
                MessageView::Error(err) => {
                    let src_path = err.src().map(|s| s.path_string());
                    error!(
                        src = ?src_path,
                        error = %err.error(),
                        debug = ?err.debug(),
                        "Capture pipeline error"
                    );
                    if is_capture_source_error(&src_path) {
                        if let Some(tx) = signal_lost_tx_bus.as_ref() {
                            let _ = tx.send(());
                        }
                    }
                }
                MessageView::Warning(w) => {
                    let src_path = w.src().map(|s| s.path_string());
                    warn!(error = %w.error(), src = ?src_path, "Capture pipeline warning");
                    if is_capture_source_error(&src_path) {
                        if let Some(tx) = signal_lost_tx_bus.as_ref() {
                            let _ = tx.send(());
                        }
                    }
                }
                MessageView::StateChanged(state) => {
                    if let Some(src) = msg.src() {
                        if is_capture_source_element(&src)
                            && state.current() == gst::State::Playing
                            && state.old() < gst::State::Playing
                        {
                            if let Some(tx) = signal_restored_tx_bus.as_ref() {
                                let _ = tx.send(());
                            }
                        }
                    }
                }
                _ => {}
            }
            glib::ControlFlow::Continue
        })?);

        Ok(Self {
            pipeline,
            chunk_worker,
            video_stats,
            replay_bin: None,
            replay_active,
            replay_finished_tx,
            signal_lost_tx,
            signal_restored_tx,
            had_signal_error: false,
            bus_watch,
        })
    }

    pub fn start(&self) -> Result<()> {
        self.pipeline
            .set_state(gst::State::Playing)
            .context("start capture pipeline")?;
        let _ = video_stats::attach_tee_probe(&self.pipeline, self.video_stats.clone());
        super::program_sink::switch_to_live(&self.pipeline)?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.stop_replay()?;
        self.pipeline
            .set_state(gst::State::Null)
            .context("stop capture pipeline")?;
        Ok(())
    }

    pub fn chunk_index(&self) -> ChunkIndexHandle {
        self.chunk_worker.handle()
    }

    pub fn video_stats(&self) -> Arc<VideoStats> {
        self.video_stats.clone()
    }

    pub fn pipeline(&self) -> &gst::Pipeline {
        &self.pipeline
    }

    pub fn start_replay(
        &mut self,
        segments: Vec<PathBuf>,
        rate: f64,
        first_offset_ms: u64,
        show_overlay: bool,
    ) -> Result<()> {
        self.stop_replay()?;
        if segments.is_empty() {
            anyhow::bail!("no segments to replay");
        }

        let segment_count = segments.len();
        let (replay_pipe, src_pad) = super::replay::build_replay_bin(&segments, rate)?;
        let replay_cleanup = replay_pipe.clone();
        self.pipeline.add(&replay_pipe)?;

        let sel = self
            .pipeline
            .by_name("outsel")
            .context("missing outsel")?;
        let sink_pad = sel
            .static_pad("sink_1")
            .or_else(|| sel.request_pad_simple("sink_1"))
            .context("missing outsel sink_1")?;

        let start_result = (|| -> Result<()> {
            src_pad.link(&sink_pad).context("link replay to outsel")?;

            replay_pipe.set_state(gst::State::Paused)?;
            let _ = replay_pipe.state(gst::ClockTime::from_seconds(5));

            // Position seek is reliable on a single file; concat uses rate-only from segment start.
            let allow_position = segment_count == 1;
            if first_offset_ms > 0 && !allow_position {
                warn!(
                    offset_ms = first_offset_ms,
                    segments = segment_count,
                    "Mark offset skipped for multi-segment replay (plays from first segment)"
                );
            }
            match super::replay::seek_replay(
                replay_pipe.upcast_ref(),
                rate,
                first_offset_ms,
                allow_position,
            ) {
                Ok(()) => {}
                Err(e) if allow_position => return Err(e.context("replay seek failed")),
                Err(e) => {
                    warn!(
                        error = %e,
                        rate,
                        "Replay rate/seek not applied — playing at 1.0×"
                    );
                }
            }

            replay_pipe.set_state(gst::State::Playing)?;
            let _ = replay_pipe.state(gst::ClockTime::from_seconds(5));
            super::program_sink::switch_to_replay(&self.pipeline)?;

            if show_overlay {
                super::program_sink::set_status_overlay(&self.pipeline, "REPLAY")?;
            }

            self.replay_active.store(true, Ordering::SeqCst);
            self.replay_bin = Some(replay_pipe);
            info!(
                segments = segment_count,
                rate,
                offset_ms = first_offset_ms,
                position_seek = allow_position,
                "Replay started on program sink"
            );
            Ok(())
        })();

        if start_result.is_err() {
            if let (Some(outsel), Some(src)) = (
                self.pipeline.by_name("outsel"),
                replay_cleanup.static_pad("src"),
            ) {
                if let Some(sink) = outsel.static_pad("sink_1") {
                    let _ = src.unlink(&sink);
                }
            }
            let _ = replay_cleanup.set_state(gst::State::Null);
            let _ = self.pipeline.remove(&replay_cleanup);
            self.replay_active.store(false, Ordering::SeqCst);
            let _ = self.stop_replay();
            let _ = super::program_sink::switch_to_live(&self.pipeline);
        }
        start_result
    }

    pub fn stop_replay(&mut self) -> Result<()> {
        self.replay_active.store(false, Ordering::SeqCst);
        if let Some(replay_pipe) = self.replay_bin.take() {
            if let (Some(outsel), Some(src)) = (
                self.pipeline.by_name("outsel"),
                replay_pipe.static_pad("src"),
            ) {
                if let Some(sink) = outsel.static_pad("sink_1") {
                    let _ = src.unlink(&sink);
                }
            }
            let _ = replay_pipe.set_state(gst::State::Null);
            self.pipeline.remove(&replay_pipe)?;
        }
        Ok(())
    }

    pub fn return_to_live(&mut self, show_overlay: bool) -> Result<()> {
        self.stop_replay()?;
        super::program_sink::switch_to_live(&self.pipeline)?;
        if show_overlay {
            super::program_sink::set_status_overlay(&self.pipeline, "LIVE")?;
        }
        info!("Returned to live on program sink");
        Ok(())
    }

    pub fn on_signal_lost(&mut self, show_overlay: bool) -> Result<()> {
        if !self.had_signal_error {
            self.had_signal_error = true;
            if show_overlay {
                super::program_sink::set_status_overlay_black(&self.pipeline)?;
            }
        }
        Ok(())
    }

    pub fn on_signal_restored(&mut self, show_overlay: bool) -> Result<()> {
        if self.had_signal_error {
            self.had_signal_error = false;
            if show_overlay {
                super::program_sink::set_status_overlay(&self.pipeline, "LIVE")?;
            }
        }
        Ok(())
    }

    /// Legacy helpers used by runtime during migration.
    pub fn pause_live_branch(&self) -> Result<()> {
        super::program_sink::set_live_valve_open(&self.pipeline, false)
    }

    pub fn resume_live_branch(&self) -> Result<()> {
        super::program_sink::switch_to_live(&self.pipeline)
    }

    pub fn set_status_overlay(&self, text: &str) -> Result<()> {
        super::program_sink::set_status_overlay(&self.pipeline, text)
    }
}

fn build_pipeline_description(
    device_id: &str,
    width: u32,
    height: u32,
    fps: u32,
    pixel_format: &str,
    show_status_overlay: bool,
    headless: bool,
    buffer_path: &PathBuf,
    chunk_seconds: u32,
    buffer_seconds: u32,
) -> String {
    let location_pattern = buffer_path
        .join("chunk_%05d.mkv")
        .to_string_lossy()
        .replace('\\', "/");

    let source = super::capture_source::build_source_element(
        device_id, width, height, fps, pixel_format,
    );
    let chunk_ns = chunk_seconds as u64 * 1_000_000_000;
    let max_files = (buffer_seconds / chunk_seconds.max(1)) + 2;
    let encoder = buffer_encoder_element();
    let output = if headless {
        super::program_sink::headless_output_chain(show_status_overlay)
    } else {
        super::program_sink::program_output_chain(show_status_overlay)
    };

    format!(
        "{source} ! videoconvert ! videoscale ! video/x-raw,width={width},height={height},framerate={fps}/1 \
         ! tee name=capture_tee \
         capture_tee. ! queue name=live_queue leaky=downstream max-size-time=50000000 \
         ! valve name=live_valve drop=false \
         ! videoconvert ! {output} \
         capture_tee. ! queue max-size-buffers=120 max-size-time=0 \
         ! videoconvert ! {encoder} ! h264parse \
         ! splitmuxsink name=splitmux location=\"{location_pattern}\" muxer=matroskamux \
         max-size-time={chunk_ns} max-files={max_files} async-finalize=false send-keyframe-requests=true",
        source = source,
        width = width,
        height = height,
        fps = fps,
        encoder = encoder,
        output = output,
        chunk_ns = chunk_ns,
        max_files = max_files,
        location_pattern = location_pattern,
    )
}

fn buffer_encoder_element() -> &'static str {
    // x264enc works headless in CI/dev; vtenc can fail with permission errors without entitlements.
    "x264enc speed-preset=ultrafast tune=zerolatency key-int-max=60"
}

fn is_capture_source_error(src_path: &Option<glib::GString>) -> bool {
    let Some(path) = src_path else {
        return false;
    };
    let p = path.as_str();
    p.contains("avfvideosrc")
        || p.contains("v4l2src")
        || p.contains("ksvideosrc")
        || p.contains("dshowvideosrc")
        || p.contains("videotestsrc")
}

fn is_capture_source_element(obj: &gst::Object) -> bool {
    let p = obj.path_string();
    p.contains("avfvideosrc")
        || p.contains("v4l2src")
        || p.contains("ksvideosrc")
        || p.contains("dshowvideosrc")
        || p.contains("videotestsrc")
}
