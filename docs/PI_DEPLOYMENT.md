# Raspberry Pi 5 Deployment

**Repository:** https://github.com/madgunman/InstaReplayPi.git  
**Install helper:** [GITHUB_PI.md](GITHUB_PI.md)

## Requirements

- Raspberry Pi 5, 64-bit Pi OS (Bookworm)
- Active cooling (fan/heatsink)
- **USB3 SSD** mounted at `/var/lib/instant-replay` — do not use the SD card for the rolling buffer
- Powered USB hub for the capture dongle (Cam Link / UVC)
- Desktop session for touch kiosk + global hotkeys (Chromium on `:0`)

## Install from GitHub

### Release tarball (recommended)

After pushing tag `v*` and CI publishes `InstantReplay-*-pi5-aarch64.tar.gz`:

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-from-github.sh -o /tmp/install-ir.sh
chmod +x /tmp/install-ir.sh
/tmp/install-ir.sh --release
sudo systemctl start replay-engine
sudo systemctl enable --now instant-replay-kiosk
```

### Build on the Pi

```bash
git clone https://github.com/madgunman/InstaReplayPi.git ~/InstaReplayPi
cd ~/InstaReplayPi
./scripts/install-from-github.sh --build
```

`install-on-pi.sh` (from package or release) configures:

- `replay-engine` + `instant-replay` launcher in `/usr/local/bin`
- `User=pi` systemd drop-in
- `/etc/instant-replay/config.toml` from example if missing
- `/etc/default/replay-engine` with `INSTANT_REPLAY_V4L2_IO_MODE=dmabuf`
- `/var/lib/instant-replay` owned by `pi`

## Config

`/etc/instant-replay/config.toml` (example in repo: `config/default.toml`):

```toml
[input]
device_id = "v4l2:/dev/video0"
resolution = "1920x1080"
fps = 50

[storage]
buffer_path = "/var/lib/instant-replay/buffer"
auto_clean_on_start = true

[appliance]
enabled = true
autostart_live = true

[http]
enabled = true
bind_addr = "127.0.0.1:8080"

[output]
fullscreen = true
```

Operator touch UI: **http://127.0.0.1:8080** (Chromium kiosk via `instant-replay-kiosk.service`).

## IO tuning

| Symptom | Action |
|---------|--------|
| High CPU / dropped frames | Set `chunk_seconds` to **2** in `[replay]` |
| Slow writes / disk warnings | Confirm buffer on **USB3 SSD** |
| Capture glitches | `INSTANT_REPLAY_V4L2_IO_MODE=dmabuf` in `/etc/default/replay-engine` |
| dmabuf fails on dongle | Try `INSTANT_REPLAY_V4L2_IO_MODE=auto` |

## 1080p50 validation

1. `v4l2-ctl --device=/dev/video0 --list-formats-ext`
2. `sudo systemctl start replay-engine` — live on audience HDMI
3. `./scripts/doctor-pi.sh` or `curl -s http://127.0.0.1:8080/api/status`
4. Mark → replay → return live (touch or M/R/Space/L)
5. Record in [acceptance/RESULTS-pi.md](acceptance/RESULTS-pi.md)

## Sign-off

- `./scripts/doctor-pi.sh`
- Optional: `./scripts/mvp_accept-full.sh`
- **Required on device:** `SOAK_SECONDS=3600 ./scripts/soak_test.sh` (engine running)
- Fill [acceptance/RESULTS-pi.md](acceptance/RESULTS-pi.md)

## GPIO (v1.1)

GPIO buttons are not in v1; use touch UI + USB keyboard.
