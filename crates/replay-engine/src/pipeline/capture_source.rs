//! V4L2 capture source fragments for Raspberry Pi (plus videotestsrc for --test).

use crate::devices::{parse_device_id, ParsedDevice};

pub fn build_source_element(
    device_id: &str,
    width: u32,
    height: u32,
    fps: u32,
    pixel_format: &str,
) -> String {
    let parsed = parse_device_id(device_id);
    match parsed {
        ParsedDevice::Test => test_source(width, height, fps),
        ParsedDevice::Auto | ParsedDevice::Default => {
            panic!(
                "capture device_id must be resolved before building pipeline (use capture_select::resolve_input)"
            );
        }
        ParsedDevice::V4l2 { path } => v4l2_source(&path, width, height, fps, pixel_format),
    }
}

fn test_source(width: u32, height: u32, fps: u32) -> String {
    format!(
        "videotestsrc is-live=true pattern=ball \
         ! video/x-raw,width={width},height={height},framerate={fps}/1"
    )
}

fn v4l2_io_mode() -> &'static str {
    match std::env::var("INSTANT_REPLAY_V4L2_IO_MODE")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("dmabuf") => "dmabuf",
        Some("mmap") => "mmap",
        Some("read") => "read",
        Some("auto") => "auto",
        _ => "dmabuf",
    }
}

fn v4l2_source(path: &str, width: u32, height: u32, fps: u32, pixel_format: &str) -> String {
    let io_mode = v4l2_io_mode();
    let base = format!("v4l2src device={path} io-mode={io_mode}");
    match normalize_pixel_format(pixel_format) {
        PixelKind::Mjpeg => format!(
            "{base} ! image/jpeg,width={width},height={height},framerate={fps}/1 ! jpegdec"
        ),
        PixelKind::H264 => format!(
            "{base} ! video/x-h264,width={width},height={height},framerate={fps}/1 \
             ! h264parse ! avdec_h264"
        ),
        PixelKind::Raw => format!(
            "{base} ! video/x-raw,width={width},height={height},framerate={fps}/1"
        ),
    }
}

enum PixelKind {
    Mjpeg,
    H264,
    Raw,
}

fn normalize_pixel_format(pixel_format: &str) -> PixelKind {
    let p = pixel_format.to_lowercase();
    if p.contains("mjpeg") || p.contains("jpeg") || p == "image/jpeg" {
        PixelKind::Mjpeg
    } else if p.contains("h264") {
        PixelKind::H264
    } else {
        PixelKind::Raw
    }
}
