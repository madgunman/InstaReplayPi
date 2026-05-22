# Operator Guide — Raspberry Pi 5

Venue runbook for table-tennis instant replay. Config: [CONFIG.md](CONFIG.md). Install: [PI_DEPLOYMENT.md](PI_DEPLOYMENT.md).

## Pre-match checklist (5 minutes)

| Step | Check |
|------|--------|
| 1 | Cam Link / UVC on `/dev/video0` — `./scripts/doctor-pi.sh` |
| 2 | Audience HDMI shows live after boot (`replay-engine.service`) |
| 3 | USB3 SSD mounted at `/var/lib/instant-replay` — **≥ 5 GB** free |
| 4 | Touch UI at **http://127.0.0.1:8080** — status **LIVE**, buffer **≥ 2 s** |
| 5 | Test **Mark** → **Replay** → return live (or **L** on keyboard) |
| 6 | Test **Replay Last** once |
| 7 | Keyboard **M / R / Space / L / C** work with touch browser unfocused |

## Match operation

1. Power on Pi → systemd starts `replay-engine --appliance` → live on audience HDMI.
2. Operator uses **Pi touchscreen** (Chromium kiosk) or **USB keyboard** — same actions.
3. During rally:
   - **Mark** at rally start
   - **Replay** at rally end → 0.5× replay, auto return live when finished
   - **Replay Last** for instant last N seconds
   - **Live** to interrupt replay immediately

### Two displays

| Display | Role |
|---------|------|
| Audience HDMI | GStreamer/winit program output (fullscreen) |
| Pi official 7" touch | Chromium kiosk → `http://127.0.0.1:8080` |

Document which physical HDMI port is audience vs operator during install.

## Touch UI

Large buttons: Mark, Replay, Replay Last, Live, Clear. Buttons stay disabled until `buffer_ready` (same as keyboard gating).

Technician setup (devices, config) is **not** on the touch page during a match — edit `/etc/instant-replay/config.toml` before play.

## Status indicators

| Status | Meaning |
|--------|---------|
| LIVE | Normal live output |
| MARKED | Mark set |
| REPLAYING | Playing buffer |
| NO SIGNAL | Input lost — check HDMI/USB or disk |
| ERROR | See `last_error` in diagnostics |

**Mark** requires `buffer_ready`. **Replay** needs **≥ 1.5 s** buffered video.

### Replay mode (`[replay] mode`)

| Mode | Behavior |
|------|----------|
| `marked` (default) | **R** replays from mark; else last N seconds |
| `last` | **R** always replays last N seconds |

## Failure playbook

| Symptom | Likely cause | What to do |
|---------|----------------|------------|
| **NO SIGNAL** on audience HDMI | Cable / wrong input / disk full | Reseat capture; free SSD space |
| Touch UI unreachable | Engine down or HTTP disabled | `sudo systemctl restart replay-engine`; check `[http] enabled` |
| Mark disabled | Buffer not ready | Wait for LIVE + buffer ≥ 2 s |
| Replay fails | Buffer &lt; 1.5 s | Wait; verify SSD path in config |
| Stuck REPLAYING | Rare | Press **L** (Live); restart service if needed |
| Hotkeys dead on Pi | Wayland/global-hotkey | Use touch UI; see [PI_ONLY.md](PI_ONLY.md) v1.1 evdev note |
| Wrong segment | Mark cleared or `mode=last` | Check state MARKED vs config |

Diagnostics: `curl -s http://127.0.0.1:8080/api/diagnostics | jq` or `./scripts/doctor-pi.sh`

## Logs

```bash
journalctl -u replay-engine -f
```

File logs (if configured): under `/etc/instant-replay/logs/` or legacy `~/.config/instant-replay/logs/`

`RUST_LOG=replay_engine=debug` in systemd drop-in for verbose traces.

## Hotkeys (configurable in config.toml)

| Key | Action |
|-----|--------|
| M | Mark |
| R | Replay |
| Space | Replay last (default 10 s) |
| L | Return live |
| C | Clear mark |

## GPIO (v1.1 backlog)

Physical buttons via `rppal` are **not** in v1. Use touch + keyboard for v1 ship.

## Automated acceptance

On device or CI (test pattern):

```bash
./scripts/mvp_accept-full.sh
SOAK_SECONDS=3600 ./scripts/soak_test.sh   # engine must be running
```

Sign-off template: [acceptance/RESULTS-pi.md](acceptance/RESULTS-pi.md)
