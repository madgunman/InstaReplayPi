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

Use **`v0.1.1`** or newer (fixes crash loop / `status=11/SEGV` on boot).

## Fix existing broken install (crash loop / no HTTP)

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
```

Or only re-apply systemd (needs **v0.1.1+** binary for the winit fix):

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/enable-appliance-autostart.sh -o /tmp/e.sh
chmod +x /tmp/e.sh
sudo /tmp/e.sh admin
```

**Requirements:** Desktop **Autologin** for your user (so `DISPLAY=:0` exists). The engine unit starts after `graphical.target`.

If `journalctl` shows `winit` / “event loop outside of the main thread”, upgrade to **v0.1.1+** and re-run the installer.

## After install

```bash
sudo nano /etc/instant-replay/config.toml
systemctl status replay-engine
curl -s http://127.0.0.1:8080/api/health
doctor-pi
```
