#!/usr/bin/env bash
# Re-apply autostart + systemd overrides (same as install-on-pi.sh drop-ins).
# Usage: ./enable-appliance-autostart.sh [username]
set -euo pipefail

OPT_PREFIX="/opt/instant-replay"
RUN_USER="${1:-${SUDO_USER:-$USER}}"
if [ -z "$RUN_USER" ] || [ "$RUN_USER" = "root" ]; then
  RUN_USER="$(logname 2>/dev/null || echo admin)"
fi

if [ ! -f /etc/systemd/system/replay-engine.service ]; then
  echo "Not installed. Run: curl -fsSL .../install-instant-replay.sh | bash" >&2
  exit 1
fi

ENGINE_BIN="$OPT_PREFIX/bin/replay-engine"
if [ ! -x "$ENGINE_BIN" ]; then
  ENGINE_BIN="/usr/local/bin/replay-engine"
fi

sudo mkdir -p /etc/systemd/system/replay-engine.service.d
sudo rm -f /etc/systemd/system/replay-engine.service.d/user.conf
sudo tee /etc/systemd/system/replay-engine.service.d/override.conf >/dev/null <<EOF
[Service]
User=$RUN_USER
ExecStart=
ExecStart=$ENGINE_BIN --appliance
Environment=DISPLAY=:0
Environment=GST_PLUGIN_PATH=/usr/lib/aarch64-linux-gnu/gstreamer-1.0
EOF

sudo systemctl disable instant-replay-kiosk.service 2>/dev/null || true
sudo chown -R "$RUN_USER:$RUN_USER" /var/lib/instant-replay
sudo systemctl daemon-reload
sudo systemctl enable replay-engine
sudo systemctl restart replay-engine

echo "Autostart enabled for $RUN_USER (native operator UI in replay-engine)."
