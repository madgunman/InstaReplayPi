# Raspberry Pi 5 Deployment

**Repository:** [github.com/madgunman/InstaReplayPi](https://github.com/madgunman/InstaReplayPi)  
**Install:** [GITHUB_PI.md](GITHUB_PI.md)

## Requirements

- Raspberry Pi 5, 64-bit Pi OS (Bookworm)
- Active cooling, USB3 SSD at `/var/lib/instant-replay`, powered USB hub for capture
- **Desktop autologin** for Option B (boot → live + touch UI)

## Install + autostart (Option B)

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-from-github.sh -o /tmp/install-ir.sh
chmod +x /tmp/install-ir.sh
/tmp/install-ir.sh --release
# or: /tmp/install-ir.sh --build
```

`install-on-pi.sh` installs binaries, enables **replay-engine** + **instant-replay-kiosk**, sets systemd `User=` to your login (e.g. `admin`).

**One-time OS setting:** Raspberry Pi OS → **Auto Login** → **Desktop** → your user → **reboot**.

## Autostart only (already installed)

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/enable-appliance-autostart.sh -o /tmp/enable-ir.sh
chmod +x /tmp/enable-ir.sh
/tmp/enable-ir.sh admin
```

Or: `sudo enable-instant-replay-autostart admin` (after package install).

## Desktop icon

The Pi desktop shortcut runs **`doctor-pi`** (health check), not a second engine. Production UI comes from **`replay-engine.service`** only.

## Config

`/etc/instant-replay/config.toml` — see [CONFIG.md](CONFIG.md) and `config/default.toml`.

## Verify

```bash
systemctl status replay-engine
pgrep -c -x replay-engine    # expect 1
replay-engine --list-devices
./scripts/doctor-pi.sh
```

## Sign-off

[acceptance/RESULTS-pi.md](acceptance/RESULTS-pi.md) · [HARDWARE_ACCEPTANCE.md](HARDWARE_ACCEPTANCE.md)
