//! USB / V4L2 capture device discovery, filtering, and venue-friendly mode selection.

use anyhow::{Context, Result};
use replay_core::config::InputConfig;
use replay_core::types::{DisplayInfo, VideoFormat};
use tracing::{info, warn};

use crate::devices::CaptureDevice;
use crate::format_probe;

const MIN_CAPTURE_EDGE: i32 = 640;
const MAX_CAPTURE_DIM: i32 = 3840;
const MAX_CAPTURE_FPS: i32 = 120;

/// Resolved capture parameters ready for `start_live`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInput {
    pub device_id: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub pixel_format: String,
}

pub fn is_internal_platform_device(card_name: &str) -> bool {
    let lower = card_name.to_lowercase();
    const DENY: &[&str] = &[
        "pispbe",
        "pisp_be",
        "rpi-hevc",
        "hevc",
        "bcm2835",
        "codec",
        "bcm2835-codec",
    ];
    DENY.iter().any(|d| lower.contains(d))
}

pub fn usable_formats(device_id: &str) -> Vec<VideoFormat> {
    format_probe::probe_formats(device_id)
        .into_iter()
        .filter(|f| f.width >= MIN_CAPTURE_EDGE || f.height >= MIN_CAPTURE_EDGE)
        .filter(|f| f.width <= MAX_CAPTURE_DIM && f.height <= MAX_CAPTURE_DIM)
        .filter(|f| {
            let fps = if f.fps_den > 0 {
                f.fps_num / f.fps_den
            } else {
                0
            };
            fps <= MAX_CAPTURE_FPS
        })
        .filter(|f| !is_grey_only_small(f))
        .collect()
}

fn is_grey_only_small(f: &VideoFormat) -> bool {
    let pf = f.pixel_format.to_uppercase();
    pf.contains("GREY") && f.width < MIN_CAPTURE_EDGE && f.height < MIN_CAPTURE_EDGE
}

fn format_score(f: &VideoFormat) -> u64 {
    let fps = if f.fps_den > 0 {
        f.fps_num as u64 / f.fps_den as u64
    } else {
        0
    };
    let pf_bonus = if f.pixel_format.to_uppercase().contains("MJPEG")
        || f.pixel_format.to_uppercase().contains("H264")
    {
        1_000_000
    } else {
        0
    };
    f.width as u64 * f.height as u64 * fps.max(1) + pf_bonus
}

pub fn pick_venue_format(formats: &[VideoFormat]) -> Option<VideoFormat> {
    if formats.is_empty() {
        return None;
    }

    const PREFERRED: &[(i32, i32, i32, &str)] = &[
        (1920, 1080, 60, "MJPEG"),
        (1920, 1080, 50, "MJPEG"),
        (1920, 1080, 60, "H264"),
        (1920, 1080, 50, "H264"),
        (1920, 1080, 30, "MJPEG"),
        (1920, 1080, 30, "H264"),
        (1280, 720, 60, "MJPEG"),
        (1280, 720, 50, "MJPEG"),
        (1280, 720, 30, "MJPEG"),
        (1280, 720, 30, "auto"),
        (640, 480, 30, "MJPEG"),
    ];

    for &(w, h, fps, pf) in PREFERRED {
        if let Some(found) = formats.iter().find(|f| format_matches_preference(f, w, h, fps, pf)) {
            return Some(found.clone());
        }
    }

    formats
        .iter()
        .max_by_key(|f| format_score(f))
        .cloned()
}

fn format_matches_preference(f: &VideoFormat, w: i32, h: i32, fps: i32, pf: &str) -> bool {
    if f.width != w || f.height != h {
        return false;
    }
    let actual_fps = if f.fps_den > 0 {
        f.fps_num / f.fps_den
    } else {
        0
    };
    if actual_fps < fps {
        return false;
    }
    if pf == "auto" {
        return true;
    }
    let upper = f.pixel_format.to_uppercase();
    if pf == "MJPEG" {
        return upper.contains("MJPEG") || upper.contains("JPEG");
    }
    if pf == "H264" {
        return upper.contains("H264");
    }
    upper.contains(pf)
}

