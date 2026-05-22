# GitHub — Pi install from [InstaReplayPi](https://github.com/madgunman/InstaReplayPi)

## One command (recommended)

On the Pi as user **admin** (not root):

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
```

This will:

1. Install apt packages (GStreamer, Chromium)
2. Download the latest **release** binary (`pi5-aarch64` tarball)
3. Install to `/opt/instant-replay`
4. Fix systemd (`User=admin`, correct binary path)
5. Enable engine + touch kiosk at boot

Then set **Desktop Autologin** → your user → `sudo reboot`.

### Options

```bash
INSTANT_REPLAY_USER=admin INSTANT_REPLAY_TAG=v0.1.0 \
  curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
```

Until `v0.1.1` is released, `v0.1.0` binary + latest `install-on-pi.sh` from GitHub is fine.

## Fix existing broken install (v0.1.0)

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
```

Or only re-apply systemd:

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/enable-appliance-autostart.sh -o /tmp/e.sh
chmod +x /tmp/e.sh
/tmp/e.sh admin
```

## After install

```bash
sudo nano /etc/instant-replay/config.toml
systemctl status replay-engine
curl -s http://127.0.0.1:8080/api/health
doctor-pi
```
