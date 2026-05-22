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

sudo apt-get install -y -qq libegl1 libgles2 2>/dev/null || \
  sudo apt-get install -y -qq libegl1-mesa libgles2-mesa 2>/dev/null || true

sudo mkdir -p "$OPT_PREFIX"
sudo cp -a "$PKG_ROOT"/. "$OPT_PREFIX/"
sudo chmod +x "$OPT_PREFIX/bin/replay-engine" \
  "$OPT_PREFIX/scripts/"*.sh 2>/dev/null || true

sudo ln -sf "$OPT_PREFIX/bin/replay-engine" /usr/local/bin/replay-engine
if [ -x "$OPT_PREFIX/bin/instant-replay" ]; then
  sudo ln -sf "$OPT_PREFIX/bin/instant-replay" /usr/local/bin/instant-replay
fi

sudo mkdir -p /etc/instant-replay /var/lib/instant-replay /usr/share/instant-replay
CONFIG_EX="$OPT_PREFIX/etc/instant-replay/config.toml.example"
if [ -f "$CONFIG_EX" ]; then
  sudo cp -f "$CONFIG_EX" /usr/share/instant-replay/config.toml.example
  if [ ! -f /etc/instant-replay/config.toml ]; then
    sudo cp "$CONFIG_EX" /etc/instant-replay/config.toml
  elif ! grep -q '^\[input\]' /etc/instant-replay/config.toml 2>/dev/null; then
    echo "==> Replacing incomplete /etc/instant-replay/config.toml (backup: config.toml.bak)"
    sudo cp -a /etc/instant-replay/config.toml /etc/instant-replay/config.toml.bak
    sudo cp "$CONFIG_EX" /etc/instant-replay/config.toml
  fi
fi
sudo chown root:"$RUN_USER" /etc/instant-replay 2>/dev/null || true
sudo chmod 775 /etc/instant-replay 2>/dev/null || true
if [ -f /etc/instant-replay/config.toml ]; then
  sudo chown root:"$RUN_USER" /etc/instant-replay/config.toml
  sudo chmod 664 /etc/instant-replay/config.toml
fi

sudo cp "$OPT_PREFIX/systemd/replay-engine.service" /etc/systemd/system/
sudo systemctl disable instant-replay-kiosk.service 2>/dev/null || true
sudo rm -f /etc/systemd/system/instant-replay-kiosk.service \
  /etc/systemd/system/instant-replay-kiosk.service.d/override.conf 2>/dev/null || true

if [ -f "$OPT_PREFIX/replay-engine.default" ]; then
  sudo cp "$OPT_PREFIX/replay-engine.default" /etc/default/replay-engine
fi
if ! grep -q INSTANT_REPLAY_V4L2_IO_MODE /etc/default/replay-engine 2>/dev/null; then
  echo 'INSTANT_REPLAY_V4L2_IO_MODE=dmabuf' | sudo tee -a /etc/default/replay-engine >/dev/null
fi

sudo mkdir -p /etc/systemd/system/replay-engine.service.d
sudo rm -f /etc/systemd/system/replay-engine.service.d/user.conf
sudo tee /etc/systemd/system/replay-engine.service.d/override.conf >/dev/null <<EOF
[Service]
User=$RUN_USER
ExecStart=
ExecStart=$OPT_PREFIX/bin/replay-engine --appliance
Environment=DISPLAY=:0
Environment=GST_PLUGIN_PATH=/usr/lib/aarch64-linux-gnu/gstreamer-1.0
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
[ -f "$OPT_PREFIX/scripts/doctor-pi.sh" ] && \
  sudo ln -sf "$OPT_PREFIX/scripts/doctor-pi.sh" /usr/local/bin/doctor-pi

sudo chown -R "$RUN_USER:$RUN_USER" /var/lib/instant-replay

sudo systemctl daemon-reload
sudo systemctl enable replay-engine

if systemctl is-active --quiet graphical.target 2>/dev/null; then
  sudo systemctl restart replay-engine
else
  sudo systemctl start replay-engine 2>/dev/null || true
fi

echo ""
echo "=============================================="
echo " Instant Replay installed"
echo "  Prefix:   $OPT_PREFIX"
echo "  User:     $RUN_USER"
echo "  Config:   /etc/instant-replay/config.toml"
echo "  Operator: native window on Pi touch (egui)"
echo ""
echo "  Set Desktop Autologin → $RUN_USER, then: sudo reboot"
echo "  Status:   systemctl status replay-engine"
echo "  Logs:     journalctl -u replay-engine -f"
echo "=============================================="

if systemctl is-active --quiet replay-engine 2>/dev/null; then
  echo "Service: active"
else
  echo "Service not active — check: journalctl -u replay-engine -n 40"
fi
