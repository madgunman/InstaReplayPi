# GitHub — Pi install from [InstaReplayPi](https://github.com/madgunman/InstaReplayPi)

## One command (recommended)

On the Pi as user **admin** (not root):

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
```

This will:

1. Install apt packages (GStreamer, EGL/GLES for native UI)
2. Download the latest **release** binary (`pi5-aarch64` tarball)
3. Install to `/opt/instant-replay`
4. Enable `replay-engine` at boot (native operator window + HDMI)

Then set **Desktop Autologin** → your user → `sudo reboot`.

### Options

```bash
INSTANT_REPLAY_USER=admin INSTANT_REPLAY_TAG=v0.2.0 \
  curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
```

## Fix existing broken install

```bash
INSTANT_REPLAY_USER=admin INSTANT_REPLAY_TAG=v0.2.0 \
  curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
```

Or only re-apply systemd:

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/enable-appliance-autostart.sh -o /tmp/e.sh
chmod +x /tmp/e.sh
sudo /tmp/e.sh admin
```

**Requirements:** Desktop **Autologin** for your user (so `DISPLAY=:0` exists). Engine starts after `graphical.target`.

## After install

```bash
sudo nano /etc/instant-replay/config.toml
systemctl status replay-engine
journalctl -u replay-engine -f
doctor-pi
```

Look for `Native operator UI running` in the journal. The operator window opens on the Pi touch display automatically.
