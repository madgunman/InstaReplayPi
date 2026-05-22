#!/usr/bin/env bash
# Remove Instant Replay appliance install (keeps /etc/instant-replay and /var/lib by default).
#
#   curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/uninstall-instant-replay.sh | bash
#
# Env:
#   INSTANT_REPLAY_PURGE_CONFIG=1  — also remove /etc/instant-replay
#   INSTANT_REPLAY_PURGE_DATA=1    — also remove /var/lib/instant-replay
set -euo pipefail

OPT_PREFIX="/opt/instant-replay"

log() { echo "==> $*"; }

if [ "$(id -u)" -eq 0 ]; then
  echo "Run as your normal user (e.g. admin), not root. The script uses sudo when needed." >&2
  exit 1
fi

log "Stopping services..."
sudo systemctl stop replay-engine 2>/dev/null || true
sudo systemctl disable replay-engine 2>/dev/null || true
sudo systemctl stop instant-replay-kiosk.service 2>/dev/null || true
sudo systemctl disable instant-replay-kiosk.service 2>/dev/null || true
sudo pkill -x replay-engine 2>/dev/null || true
sleep 1

log "Removing install tree and systemd units..."
sudo rm -rf "$OPT_PREFIX"
sudo rm -f /usr/local/bin/replay-engine /usr/local/bin/instant-replay
sudo rm -f /usr/local/bin/enable-instant-replay-autostart /usr/local/bin/doctor-pi
sudo rm -f /etc/systemd/system/replay-engine.service
sudo rm -rf /etc/systemd/system/replay-engine.service.d
sudo rm -f /etc/systemd/system/instant-replay-kiosk.service \
  /etc/systemd/system/instant-replay-kiosk.service.d/override.conf 2>/dev/null || true
sudo rm -f /usr/share/applications/instant-replay.desktop 2>/dev/null || true

if [ "${INSTANT_REPLAY_PURGE_CONFIG:-0}" = "1" ]; then
  log "Removing /etc/instant-replay"
  sudo rm -rf /etc/instant-replay
fi

if [ "${INSTANT_REPLAY_PURGE_DATA:-0}" = "1" ]; then
  log "Removing /var/lib/instant-replay"
  sudo rm -rf /var/lib/instant-replay
fi

sudo systemctl daemon-reload

cat <<EOF

Instant Replay uninstalled.
  Kept: /etc/instant-replay (unless INSTANT_REPLAY_PURGE_CONFIG=1)
  Kept: /var/lib/instant-replay (unless INSTANT_REPLAY_PURGE_DATA=1)

Reinstall:
  INSTANT_REPLAY_TAG=v0.3.0 curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
EOF
