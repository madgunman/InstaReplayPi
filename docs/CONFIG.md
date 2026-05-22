# Configuration — Pi appliance

## Canonical path

```
/etc/instant-replay/config.toml
```

Example template: [config/default.toml](../config/default.toml)

Legacy JSON (`~/.config/instant-replay/config.json`) is still read if TOML is missing.

## Key fields

| Section | Field | Default |
|---------|-------|---------|
| `input` | `device_id` | `auto` (best USB UVC / Cam Link) |
| `input` | `resolution` | `auto` |
| `input` | `fps` | `0` (= auto) |
| `input` | `pixel_format` | `auto` (prefers MJPEG for 1080p) |
| `storage` | `buffer_path` | `/var/lib/instant-replay/buffer` |
| `appliance` | `autostart_live` | `true` |
| `operator` | `display_id` | `0` (Pi touch) |
| `operator` | `setup_pin` | `0000` (empty = PIN off) |
| `operator` | `setup_unlock_seconds` | `600` |
| `output` | `display_id` | `0` |
| `output` | `auto_display` | `true` (HDMI = non-touch monitor) |
| `output` | `fullscreen` | `true` |
| `replay` | `buffer_encoder` | `auto` (`auto` \| `x264` \| `vah264`) |
| `replay` | `buffer_seconds` | `20` |
| `replay` | `speed` | `0.5` |

## Buffer encoder (Pi CPU)

- **`replay.buffer_encoder = "auto"`** — On aarch64, uses `vah264enc` when GStreamer provides it; otherwise `x264enc`.
- **`vah264`** — Force Pi hardware encode (lower CPU at 1080p60).
- **`x264`** — Software encode (CI / fallback).

## Loopback HTTP (acceptance / diagnostics)

- Binds **`127.0.0.1:8080`** when `--test`, `--appliance`, or `--http-api` (not the operator UI).
- Disable: `INSTANT_REPLAY_HTTP=0`
- Port: `INSTANT_REPLAY_HTTP_PORT` (default `8080`)

## V4L2 capture tuning

- **`INSTANT_REPLAY_V4L2_IO_MODE`** in `/etc/default/replay-engine`: `dmabuf` (default on Pi), `mmap`, `read`, or `auto`.

## Plug-and-play capture

- **`device_id = "auto"`** — On boot the engine picks the best external capture card (one entry per physical device; skips Pi `pispbe` / `rpi-hevc` nodes and empty `/dev/video*` metadata nodes).
- **`resolution` / `fps` / `pixel_format` = `auto` or `0`** — Chooses a venue-friendly mode (e.g. 1080p50/60 MJPEG for Cam Link, 1080p30 MJPEG for webcams).
- Explicit `v4l2:/dev/videoN` still works; invalid nodes fall back to `auto`.

## Technician Setup (touch UI)

Unlock **Setup** on the operator screen:

- **Hold 3s to unlock** on the banner (or long-press the banner), or
- Tap **Unlock setup (PIN)** and enter `operator.setup_pin` (default `0000`).

Then choose **Camera**, **Format**, and **Audience HDMI**, tap **Apply & go live**. Settings are saved to `config.toml`.

## Displays

- **`output.display_id`** — audience HDMI program window.
- **`operator.display_id`** — Pi touch operator window.
- With **`output.auto_display = true`** and two monitors, audience uses the largest monitor that is not the operator display.

## systemd

- **Production start:** `sudo systemctl start replay-engine` only — do not run a second `replay-engine` from a desktop launcher.
- `replay-engine.service` — `ExecStart=/opt/instant-replay/bin/replay-engine --appliance`
- Requires `DISPLAY=:0`, `XDG_RUNTIME_DIR`, and desktop autologin.
- Single-instance lock: `/run/instant-replay/replay-engine.lock`
- Logs: `journalctl -u replay-engine`
- List devices on the Pi: `replay-engine --list-devices`
