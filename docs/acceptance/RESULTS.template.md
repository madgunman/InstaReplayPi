# Instant Replay — Hardware Acceptance Record

| Field | Value |
|-------|-------|
| Platform | e.g. macOS 14 / Apple M2 |
| Date | YYYY-MM-DD |
| Tester | |
| Engine version | `replay-engine --version` or Cargo.toml version |
| Capture device | e.g. Elgato Cam Link 4K, UVC generic |
| Program display | e.g. HDMI → LG 1920×1080 |
| Buffer storage | path + disk type |

## Automated runs

| Script | Result | Notes |
|--------|--------|-------|
| `./scripts/mvp_accept-full.sh` | pass / fail | |
| `SOAK_SECONDS=3600 ./scripts/soak_test.sh` | pass / fail / skipped | |

## MVP checklist

| # | Item | Result | Notes |
|---|------|--------|-------|
| 1 | UVC device in input list | pass / fail | device id: |
| 2 | 1080p50/60 live fullscreen external | pass / fail | format used: |
| 3 | Rolling 20s buffer, bounded disk, auto clean | pass / fail | |
| 4 | Replay last 10s @ 0.5× | pass / fail | |
| 5 | Mark → replay from mark @ 0.5× | pass / fail | |
| 6 | Return live after replay; L interrupts | pass / fail | |
| 7 | Engine hotkeys without Flutter | pass / fail | |
| 8 | 60 min soak no crash | pass / fail / skipped | |
| 9 | Input disconnect no crash | pass / fail | |
| 10 | NO SIGNAL on loss | pass / fail | |
| 11 | Restart cleans buffer files | pass / fail | |

## Platform sign-off

| Platform | Signed off |
|----------|------------|
| macOS | ☐ |
| Windows | ☐ |
| Linux x86_64 | ☐ |
| Raspberry Pi 5 | ☐ |

## Issues found

1. 
2. 

## Overall

- [ ] **MVP accepted** for this platform
- [ ] **Blocked** — list blockers above
