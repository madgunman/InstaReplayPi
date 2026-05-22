# Instant Replay — macOS hardware acceptance

| Field | Value |
|-------|-------|
| Platform | macOS (Apple Silicon / Intel) |
| Date | _YYYY-MM-DD_ |
| Tester | |
| Engine version | |
| Capture device | e.g. Elgato Cam Link 4K |
| Program display | e.g. HDMI 1920×1080 @ 60 Hz |
| Buffer storage | e.g. local SSD path |

## Minimum hardware

| Requirement | Met |
|-------------|-----|
| Cam Link or UVC | ☐ |
| External HDMI 1080p50/60 | ☐ |
| Local SSD buffer path | ☐ |
| `flutter build macos` | ☐ |

## Automated

| Script | Result | Notes |
|--------|--------|-------|
| `./scripts/mvp_accept-full.sh` | pass / fail | See [RESULTS-macos-automated.md](RESULTS-macos-automated.md) |
| `SOAK_SECONDS=3600 ./scripts/soak_test.sh` | pass / fail / skipped | |

**Dev commands:** `./scripts/run-mac.sh live`, `./scripts/check-camera-macos.sh`  
**Package:** `./scripts/package-macos.sh` → `Start Instant Replay.command`  
**Codesign / notarize:** [packaging/macos/README.md](../../packaging/macos/README.md)

## MVP checklist (manual)

| # | Item | Result | Notes |
|---|------|--------|-------|
| 1 | UVC device in input list | | device id: |
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

## P0 issues (mark → replay → return live)

| Issue | Status |
|-------|--------|
| Wrong replay segment | open / fixed / n/a |
| Stuck REPLAYING | open / fixed / n/a |
| Crash on disconnect | open / fixed / n/a |
| Silent empty replay | open / fixed / n/a |

## Overall

- [ ] **MVP accepted** for macOS
- [ ] **Blocked**