fn read_v4l2_card(dev_path: &str) -> Option<String> {
    let dev_name = dev_path.strip_prefix("/dev/")?;
    let card_path = format!("/sys/class/video4linux/{dev_name}/name");
    std::fs::read_to_string(card_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn sysfs_video_nodes() -> Vec<String> {
    let Ok(read) = std::fs::read_dir("/sys/class/video4linux") else {
        return Vec::new();
    };
    let mut nodes: Vec<String> = read
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("video"))
        .map(|n| format!("/dev/{n}"))
        .collect();
    nodes.sort();
    nodes
}

pub fn best_node_for_card(card: &str) -> Option<(String, Vec<VideoFormat>)> {
    let mut best: Option<(String, Vec<VideoFormat>, u64)> = None;
    for path in sysfs_video_nodes() {
        let Some(name) = read_v4l2_card(&path) else {
            continue;
        };
        if name != card {
            continue;
        }
        let id = format!("v4l2:{path}");
        let formats = usable_formats(&id);
        if formats.is_empty() {
            continue;
        }
        let score = formats.iter().map(format_score).max().unwrap_or(0);
        if best.as_ref().map(|(_, _, s)| *s < score).unwrap_or(true) {
            best = Some((path, formats, score));
        }
    }
    best.map(|(path, formats, _)| (path, formats))
}

pub fn discover_capture_devices() -> Vec<CaptureDevice> {
    gstreamer::init().ok();

    let mut cards: std::collections::BTreeMap<String, (String, Vec<VideoFormat>)> =
        std::collections::BTreeMap::new();

    for path in sysfs_video_nodes() {
        let Some(card) = read_v4l2_card(&path) else {
            continue;
        };
        if is_internal_platform_device(&card) {
            continue;
        }
        let id = format!("v4l2:{path}");
        let formats = usable_formats(&id);
        if formats.is_empty() {
            continue;
        }
        let score = formats.iter().map(format_score).max().unwrap_or(0);
        cards
            .entry(card.clone())
            .and_modify(|(best_path, best_formats)| {
                let best_score = best_formats.iter().map(format_score).max().unwrap_or(0);
                if score > best_score {
                    *best_path = path.clone();
                    *best_formats = formats.clone();
                }
            })
            .or_insert((path, formats));
    }

    cards
        .into_iter()
        .map(|(display_name, (path, _))| CaptureDevice {
            id: format!("v4l2:{path}"),
            display_name,
            backend: "v4l2".into(),
        })
        .collect()
}

fn auto_pick_device() -> Option<(CaptureDevice, Vec<VideoFormat>)> {
    let devices = discover_capture_devices();
    let mut best: Option<(CaptureDevice, Vec<VideoFormat>, u64)> = None;
    for dev in devices {
        let formats = usable_formats(&dev.id);
        if formats.is_empty() {
            continue;
        }
        let score = formats.iter().map(format_score).max().unwrap_or(0);
        if best.as_ref().map(|(_, _, s)| *s < score).unwrap_or(true) {
            best = Some((dev, formats, score));
        }
    }
    best.map(|(d, f, _)| (d, f))
}

fn wants_auto_device(device_id: &str) -> bool {
    let d = device_id.trim().to_lowercase();
    d.is_empty() || d == "auto" || d == "default"
}

fn wants_auto_resolution(resolution: &str) -> bool {
    resolution.trim().eq_ignore_ascii_case("auto")
}

fn parse_resolution_pair(resolution: &str) -> Option<(u32, u32)> {
    let parts: Vec<_> = resolution.split('x').collect();
    if parts.len() == 2 {
        Some((parts[0].parse().ok()?, parts[1].parse().ok()?))
    } else {
        None
    }
}

pub fn resolve_input(input: &InputConfig) -> Result<ResolvedInput> {
    if wants_auto_device(&input.device_id) {
        return resolve_auto(input);
    }

    let id = normalize_device_id(&input.device_id);
    let formats = usable_formats(&id);
    if formats.is_empty() {
        warn!(device_id = %id, "Configured capture device has no usable formats; falling back to auto");
        return resolve_auto(input);
    }

    Ok(build_resolved(&id, input, &formats))
}

fn normalize_device_id(device_id: &str) -> String {
    if device_id.starts_with("v4l2:") || device_id == "test" {
        device_id.to_string()
    } else if device_id.starts_with("/dev/video") {
        format!("v4l2:{device_id}")
    } else {
        device_id.to_string()
    }
}

fn resolve_auto(input: &InputConfig) -> Result<ResolvedInput> {
    let (dev, formats) = auto_pick_device().context(
        "No USB capture device found. Connect a webcam or HDMI capture card (UVC), then retry.",
    )?;
    Ok(build_resolved(&dev.id, input, &formats))
}

fn build_resolved(device_id: &str, input: &InputConfig, formats: &[VideoFormat]) -> ResolvedInput {
    let venue = pick_venue_format(formats).unwrap_or_else(|| {
        VideoFormat {
            width: 1280,
            height: 720,
            fps_num: 30,
            fps_den: 1,
            pixel_format: "MJPEG".into(),
        }
    });

    let (width, height, fps, pixel_format) = if wants_auto_resolution(&input.resolution)
        || input.fps == 0
        || input.pixel_format.eq_ignore_ascii_case("auto")
    {
        let fps = if input.fps == 0 {
            (venue.fps_num / venue.fps_den.max(1)) as u32
        } else {
            input.fps
        };
        let pf = if input.pixel_format.eq_ignore_ascii_case("auto") {
            venue.pixel_format.clone()
        } else {
            input.pixel_format.clone()
        };
        let (w, h) = if wants_auto_resolution(&input.resolution) {
            (venue.width as u32, venue.height as u32)
        } else {
            parse_resolution_pair(&input.resolution)
                .unwrap_or((venue.width as u32, venue.height as u32))
        };
        (w, h, fps, pf)
    } else {
        let (w, h) = parse_resolution_pair(&input.resolution)
            .unwrap_or((venue.width as u32, venue.height as u32));
        (w, h, input.fps, input.pixel_format.clone())
    };

    // If explicit resolution/fps doesn't match any format, prefer venue pick for pixel format.
    let pixel_format = if pixel_format.eq_ignore_ascii_case("auto") {
        venue.pixel_format
    } else {
        pixel_format
    };

    let resolved = ResolvedInput {
        device_id: device_id.to_string(),
        width,
        height,
        fps,
        pixel_format,
    };
    info!(
        device_id = %resolved.device_id,
        width = resolved.width,
        height = resolved.height,
        fps = resolved.fps,
        pixel_format = %resolved.pixel_format,
        "Resolved capture input"
    );
    resolved
}

pub fn resolve_output_display(
    auto_display: bool,
    operator_display_id: u32,
    displays: &[DisplayInfo],
) -> u32 {
    if !auto_display || displays.len() <= 1 {
        return displays
            .first()
            .map(|d| d.id)
            .unwrap_or(operator_display_id);
    }

    displays
        .iter()
        .filter(|d| d.id != operator_display_id)
        .max_by_key(|d| (d.width as i64) * (d.height as i64))
        .map(|d| d.id)
        .unwrap_or(operator_display_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(w: i32, h: i32, fps: i32, pf: &str) -> VideoFormat {
        VideoFormat {
            width: w,
            height: h,
            fps_num: fps,
            fps_den: 1,
            pixel_format: pf.into(),
        }
    }

    #[test]
    fn platform_filter() {
        assert!(is_internal_platform_device("pispbe"));
        assert!(is_internal_platform_device("rpi-hevc-dec"));
        assert!(!is_internal_platform_device("Logitech BRIO"));
    }

    #[test]
    fn pick_venue_prefers_1080p30_mjpeg_over_yuyv_5fps() {
        let formats = vec![
            fmt(1920, 1080, 5, "YUYV"),
            fmt(1920, 1080, 30, "MJPEG"),
        ];
        let picked = pick_venue_format(&formats).unwrap();
        assert_eq!(picked.pixel_format, "MJPEG");
        assert_eq!(picked.fps_num, 30);
    }

    #[test]
    fn pick_venue_prefers_1080p50_when_available() {
        let formats = vec![fmt(1920, 1080, 50, "MJPEG"), fmt(1280, 720, 30, "MJPEG")];
        let picked = pick_venue_format(&formats).unwrap();
        assert_eq!(picked.width, 1920);
        assert_eq!(picked.fps_num, 50);
    }

    #[test]
    fn grey_small_format_filtered() {
        assert!(is_grey_only_small(&fmt(340, 340, 30, "GREY")));
        assert!(!is_grey_only_small(&fmt(1280, 720, 30, "MJPEG")));
    }

    #[test]
    fn oversize_format_rejected_by_usable_filter() {
        let huge = fmt(4096, 2160, 30, "MJPEG");
        let formats = vec![huge.clone(), fmt(1920, 1080, 30, "MJPEG")];
        let filtered: Vec<_> = formats
            .into_iter()
            .filter(|f| f.width <= MAX_CAPTURE_DIM && f.height <= MAX_CAPTURE_DIM)
            .filter(|f| {
                let fps = if f.fps_den > 0 {
                    f.fps_num / f.fps_den
                } else {
                    0
                };
                fps <= MAX_CAPTURE_FPS
            })
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].width, 1920);
    }
}
