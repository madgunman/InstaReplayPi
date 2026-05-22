#!/usr/bin/env bash
# Install Instant Replay under /opt/instant-replay and enable autostart.
# Usage: sudo ./install-on-pi.sh [username]
# Run from extracted tarball root (contains bin/replay-engine).
set -euo pipefail

OPT_PREFIX="/opt/instant-replay"
DIR="$(cd "$(dirname "$0")" && pwd)"

if [ -x "$DIR/bin/replay-engine" ]; then
  PKG_ROOT="$DIR"
else
  echo "Run from package root (contains bin/replay-engine)." >&2
  exit 1
fi

RUN_USER="${1:-${SUDO_USER:-$USER}}"
if [ -z "$RUN_USER" ] || [ "$RUN_USER" = "root" ]; then
  RUN_USER="$(logname 2>/dev/null || echo admin)"
fi

if ! id "$RUN_USER" &>/dev/null; then
  echo "User '$RUN_USER' does not exist." >&2
  exit 1
fi

echo "==> Installing Instant Replay to $OPT_PREFIX (user: $RUN_USER)"

sudo mkdir -p "$OPT_PREFIX"
sudo cp -a "$PKG_ROOT"/. "$OPT_PREFIX/"
sudo chmod +x "$OPT_PREFIX/bin/replay-engine" \
  "$OPT_PREFIX/scripts/"*.sh 2>/dev/null || true

sudo ln -sf "$OPT_PREFIX/bin/replay-engine" /usr/local/bin/replay-engine
if [ -x "$OPT_PREFIX/bin/instant-replay" ]; then
  sudo ln -sf "$OPT_PREFIX/bin/instant-replay" /usr/local/bin/instant-replay
fi

sudo mkdir -p /etc/instant-replay /var/lib/instant-replay /usr/share/instant-replay
if [ ! -f /etc/instant-replay/config.toml ]; then
  if [ -f "$OPT_PREFIX/etc/instant-replay/config.toml.example" ]; then
    sudo cp "$OPT_PREFIX/etc/instant-replay/config.toml.example" /etc/instant-replay/config.toml
  fi
fi
sudo cp -f "$OPT_PREFIX/etc/instant-replay/config.toml.example" /usr/share/instant-replay/ 2>/dev/null || true

sudo cp "$OPT_PREFIX/systemd/replay-engine.service" /etc/systemd/system/
sudo cp "$OPT_PREFIX/systemd/instant-replay-kiosk.service" /etc/systemd/system/

if [ -f "$OPT_PREFIX/replay-engine.default" ]; then
  sudo cp "$OPT_PREFIX/replay-engine.default" /etc/default/replay-engine
fi
if ! grep -q INSTANT_REPLAY_V4L2_IO_MODE /etc/default/replay-engine 2>/dev/null; then
  echo 'INSTANT_REPLAY_V4L2_IO_MODE=dmabuf' | sudo tee -a /etc/default/replay-engine >/dev/null
fi

# Single drop-in: correct user + binary path (fixes v0.1.0 User=pi and /usr/bin/instant-replay bugs).
sudo mkdir -p /etc/systemd/system/replay-engine.service.d
sudo rm -f /etc/systemd/system/replay-engine.service.d/user.conf
sudo tee /etc/systemd/system/replay-engine.service.d/override.conf >/dev/null <<EOF
[Service]
User=$RUN_USER
ExecStart=
ExecStart=$OPT_PREFIX/bin/replay-engine --appliance
Environment=GST_PLUGIN_PATH=/usr/lib/aarch64-linux-gnu/gstreamer-1.0
EOF

sudo mkdir -p /etc/systemd/system/instant-replay-kiosk.service.d
sudo rm -f /etc/systemd/system/instant-replay-kiosk.service.d/user.conf
sudo tee /etc/systemd/system/instant-replay-kiosk.service.d/override.conf >/dev/null <<EOF
[Service]
User=$RUN_USER
Environment=DISPLAY=:0
EOF

if [ -f "$OPT_PREFIX/share/instant-replay.desktop" ] || [ -f "$OPT_PREFIX/packaging/pi/instant-replay.desktop" ]; then
  desk="$OPT_PREFIX/share/instant-replay.desktop"
  [ -f "$desk" ] || desk="$OPT_PREFIX/packaging/pi/instant-replay.desktop"
  sudo cp "$desk" /usr/share/applications/instant-replay.desktop
  home="$(getent passwd "$RUN_USER" | cut -d: -f6)"
  if [ -n "$home" ]; then
    sudo mkdir -p "$home/Desktop"
    sudo cp /usr/share/applications/instant-replay.desktop "$home/Desktop/"
    sudo chown "$RUN_USER:$RUN_USER" "$home/Desktop/instant-replay.desktop"
    sudo chmod +x "$home/Desktop/instant-replay.desktop"
  fi
fi

[ -f "$OPT_PREFIX/scripts/enable-appliance-autostart.sh" ] && \
  sudo ln -sf "$OPT_PREFIX/scripts/enable-appliance-autostart.sh" /usr/local/bin/enable-instant-replay-autostart
[ -f "$OPT_PREFIX/scripts/start-instant-replay-ui.sh" ] && \
  sudo ln -sf "$OPT_PREFIX/scripts/start-instant-replay-ui.sh" /usr/local/bin/start-instant-replay-ui
[ -f "$OPT_PREFIX/scripts/doctor-pi.sh" ] && \
  sudo ln -sf "$OPT_PREFIX/scripts/doctor-pi.sh" /usr/local/bin/doctor-pi

sudo chown -R "$RUN_USER:$RUN_USER" /var/lib/instant-replay

sudo systemctl daemon-reload
sudo systemctl enable replay-engine instant-replay-kiosk

if systemctl is-active --quiet graphical.target 2>/dev/null; then
  sudo systemctl restart replay-engine
  sleep 2
  sudo systemctl restart instant-replay-kiosk 2>/dev/null || true
else
  sudo systemctl start replay-engine 2>/dev/null || true
fi

echo ""
echo "=============================================="
echo " Instant Replay installed"
echo "  Prefix:  $OPT_PREFIX"
echo "  User:    $RUN_USER"
echo "  Config:  /etc/instant-replay/config.toml"
echo "  Touch:   http://127.0.0.1:8080"
echo ""
echo "  Set Desktop Autologin → $RUN_USER, then: sudo reboot"
echo "  Status:  systemctl status replay-engine"
echo "  Logs:    journalctl -u replay-engine -f"
echo "=============================================="

if curl -sfS --max-time 5 http://127.0.0.1:8080/api/health >/dev/null 2>&1; then
  echo "HTTP health: OK"
else
  echo "HTTP not ready yet — check: journalctl -u replay-engine -n 40"
fi
