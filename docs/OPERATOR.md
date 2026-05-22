# Operator Guide — Raspberry Pi 5

Venue runbook for table-tennis instant replay. Config: [CONFIG.md](CONFIG.md). Install: [PI_DEPLOYMENT.md](PI_DEPLOYMENT.md).

## Pre-match checklist (5 minutes)

| Step | Check |
|------|--------|
| 1 | Cam Link / UVC on `/dev/video0` — `./scripts/doctor-pi.sh` |
| 2 | Audience HDMI shows live after boot (`replay-engine.service`) |
| 3 | USB3 SSD mounted at `/var/lib/instant-replay` — **≥ 5 GB** free |
| 4 | **Native operator window** on Pi touch — status **LIVE**, buffer **≥ 2 s** |
| 5 | Test **Mark** → **Replay** → return live (or **L** on keyboard) |
| 6 | Test **Replay Last** once |
| 7 | Keyboard **M / R / Space / L / C** work with operator window unfocused |

## Match operation

1. Power on Pi → systemd starts `replay-engine --appliance` → live on audience HDMI + operator window.
2. Operator uses **Pi touchscreen** (native UI) or **USB keyboard** — same actions.
3. During rally:
   - **Mark** at rally start
   - **Replay** at rally end → 0.5× replay, auto return live when finished
   - **Replay Last** for instant last N seconds
   - **Live** to interrupt replay immediately

### Two displays

| Display | Role |
|---------|------|
| Audience HDMI | GStreamer/winit program output (fullscreen) |
| Pi official 7" touch | Native egui operator shell |

Set `output.display_id` and `operator.display_id` in config if monitors are swapped.

## Operator UI

Large buttons: Mark, Replay, Replay Last, Live, Clear. Buttons stay disabled until `buffer_ready` (same as keyboard gating).

Technician setup (devices, config) is **not** in the operator UI during a match — edit `/etc/instant-replay/config.toml` before play.

## Status indicators

| Status | Meaning |
|--------|---------|
| LIVE | Normal live output |
| MARKED | Mark set |
| REPLAYING | Playing buffer |
| NO SIGNAL | Input lost — check HDMI/USB or disk |
| ERROR | See `last_error` in banner |

## Hotkeys (configurable in config.toml)

| Key | Action |
|-----|--------|
| M | Mark |
| R | Replay |
| Space | Replay last 10 s |
| L | Return live |
| C | Clear mark |

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|----------------|-----|
| No operator window | No DISPLAY / autologin | Desktop autologin; `DISPLAY=:0` in systemd |
| Black operator window | GL/EGL missing | `sudo apt install libegl1 libgles2` |
| Audience HDMI black | Wrong `output.display_id` | Edit config; restart service |
| Capture fails | Wrong `/dev/video*` | `v4l2-ctl --list-devices`; fix `input.device_id` |

Diagnostics: `journalctl -u replay-engine -f` or `./scripts/doctor-pi.sh`
