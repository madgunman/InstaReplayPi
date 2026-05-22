# Instant Replay — Windows hardware acceptance

| Field | Value |
|-------|-------|
| Platform | Windows 10/11 x86_64 |
| Date | _YYYY-MM-DD_ |
| Tester | |
| Engine version | |
| Capture device | e.g. Cam Link (`ks:0` / `dshow:0`) |
| Program display | HDMI 1920×1080 |
| Buffer storage | fast local disk (not OneDrive-synced) |

## Minimum hardware

| Requirement | Met |
|-------------|-----|
| UVC / Cam Link | ☐ |
| External HDMI | ☐ |
| GStreamer MSVC in bundle or `GSTREAMER_ROOT` | ☐ |
| `flutter build windows` | ☐ |

## Automated

| Script | Result | Notes |
|--------|--------|-------|
| `mvp_accept` via grpcurl (engine `--test`) | pass / fail | Git Bash or WSL |
| `SOAK_SECONDS=3600 ./scripts/soak_test.sh` | pass / fail / skipped | |

**Dev commands:** `.\scripts\run-windows.ps1 test` / `live` / `ui`  
**Package:** `powershell -File packaging\windows\bundle.ps1` → `Start Instant Replay.bat` sets `PATH` + `GST_PLUGIN_PATH`

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

- [ ] **MVP accepted** for Windows
- [ ] **Blocked**
