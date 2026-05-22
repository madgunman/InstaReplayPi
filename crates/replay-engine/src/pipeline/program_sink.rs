//! Program output video sink (glimagesink) and window-handle attachment.

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_video::ffi as video_ffi;
use libc::uintptr_t;
use raw_window_handle::RawWindowHandle;
use tracing::warn;

/// GStreamer video sink + input-selector for live/replay switching on one window.
pub fn program_output_chain(show_overlay: bool) -> String {
    let sink = program_sink_element(show_overlay);
    format!("input-selector name=outsel ! {sink}")
}

/// Headless / CI / `--test`: no native window; discards frames with fakesink.
pub fn headless_output_chain(show_overlay: bool) -> String {
    let sink = if show_overlay {
        "videoconvert ! textoverlay name=status_overlay text=\"LIVE\" \
         valignment=top halignment=left font-desc=\"Sans Bold 28\" \
         ! fakesink name=program_sink sync=false"
    } else {
        "videoconvert ! fakesink name=program_sink sync=false"
    };
    format!("input-selector name=outsel ! {sink}")
}

/// GStreamer video sink element (after input-selector).
pub fn program_sink_element(show_overlay: bool) -> String {
    if show_overlay {
        "videoconvert ! textoverlay name=status_overlay text=\"LIVE\" \
         valignment=top halignment=left font-desc=\"Sans Bold 28\" \
         ! glimagesink name=program_sink sync=false"
            .to_string()
    } else {
        "videoconvert ! glimagesink name=program_sink sync=false".to_string()
    }
}

pub fn attach_window_handle(pipeline: &gst::Pipeline, sink_name: &str, handle: usize) -> Result<()> {
    let sink = pipeline
        .by_name(sink_name)
        .with_context(|| format!("missing sink {sink_name}"))?;
    unsafe {
        video_ffi::gst_video_overlay_set_window_handle(
            sink.as_ptr() as *mut video_ffi::GstVideoOverlay,
            handle as uintptr_t,
        );
    }
    Ok(())
}

pub fn set_live_valve_open(pipeline: &gst::Pipeline, open: bool) -> Result<()> {
    if let Some(valve) = pipeline.by_name("live_valve") {
        valve.set_property("drop", !open);
    }
    Ok(())
}

pub fn switch_to_live(pipeline: &gst::Pipeline) -> Result<()> {
    set_live_valve_open(pipeline, true)?;
    let sel = pipeline
        .by_name("outsel")
        .context("missing input-selector outsel")?;
    let pad = sel
        .static_pad("sink_0")
        .context("missing outsel sink_0 (live)")?;
    sel.set_property("active-pad", &pad);
    Ok(())
}

pub fn switch_to_replay(pipeline: &gst::Pipeline) -> Result<()> {
    set_live_valve_open(pipeline, false)?;
    let sel = pipeline
        .by_name("outsel")
        .context("missing input-selector outsel")?;
    let pad = sel
        .static_pad("sink_1")
        .or_else(|| sel.request_pad_simple("sink_1"))
        .context("missing outsel sink_1 (replay)")?;
    sel.set_property("active-pad", &pad);
    Ok(())
}

pub fn set_status_overlay(pipeline: &gst::Pipeline, text: &str) -> Result<()> {
    if let Some(overlay) = pipeline.by_name("status_overlay") {
        overlay.set_property("text", text);
    }
    Ok(())
}

pub fn set_status_overlay_black(pipeline: &gst::Pipeline) -> Result<()> {
    set_status_overlay(pipeline, "NO SIGNAL")?;
    Ok(())
}

/// Extract a platform window handle suitable for `gst_video_overlay_set_window_handle`.
pub fn window_handle_from_winit(window: &winit::window::Window) -> Result<usize> {
    use winit::raw_window_handle::HasWindowHandle;

    let handle = window
        .window_handle()
        .context("window handle not available yet")?;

    let raw = handle.as_raw();
    let ptr = match raw {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as usize,
        RawWindowHandle::Xlib(h) => h.window as usize,
        RawWindowHandle::Xcb(h) => h.window.get() as usize,
        RawWindowHandle::Wayland(_) => {
            warn!("Wayland raw handle may need waylandsink; trying glimagesink");
            0
        }
        RawWindowHandle::Win32(h) => h.hwnd.get() as usize,
        other => {
            anyhow::bail!("unsupported window handle type: {other:?}");
        }
    };

    if ptr == 0 {
        anyhow::bail!("null window handle");
    }
    Ok(ptr)
}
