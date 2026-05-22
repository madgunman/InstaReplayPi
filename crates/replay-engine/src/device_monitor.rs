//! V4L2 / GStreamer device discovery for Raspberry Pi.

use gstreamer as gst;
use gstreamer::prelude::*;
use tracing::debug;

use crate::devices::CaptureDevice;

pub fn discover_capture_devices() -> Vec<CaptureDevice> {
    let _ = gst::init();

    let mut devices = from_device_monitor();
    merge_unique(&mut devices, probe_v4l2_sysfs());
    devices
}

fn from_device_monitor() -> Vec<CaptureDevice> {
    let monitor = gst::DeviceMonitor::new();
    monitor.add_filter(Some("Video/Source"), None);
    if monitor.start().is_err() {
        debug!("DeviceMonitor failed to start");
        return Vec::new();
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    monitor
        .devices()
        .iter()
        .filter_map(device_to_capture)
        .collect()
}

fn device_to_capture(device: &gst::Device) -> Option<CaptureDevice> {
    let display_name = device.display_name().to_string();
    if display_name.is_empty() {
        return None;
    }
    let (id, backend) = stable_id_from_device(device, &display_name)?;
    Some(CaptureDevice {
        id,
        display_name,
        backend,
    })
}

fn stable_id_from_device(device: &gst::Device, display_name: &str) -> Option<(String, String)> {
    if let Some(props) = device.properties() {
        if let Ok(path) = props.get::<String>("device.path") {
            if path.starts_with("/dev/video") {
                return Some((format!("v4l2:{path}"), "v4l2".into()));
            }
        }
    }
    let slug = display_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    Some((format!("v4l2:{slug}"), "v4l2".into()))
}

fn probe_v4l2_sysfs() -> Vec<CaptureDevice> {
    let mut out = Vec::new();
    let Ok(read) = std::fs::read_dir("/sys/class/video4linux") else {
        return out;
    };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("video") {
            continue;
        }
        let path = format!("/dev/{name}");
        let display_name = read_v4l2_card(&path).unwrap_or_else(|| name.clone());
        out.push(CaptureDevice {
            id: format!("v4l2:{path}"),
            display_name,
            backend: "v4l2".into(),
        });
    }
    out
}

fn read_v4l2_card(dev_path: &str) -> Option<String> {
    let dev_name = dev_path.strip_prefix("/dev/")?;
    let card_path = format!("/sys/class/video4linux/{dev_name}/name");
    std::fs::read_to_string(card_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn merge_unique(devices: &mut Vec<CaptureDevice>, more: Vec<CaptureDevice>) {
    for d in more {
        if !devices.iter().any(|x| x.id == d.id) {
            devices.push(d);
        }
    }
}
