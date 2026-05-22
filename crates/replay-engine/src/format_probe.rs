//! Probe supported video modes for a capture device via GStreamer caps.

use std::collections::HashSet;

use gstreamer as gst;
use gstreamer::prelude::*;
use replay_core::types::VideoFormat;
use tracing::debug;

use crate::devices::{parse_device_id, ParsedDevice};

const PREFERRED: &[(u32, u32, u32)] = &[
    (1920, 1080, 60),
    (1920, 1080, 50),
    (1920, 1080, 30),
    (1280, 720, 60),
    (1280, 720, 50),
    (1280, 720, 30),
    (640, 480, 60),
    (640, 480, 30),
];

pub fn probe_formats(device_id: &str) -> Vec<VideoFormat> {
    if device_id == "test" {
        return test_pattern_formats();
    }

    let _ = gst::init();
    let parsed = parse_device_id(device_id);

    let caps = match parsed {
        ParsedDevice::Test => return test_pattern_formats(),
        ParsedDevice::Auto | ParsedDevice::Default => return preferred_fallback_formats(),
        ParsedDevice::V4l2 { path } => probe_element_caps("v4l2src", 0, Some(path)),
    };

    let mut formats = caps.map(|c| caps_to_formats(&c)).unwrap_or_default();

    if formats.is_empty() {
        debug!(device_id, "No caps from probe; using preferred fallback list");
        formats = preferred_fallback_formats();
    } else {
        formats = filter_and_sort_formats(formats);
    }

    formats
}

fn test_pattern_formats() -> Vec<VideoFormat> {
    preferred_fallback_formats()
}

fn preferred_fallback_formats() -> Vec<VideoFormat> {
    PREFERRED
        .iter()
        .map(|&(w, h, fps)| VideoFormat {
            width: w as i32,
            height: h as i32,
            fps_num: fps as i32,
            fps_den: 1,
            pixel_format: "auto".into(),
        })
        .collect()
}

fn probe_element_caps(
    factory: &str,
    device_index: u32,
    v4l2_path: Option<String>,
) -> Option<gst::Caps> {
    let elem = gst::ElementFactory::make(factory).build().ok()?;
    if let Some(ref path) = v4l2_path {
        elem.set_property("device", path.as_str());
    } else if factory != "v4l2src" {
        elem.set_property("device-index", device_index);
    }

    let caps = if elem.set_state(gst::State::Ready).is_ok() {
        elem.static_pad("src")
            .and_then(|pad| pad.current_caps().or_else(|| Some(pad.pad_template_caps())))
    } else {
        None
    };

    let _ = elem.set_state(gst::State::Null);
    caps.or_else(|| pad_template_caps(factory))
}

fn pad_template_caps(factory: &str) -> Option<gst::Caps> {
    let elem = gst::ElementFactory::make(factory).build().ok()?;
    elem.static_pad("src").map(|pad| pad.pad_template_caps())
}

fn caps_to_formats(caps: &gst::Caps) -> Vec<VideoFormat> {
    let mut out = Vec::new();
    for i in 0..caps.size() {
        if let Some(s) = caps.structure(i) {
            out.extend(structure_to_formats(s));
        }
    }
    out
}

fn structure_to_formats(s: &gst::StructureRef) -> Vec<VideoFormat> {
    let name = s.name();
    if !name.starts_with("video/") && !name.starts_with("image/") {
        return Vec::new();
    }

    let pixel_format = if name.contains("jpeg") {
        "MJPEG".to_string()
    } else if name.contains("h264") {
        "H264".to_string()
    } else {
        s.get::<String>("format")
            .unwrap_or_else(|_| "auto".to_string())
    };

    let widths = int_values(s, "width");
    let heights = int_values(s, "height");
    let fps_list = framerate_values(s);

    let mut formats = Vec::new();
    for &w in &widths {
        for &h in &heights {
            for &(num, den) in &fps_list {
                if w > 0 && h > 0 && num > 0 && den > 0 {
                    formats.push(VideoFormat {
                        width: w as i32,
                        height: h as i32,
                        fps_num: num as i32,
                        fps_den: den as i32,
                        pixel_format: pixel_format.clone(),
                    });
                }
            }
        }
    }
    formats
}

fn int_values(s: &gst::StructureRef, field: &str) -> Vec<i32> {
    if let Ok(v) = s.get::<i32>(field) {
        return vec![v];
    }
    if let Ok(range) = s.get::<gst::IntRange<i32>>(field) {
        let mut vals = Vec::new();
        vals.push(range.max());
        if range.min() != range.max() {
            vals.push(range.min());
        }
        return vals;
    }
    Vec::new()
}

fn framerate_values(s: &gst::StructureRef) -> Vec<(u32, u32)> {
    if let Ok(f) = s.get::<gst::Fraction>("framerate") {
        return vec![(f.numer() as u32, f.denom() as u32)];
    }
    if let Ok(range) = s.get::<gst::FractionRange>("framerate") {
        let mut out = Vec::new();
        let high = range.max();
        let low = range.min();
        out.push((high.numer() as u32, high.denom() as u32));
        if low.numer() != high.numer() || low.denom() != high.denom() {
            out.push((low.numer() as u32, low.denom() as u32));
        }
        return dedupe_fractions(out);
    }
    vec![(60, 1), (50, 1), (30, 1)]
}

fn dedupe_fractions(list: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    let mut seen = HashSet::new();
    list.into_iter()
        .filter(|f| seen.insert(*f))
        .collect()
}

fn filter_and_sort_formats(mut formats: Vec<VideoFormat>) -> Vec<VideoFormat> {
    formats.sort_by(|a, b| {
        (b.width, b.height, b.fps_num)
            .cmp(&(a.width, a.height, a.fps_num))
    });
    formats.dedup_by(|a, b| {
        a.width == b.width
            && a.height == b.height
            && a.fps_num == b.fps_num
            && a.fps_den == b.fps_den
            && a.pixel_format == b.pixel_format
    });

    let preferred: Vec<VideoFormat> = PREFERRED
        .iter()
        .filter_map(|&(w, h, fps)| {
            formats.iter().find(|f| {
                f.width == w as i32 && f.height == h as i32 && f.fps_num == fps as i32
            }).cloned()
        })
        .collect();

    if !preferred.is_empty() {
        let mut rest: Vec<VideoFormat> = formats
            .into_iter()
            .filter(|f| {
                !preferred.iter().any(|p| {
                    p.width == f.width && p.height == f.height && p.fps_num == f.fps_num
                })
            })
            .take(12)
            .collect();
        let mut out = preferred;
        out.append(&mut rest);
        return out;
    }

    formats.truncate(16);
    formats
}
