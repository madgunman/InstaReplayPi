# MVP Acceptance Checklist — Pi 5

## Quick validation

```bash
./scripts/mvp_accept-full.sh
SOAK_SECONDS=3600 ./scripts/soak_test.sh   # engine running
./scripts/doctor-pi.sh                     # on device
```

Hardware: **[HARDWARE_ACCEPTANCE.md](HARDWARE_ACCEPTANCE.md)**  
Sign-off: **[acceptance/RESULTS-pi.md](acceptance/RESULTS-pi.md)**

## Functional criteria

| # | Criterion | Auto | Manual |
|---|-----------|------|--------|
| 1 | UVC on `/dev/video0` | `/api/devices` | Cam Link name |
| 2 | 1080p50/60 live on audience HDMI | — | Required |
| 3 | Rolling 20s buffer on USB SSD | buffer wait | `df` / folder size |
| 4 | Replay last at 0.5× | HTTP | Visual |
| 5 | Mark → replay at 0.5× | HTTP | Visual |
| 6 | Return live; **L** interrupt | HTTP | Keyboard |
| 7 | Touch UI Mark/Replay/Live | HTTP gates | Pi 7" kiosk |
| 8 | Keyboard without browser focus | — | M/R/Space/L/C |
| 9 | 60 min soak | `soak_test.sh` | On Pi hardware |
| 10 | Disconnect no crash | — | Unplug HDMI |
| 11 | NO SIGNAL state | — | While live |
| 12 | Restart cleans buffer | — | Reboot service |

## Definition of done

1. `mvp_accept-full.sh` passes on CI (`ubuntu-24.04-arm`)  
2. [RESULTS-pi.md](acceptance/RESULTS-pi.md) signed on real Pi 5 + SSD + dual displays  
3. No P0: wrong replay, stuck REPLAYING, crash on disconnect, silent empty replay  
4. Release tag `v*` produces **pi5-aarch64** tarball only  

## Production sign-off

- [ ] `make finalize` green  
- [ ] `cargo test --workspace` green  
- [ ] [RESULTS-pi.md](acceptance/RESULTS-pi.md) complete  
- [ ] 60 min soak on Pi 5  
- [ ] Boot → systemd → live without manual commands  
