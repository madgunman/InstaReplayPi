//! Live input FPS and drop counters from a GStreamer pad probe on the capture tee.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use gstreamer as gst;
use gstreamer::prelude::*;

#[derive(Debug)]
pub struct VideoStats {
    frames_window: AtomicU64,
    drops: AtomicU64,
    window_start: Mutex<Instant>,
}

impl Default for VideoStats {
    fn default() -> Self {
        Self {
            frames_window: AtomicU64::new(0),
            drops: AtomicU64::new(0),
            window_start: Mutex::new(Instant::now()),
        }
    }
}

impl VideoStats {
    pub fn record_buffer(&self) {
        self.frames_window.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_drop(&self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }

    /// Rolling FPS over the last ~1s window; drop count is cumulative since live start.
    pub fn snapshot(&self) -> (f64, u64) {
        let mut start = self.window_start.lock().unwrap();
        let elapsed = start.elapsed().as_secs_f64();
        let frames = self.frames_window.swap(0, Ordering::Relaxed);
        if elapsed >= 1.0 {
            *start = Instant::now();
            let fps = frames as f64 / elapsed;
            (fps, self.drops.load(Ordering::Relaxed))
        } else if elapsed > 0.0 {
            (frames as f64 / elapsed, self.drops.load(Ordering::Relaxed))
        } else {
            (0.0, self.drops.load(Ordering::Relaxed))
        }
    }

    pub fn reset(&self) {
        self.frames_window.store(0, Ordering::Relaxed);
        self.drops.store(0, Ordering::Relaxed);
        *self.window_start.lock().unwrap() = Instant::now();
    }
}

pub fn attach_tee_probe(pipeline: &gst::Pipeline, stats: std::sync::Arc<VideoStats>) -> anyhow::Result<()> {
    let tee = pipeline
        .by_name("capture_tee")
        .ok_or_else(|| anyhow::anyhow!("missing capture_tee"))?;
    let pad = tee
        .pads()
        .into_iter()
        .find(|p| p.direction() == gst::PadDirection::Src)
        .ok_or_else(|| anyhow::anyhow!("capture_tee has no src pad"))?;

    let stats_probe = stats.clone();
    pad.add_probe(gst::PadProbeType::BUFFER, move |_, info| {
        if let Some(gst::PadProbeData::Buffer(_)) = info.data {
            stats_probe.record_buffer();
        }
        gst::PadProbeReturn::Ok
    });

    if let Some(live_queue) = pipeline.by_name("live_queue") {
        if let Some(src) = live_queue.static_pad("src") {
            let stats_drop = stats.clone();
            src.add_probe(gst::PadProbeType::BUFFER, move |_, info| {
                if let Some(gst::PadProbeData::Buffer(ref buf)) = info.data.as_ref() {
                    if buf.flags().contains(gst::BufferFlags::DISCONT) {
                        stats_drop.record_drop();
                    }
                }
                gst::PadProbeReturn::Ok
            });
        }
    }

    Ok(())
}
