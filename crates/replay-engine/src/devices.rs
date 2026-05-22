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
    if lower.contains("permission") || lower.contains("denied") {
        return format!(
            "{raw}\n\nCheck UVC device permissions (groups: video) and unlock Setup to pick the camera."
        );
    }
    if lower.contains("not-negotiated") || lower.contains("negotiat") {
        return format!(
            "{raw}\n\nTry MJPEG, 1280x720 @ 30, or another device in Setup (PIN / long-press banner)."
        );
    }
    if lower.contains("no usb capture") || lower.contains("no capture device") {
        return format!("{raw}\n\nConnect a webcam or HDMI capture card on USB 3, then Refresh in Setup.");
    }
    raw.to_string()
}
