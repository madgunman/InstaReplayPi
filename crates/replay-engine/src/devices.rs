use replay_core::types::{VideoDevice, VideoFormat};

use crate::capture_select;
use crate::format_probe;

#[derive(Debug, Clone)]
pub struct CaptureDevice {
    pub id: String,
    pub display_name: String,
    pub backend: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedDevice {
    Test,
    Auto,
    V4l2 { path: String },
    Default,
}

pub fn parse_device_id(device_id: &str) -> ParsedDevice {
    if device_id == "test" {
        return ParsedDevice::Test;
    }
    let d = device_id.trim().to_lowercase();
    if d == "auto" {
        return ParsedDevice::Auto;
    }
    if device_id == "default" || device_id.is_empty() {
        return ParsedDevice::Auto;
    }
    if let Some(path) = device_id.strip_prefix("v4l2:") {
        return ParsedDevice::V4l2 {
            path: path.to_string(),
        };
    }
    if device_id.starts_with("/dev/video") {
        return ParsedDevice::V4l2 {
            path: device_id.to_string(),
        };
    }
    ParsedDevice::Default
}

pub fn list_devices(test_mode: bool) -> Vec<CaptureDevice> {
    gstreamer::init().ok();
    let mut devices = Vec::new();

    devices.push(CaptureDevice {
        id: "test".into(),
        display_name: "Test Pattern (no camera)".into(),
        backend: "test".into(),
    });

    if !test_mode {
        devices.extend(capture_select::discover_capture_devices());
    }

    if devices.len() == 1 {
        tracing::warn!("No capture hardware detected; only test pattern available");
    }

    devices
}

pub fn list_formats(device_id: &str) -> Vec<VideoFormat> {
    format_probe::probe_formats(device_id)
}

pub fn to_video_devices(devices: &[CaptureDevice]) -> Vec<VideoDevice> {
    devices
        .iter()
        .map(|d| VideoDevice {
            id: d.id.clone(),
            display_name: d.display_name.clone(),
            backend: d.backend.clone(),
        })
        .collect()
}

pub fn live_start_error_hint(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("32768") {
        return "Invalid capture mode — unlock Setup, pick 1080p MJPEG".into();
    }
    if lower.contains("not-negotiated") || lower.contains("negotiat") {
        return "Format not supported — try MJPEG 1080p30 in Setup".into();
    }
    if lower.contains("no element") || lower.contains("no such element") {
        return "GStreamer plugin missing — run doctor-pi on the Pi".into();
    }
    if lower.contains("capture not running") {
        return "Capture not running — wait or unlock Setup to retry".into();
    }
    if lower.contains("permission") || lower.contains("denied") {
        return "Camera permission denied — add user to video group".into();
    }
    if lower.contains("no usb capture") || lower.contains("no capture device") {
        return "No USB capture device — plug in webcam or HDMI card".into();
    }
    raw.lines().next().unwrap_or(raw).trim().to_string()
}

/// Operator banner / toast — one line, max ~80 chars; full detail stays in journal.
pub fn short_operator_error(raw: &str) -> String {
    let hint = live_start_error_hint(raw);
    let line = hint.lines().next().unwrap_or(&hint).trim();
    if line.chars().count() <= 80 {
        line.to_string()
    } else {
        let truncated: String = line.chars().take(77).collect();
        format!("{truncated}…")
    }
}
