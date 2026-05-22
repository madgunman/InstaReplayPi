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
| `http` | `enabled` | `true` |
| `http` | `bind_addr` | `127.0.0.1:8080` |
| `output` | `fullscreen` | `true` |

## HTTP API (localhost)

| Method | Path | Action |
|--------|------|--------|
| GET | `/api/health` | Reachability + `ready` |
| GET | `/api/status` | Operator status JSON |
| GET | `/api/diagnostics` | FPS, state, buffer seconds |
| POST | `/api/mark`, `/api/replay`, … | Control actions |

Touch UI and `scripts/mvp_accept.sh` use these endpoints.

## systemd

- `replay-engine.service` — `ExecStart=/usr/bin/instant-replay --appliance`
- Environment: `/etc/default/replay-engine` (`INSTANT_REPLAY_V4L2_IO_MODE=dmabuf` on Pi)
- Logs: `journalctl -u replay-engine`

## Version sync

```bash
./scripts/sync-version.sh
```
