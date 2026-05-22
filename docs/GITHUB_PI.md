# GitHub — Pi install from [InstaReplayPi](https://github.com/madgunman/InstaReplayPi)

**https://github.com/madgunman/InstaReplayPi.git**

## On the Pi (release v0.1.0+)

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-from-github.sh -o /tmp/install-ir.sh
chmod +x /tmp/install-ir.sh
/tmp/install-ir.sh --release
```

Install runs `install-on-pi.sh` + **Option B autostart** (`enable-appliance-autostart.sh`) for your username.

Then:

1. `sudo nano /etc/instant-replay/config.toml` (camera + SSD buffer path)
2. **Desktop Autologin** for your user (`admin`, etc.)
3. `sudo reboot`

## Build on Pi

```bash
git clone https://github.com/madgunman/InstaReplayPi.git ~/InstaReplayPi
cd ~/InstaReplayPi
./scripts/install-from-github.sh --build
```

## Autostart script only

```bash
./scripts/enable-appliance-autostart.sh admin
```

Branch `feature/appliance-autostart` adds this to the default install path.

## Updating

```bash
/tmp/install-ir.sh --release v0.2.0
sudo systemctl restart replay-engine instant-replay-kiosk
```
