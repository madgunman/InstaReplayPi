//! Build replay bins that feed the capture pipeline's input-selector (same program sink).

use std::path::PathBuf;

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use tracing::debug;

/// Build a replay bin and its output pad (link to `input-selector` sink_1).
pub fn build_replay_bin(
    segments: &[PathBuf],
    rate: f64,
) -> Result<(gst::Bin, gst::Pad)> {
    if segments.is_empty() {
        anyhow::bail!("no segments to replay");
    }

    let desc = build_replay_description(segments)?;
    debug!(pipeline = %desc, rate, segments = segments.len(), "Replay bin");

    // `ghost_unless=true` adds a ghost src pad for unlinked `replay_out` — do not add a second one.
    let bin = gst::parse::bin_from_description(&desc, true).context("parse replay bin")?;
    let src_pad = bin
        .static_pad("src")
        .context("replay bin missing ghost src pad")?;

    Ok((bin, src_pad))
}

/// Seek replay bin to mark offset with playback rate (element must be PAUSED or PLAYING).
pub fn seek_replay(
    element: &gst::Element,
    rate: f64,
    offset_ms: u64,
    allow_position_seek: bool,
) -> Result<()> {
    let rate = rate.clamp(0.25, 1.0);
    let need_rate = (rate - 1.0).abs() > f64::EPSILON;
    let need_position = offset_ms > 0 && allow_position_seek;

    if !need_rate && !need_position {
        return Ok(());
    }

    let (start_type, start) = if need_position {
        (
            gst::SeekType::Set,
            gst::ClockTime::from_nseconds((offset_ms as u64).saturating_mul(1_000_000)),
        )
    } else {
        (gst::SeekType::Set, gst::ClockTime::ZERO)
    };

    let seek = gst::event::Seek::new(
        rate,
        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
        start_type,
        start,
        gst::SeekType::None,
        gst::ClockTime::NONE,
    );
    if !element.send_event(seek) {
        anyhow::bail!("replay seek event was not handled");
    }
    Ok(())
}

fn build_replay_description(segments: &[PathBuf]) -> Result<String> {
    if segments.len() == 1 {
        let loc = path_for_gst(&segments[0]);
        return Ok(format!(
            "filesrc location=\"{loc}\" ! matroskademux ! h264parse ! avdec_h264 \
             ! videoconvert ! queue name=replay_out"
        ));
    }

    let mut desc = String::new();
    for seg in segments {
        let loc = path_for_gst(seg);
        desc.push_str(&format!(
            "filesrc location=\"{loc}\" ! matroskademux ! h264parse ! queue ! con. "
        ));
    }
    desc.push_str(
        "concat name=con ! h264parse ! avdec_h264 ! videoconvert ! queue name=replay_out",
    );
    Ok(desc)
}

fn path_for_gst(path: &PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}
