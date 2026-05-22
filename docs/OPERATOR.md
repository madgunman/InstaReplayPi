# Operator Guide — Raspberry Pi 5

Venue runbook for table-tennis instant replay. Config: [CONFIG.md](CONFIG.md). Install: [PI_DEPLOYMENT.md](PI_DEPLOYMENT.md).

## Pre-match checklist (5 minutes)

| Step | Check |
|------|--------|
| 1 | USB capture connected — `./scripts/doctor-pi.sh` (auto-detect) |
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

With two monitors, `output.auto_display = true` (default) routes HDMI automatically; override in Setup if needed.

## Operator UI

**Match:** Mark, Replay, Replay Last, Live, Clear. Buttons stay disabled until `buffer_ready` (same as keyboard gating).

**Setup (technician, before the match):**

1. Hold **Hold 3s to unlock** on the banner (or long-press the banner) or tap **Unlock setup (PIN)** (default PIN `0000` in config).
2. Pick **Camera** (webcam, BRIO, Cam Link, etc.), **Format**, and **Audience HDMI**.
3. **Apply & go live** — saves to `/etc/instant-replay/config.toml`.
4. Tap **Lock setup** before the match so operators only see match buttons.

## Status indicators

| Status | Meaning |
|--------|---------|
| LIVE | Normal live output |
| MARKED | Mark set |
| REPLAYING | Playing buffer |
| NO SIGNAL | Input lost — check HDMI/USB or disk |
| ERROR | See `last_error` in banner |

## Replay speed

- **Replay Last** and short **Mark → Replay** clips usually play at **0.5×** as configured.
- If the mark spans **many buffer chunks**, playback may run at **1.0×** (GStreamer concat limitation). Use **Replay Last** for a fixed slow-motion window.

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
| Stuck on STARTING | No device / wrong format | Plug in USB capture; unlock Setup → Refresh → Apply; or set `device_id = auto` |
| Two operator windows / dead touch | Second replay-engine process | `sudo systemctl stop replay-engine`; ensure only one process (`doctor-pi`) |
| Capture fails | Wrong mode | Setup → pick MJPEG 1080p30 or 720p30 |

Diagnostics: `journalctl -u replay-engine -f` or `./scripts/doctor-pi.sh`
