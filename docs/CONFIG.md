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

## Plug-and-play capture

- **`device_id = "auto"`** — On boot the engine picks the best external capture card (one entry per physical device; skips Pi `pispbe` / `rpi-hevc` nodes and empty `/dev/video*` metadata nodes).
- **`resolution` / `fps` / `pixel_format` = `auto` or `0`** — Chooses a venue-friendly mode (e.g. 1080p50/60 MJPEG for Cam Link, 1080p30 MJPEG for webcams).
- Explicit `v4l2:/dev/videoN` still works; invalid nodes fall back to `auto`.

## Technician Setup (touch UI)

Unlock **Setup** on the operator screen:

- **Hold the status banner 3 seconds**, or
- Tap **Unlock setup (PIN)** and enter `operator.setup_pin` (default `0000`).

Then choose **Camera**, **Format**, and **Audience HDMI**, tap **Apply & go live**. Settings are saved to `config.toml`.

## Displays

- **`output.display_id`** — audience HDMI program window.
- **`operator.display_id`** — Pi touch operator window.
- With **`output.auto_display = true`** and two monitors, audience uses the largest monitor that is not the operator display.

## systemd

- `replay-engine.service` — `ExecStart=/opt/instant-replay/bin/replay-engine --appliance`
- Requires `DISPLAY=:0` and desktop autologin.
- Logs: `journalctl -u replay-engine`
