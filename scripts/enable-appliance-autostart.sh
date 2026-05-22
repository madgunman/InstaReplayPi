#!/usr/bin/env bash
# Option B: boot / login autostart — replay-engine + Chromium kiosk touch UI.
# Usage: sudo ./scripts/enable-appliance-autostart.sh [username]
# Example: ./enable-appliance-autostart.sh admin
set -euo pipefail

RUN_USER="${1:-${SUDO_USER:-$USER}}"
if [ -z "$RUN_USER" ] || [ "$RUN_USER" = "root" ]; then
  RUN_USER="$(logname 2>/dev/null || echo pi)"
fi

log() { echo "==> $*"; }

if [ "$(uname -m)" != "aarch64" ]; then
  echo "Warning: expected aarch64 (Pi 5); got $(uname -m)" >&2
fi

for unit in replay-engine.service instant-replay-kiosk.service; do
  if [ ! -f "/etc/systemd/system/$unit" ]; then
    echo "Missing /etc/systemd/system/$unit — run install-on-pi.sh first." >&2
    exit 1
  fi
done

if ! command -v chromium >/dev/null 2>&1 && ! command -v chromium-browser >/dev/null 2>&1; then
  echo "Warning: chromium not found — kiosk may fail. Install: sudo apt install chromium" >&2
fi

log "Config + buffer ownership for $RUN_USER"
sudo mkdir -p /etc/instant-replay /var/lib/instant-replay
if [ ! -f /etc/instant-replay/config.toml ]; then
  for example in \
    /usr/share/instant-replay/config.toml.example \
    "$HOME/InstaReplayPi/config/default.toml"; do
    if [ -f "$example" ]; then
      sudo cp "$example" /etc/instant-replay/config.toml
      break
    fi
  done
fi
sudo chown -R "$RUN_USER:$RUN_USER" /var/lib/instant-replay

log "systemd: services run as $RUN_USER"
sudo mkdir -p /etc/systemd/system/replay-engine.service.d
sudo tee /etc/systemd/system/replay-engine.service.d/user.conf >/dev/null <<EOF
[Service]
User=$RUN_USER
EOF

sudo mkdir -p /etc/systemd/system/instant-replay-kiosk.service.d
sudo tee /etc/systemd/system/instant-replay-kiosk.service.d/user.conf >/dev/null <<EOF
[Service]
User=$RUN_USER
Environment=DISPLAY=:0
EOF

log "Enable replay-engine + instant-replay-kiosk at boot"
sudo systemctl daemon-reload
sudo systemctl enable replay-engine instant-replay-kiosk

if systemctl is-active --quiet graphical.target 2>/dev/null; then
  log "Starting services now (desktop session active)"
  sudo systemctl restart replay-engine
  sleep 2
  sudo systemctl restart instant-replay-kiosk || true
else
  log "Engine will start at boot; kiosk after graphical login"
  sudo systemctl start replay-engine || true
fi

install_desktop() {
  local desktop_src="$1"
  [ -f "$desktop_src" ] || return 0
  sudo mkdir -p /usr/share/applications
  sudo cp "$desktop_src" /usr/share/applications/instant-replay.desktop
  sudo chmod 644 /usr/share/applications/instant-replay.desktop
  local home
  home="$(getent passwd "$RUN_USER" | cut -d: -f6)"
  if [ -n "$home" ] && [ -d "$home" ]; then
    mkdir -p "$home/Desktop"
    cp /usr/share/applications/instant-replay.desktop "$home/Desktop/" 2>/dev/null || \
      sudo cp /usr/share/applications/instant-replay.desktop "$home/Desktop/"
    sudo chown "$RUN_USER:$RUN_USER" "$home/Desktop/instant-replay.desktop" 2>/dev/null || true
    chmod +x "$home/Desktop/instant-replay.desktop" 2>/dev/null || true
  fi
}

for candidate in \
  "$(dirname "$0")/../packaging/pi/instant-replay.desktop" \
  "$(dirname "$0")/../../packaging/pi/instant-replay.desktop" \
  "/usr/share/instant-replay/instant-replay.desktop"; do
  if [ -f "$candidate" ]; then
    install_desktop "$candidate"
    break
  fi
done

log "Status"
systemctl is-enabled replay-engine instant-replay-kiosk 2>/dev/null || true
if curl -sfS --max-time 3 http://127.0.0.1:8080/api/health >/dev/null 2>&1; then
  echo "  HTTP health: OK"
else
  echo "  HTTP health: not ready (check: journalctl -u replay-engine -n 30)"
fi

cat <<EOF

==============================================
 Instant Replay autostart enabled (Option B)

 Services: replay-engine + instant-replay-kiosk
 Run as:   $RUN_USER

 One-time on Raspberry Pi OS:
   Settings → System → Auto Login → Desktop → $RUN_USER
   Then: sudo reboot

 After reboot: live on audience HDMI + touch UI at http://127.0.0.1:8080

 Logs:
   journalctl -u replay-engine -f
   journalctl -u instant-replay-kiosk -f
==============================================
EOF
