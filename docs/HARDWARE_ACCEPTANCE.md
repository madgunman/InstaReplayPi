# Hardware acceptance — Raspberry Pi 5

Sign-off template: [acceptance/RESULTS-pi.md](acceptance/RESULTS-pi.md)

## Minimum hardware

- Raspberry Pi 5 with active cooling
- Raspberry Pi OS 64-bit (Bookworm)
- Elgato Cam Link 4K or UVC HDMI capture on **powered USB hub**
- **USB3 SSD** for buffer at `/var/lib/instant-replay` (not SD card)
- Audience HDMI monitor 1080p50 or 1080p60
- Pi official 7" touch (optional) for operator UI kiosk

## Automated gates

```bash
# On Pi or CI (test pattern + xvfb on headless CI)
./scripts/mvp_accept-full.sh

# Engine running (appliance or --test)
SOAK_SECONDS=3600 ./scripts/soak_test.sh
```

Pre-match on device:

```bash
./scripts/doctor-pi.sh
```

## Manual checklist

1. UVC device listed (`/api/devices` or `v4l2-ctl --list-devices`)
2. Live fullscreen on audience HDMI at venue frame rate
3. Buffer writes to SSD; no sustained disk errors
4. Mark → replay at 0.5× returns to live
5. Replay last (Space / touch button)
6. **L** interrupts replay
7. Touch UI gates (Mark disabled until buffer ready)
8. Keyboard hotkeys without browser focus
9. HDMI disconnect → **NO SIGNAL**, no crash
10. 60 min soak — stable memory, no OOM
11. Reboot → autostart live without manual commands

## GPIO

Physical GPIO buttons are **v1.1** — not required for MVP sign-off.

## CI

`.github/workflows/acceptance.yml` runs HTTP `mvp_accept-full` and 120 s soak on `ubuntu-24.04-arm` with test pattern.
