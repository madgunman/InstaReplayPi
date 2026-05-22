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

## Manual checklist (InstaReplay1 / v0.3.1)

1. `pgrep -c -x replay-engine` → **1** after boot
2. `./scripts/doctor-pi.sh` → PASS
3. BRIO or UVC plugged → **LIVE** with `device_id = "auto"` (no config edit)
4. Banner **Hold 3s** or Setup PIN unlocks technician panel
5. Apply → 1080p30 MJPEG → HDMI live, **no flicker for 60 s**
6. Mark → Replay works; **L** interrupts replay
7. Buffer on USB3 SSD; Mark disabled until buffer ready
8. Reboot → `systemctl` autostart only (do not launch a second engine from desktop)

## GPIO

Physical GPIO buttons are **v1.1** — not required for MVP sign-off.

## CI

`.github/workflows/acceptance.yml` runs HTTP `mvp_accept-full` and 120 s soak on `ubuntu-24.04-arm` with test pattern.
