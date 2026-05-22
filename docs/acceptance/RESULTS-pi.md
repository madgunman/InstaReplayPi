# Instant Replay — Raspberry Pi 5 hardware acceptance

| Field | Value |
|-------|-------|
| Platform | Raspberry Pi 5 (aarch64) |
| Date | _YYYY-MM-DD_ |
| Tester | |
| Engine version | |
| Capture device | UVC on powered hub |
| Program display | HDMI 1080p50 |
| Buffer storage | **USB3 SSD** at `/var/lib/instant-replay` |

## Minimum hardware

| Requirement | Met |
|-------------|-----|
| Pi 5 + active cooling | ☐ |
| USB3 SSD for buffer (not SD) | ☐ |
| UVC dongle on powered hub | ☐ |
| 1080p50 validated | ☐ |
| Build on Pi or CI `ubuntu-24.04-arm` artifact | ☐ |

## Automated

| Script | Result | Notes |
|--------|--------|-------|
| `mvp_accept-full.sh` (HTTP, test pattern) | pass | 24 pass / 0 fail on dev CI-style run |
| On-Pi `mvp_accept-full.sh` | ☐ | Repeat on aarch64 hardware |
| `SOAK_SECONDS=3600 ./scripts/soak_test.sh` | ☐ | **Required** for Pi sign-off on device |

**Install:** `./scripts/package-pi.sh` → `./install-on-pi.sh`  
**Capture:** `INSTANT_REPLAY_V4L2_IO_MODE=dmabuf` (default on aarch64)  
**Appliance:** `systemctl status replay-engine` (User=pi drop-in)

## MVP checklist (manual)

| # | Item | Result | Notes |
|---|------|--------|-------|
| 1 | UVC device in input list | | |
| 2 | 1080p50 live fullscreen (50 Hz venue) | | |
| 3 | Buffer on SSD; no SD wear | | `df` / write rate |
| 4 | Replay last @ 0.5× | | |
| 5 | Mark → replay from mark @ 0.5× | | |
| 6 | Return live; **L** interrupt | | keyboard |
| 7 | Touch UI Mark/Replay/Live | | Native egui operator window |
| 8 | Hotkeys without touch focus | | appliance + desktop session |
| 9 | 60 min soak on Pi | | thermal / OOM |
| 10 | Disconnect no crash | | |
| 11 | NO SIGNAL | | |
| 12 | Restart cleans buffer | | |

## Tuning (if IO-bound)

| Setting | Value tried | Result |
|---------|-------------|--------|
| `chunk_seconds` | 1 / 2 | |
| `buffer_path` on SSD | | |
| `INSTANT_REPLAY_V4L2_IO_MODE` | dmabuf / auto | |

## P0 issues

| Issue | Status |
|-------|--------|
| Wrong replay segment | |
| Stuck REPLAYING | |
| Crash on disconnect | |
| Silent empty replay | |
| dmabuf capture failures | |

## Overall

- [ ] **MVP accepted** for Raspberry Pi 5
- [ ] **Blocked**
