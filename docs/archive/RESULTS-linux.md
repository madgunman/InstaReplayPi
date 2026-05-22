# Instant Replay — Linux x86_64 hardware acceptance

| Field | Value |
|-------|-------|
| Platform | Linux x86_64 |
| Date | _YYYY-MM-DD_ |
| Tester | |
| Engine version | |
| Capture device | e.g. `v4l2:/dev/video0` |
| Program display | HDMI 1920×1080 |
| Buffer storage | e.g. `/var/lib/instant-replay` |

## Minimum hardware

| Requirement | Met |
|-------------|-----|
| `v4l2src` UVC / Cam Link | ☐ |
| External HDMI | ☐ |
| Fast local buffer disk | ☐ |
| `flutter build linux` | ☐ |

## Automated

| Script | Result | Notes |
|--------|--------|-------|
| `./scripts/mvp_accept-full.sh` (xvfb) | pass / fail | Matches CI `acceptance` job |
| `SOAK_SECONDS=3600 ./scripts/soak_test.sh` | pass / fail / skipped | |

**Dev commands:** `./scripts/run-linux.sh test|live|ui|accept`  
**Package:** `./scripts/package-linux.sh` — `.deb`, systemd, desktop launcher  
**Desktop:** `instant-replay.desktop` → `instant-replay-launch`

## MVP checklist (manual)

| # | Item | Result | Notes |
|---|------|--------|-------|
| 1 | UVC device in input list | | |
| 2 | 1080p50/60 live fullscreen external | | |
| 3 | Rolling 20s buffer, bounded disk | | |
| 4 | Replay last 10s @ 0.5× | | |
| 5 | Mark → replay from mark @ 0.5× | | |
| 6 | Return live; **L** interrupts replay | | |
| 7 | Engine hotkeys without Flutter | | |
| 8 | 60 min soak no crash | | |
| 9 | Input disconnect no crash | | |
| 10 | NO SIGNAL on loss | | |
| 11 | Restart cleans buffer files | | |

## P0 issues

| Issue | Status |
|-------|--------|
| Wrong replay segment | |
| Stuck REPLAYING | |
| Crash on disconnect | |
| Silent empty replay | |

## Overall

- [ ] **MVP accepted** for Linux x86_64
- [ ] **Blocked**
