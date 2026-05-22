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
| `input` | `device_id` | `v4l2:/dev/video0` |
| `input` | `resolution` | `1920x1080` |
| `input` | `fps` | `50` |
| `storage` | `buffer_path` | `/var/lib/instant-replay/buffer` |
| `appliance` | `enabled` | `true` |
| `appliance` | `autostart_live` | `true` |
| `operator` | `enabled` | `true` |
| `operator` | `display_id` | `0` (Pi touch monitor index) |
| `operator` | `width` / `height` | `800` / `480` |
| `output` | `display_id` | `0` (audience HDMI monitor index) |
| `output` | `fullscreen` | `true` |

## Displays

- **`output.display_id`** — audience HDMI program window (fullscreen).
- **`operator.display_id`** — Pi official touch (native egui window).

If both are on the same monitor, set different indices after checking logs at boot (`list_displays` is logged when live starts).

## systemd

- `replay-engine.service` — `ExecStart=/opt/instant-replay/bin/replay-engine --appliance`
- Requires `DISPLAY=:0` and desktop autologin for operator + HDMI windows.
- Environment: `/etc/default/replay-engine` (`INSTANT_REPLAY_V4L2_IO_MODE=dmabuf` on Pi)
- Logs: `journalctl -u replay-engine`

## Version sync

```bash
./scripts/sync-version.sh
```
