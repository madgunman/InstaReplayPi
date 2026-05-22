#!/usr/bin/env bash
# Install binaries + systemd units from a package-pi dist folder or repo root.
# Usage: ./install-on-pi.sh [username]
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
# Dist tarball: install-on-pi.sh at package root next to bin/
if [ -x "$DIR/bin/replay-engine" ]; then
  PKG_ROOT="$DIR"
else
  echo "Run from extracted package root (contains bin/replay-engine), e.g.:" >&2
  echo "  cd dist/InstantReplay-*-pi5-aarch64 && ./install-on-pi.sh" >&2
  exit 1
fi

RUN_USER="${1:-${SUDO_USER:-$USER}}"
if [ -z "$RUN_USER" ] || [ "$RUN_USER" = "root" ]; then
  RUN_USER="$(logname 2>/dev/null || echo pi)"
fi

echo "==> Installing Instant Replay for user: $RUN_USER"

sudo cp "$PKG_ROOT/bin/replay-engine" /usr/local/bin/
sudo cp "$PKG_ROOT/bin/instant-replay" /usr/local/bin/
sudo chmod +x /usr/local/bin/replay-engine /usr/local/bin/instant-replay

sudo mkdir -p /etc/instant-replay /var/lib/instant-replay /usr/share/instant-replay
if [ ! -f /etc/instant-replay/config.toml ]; then
  if [ -f "$PKG_ROOT/etc/instant-replay/config.toml.example" ]; then
    sudo cp "$PKG_ROOT/etc/instant-replay/config.toml.example" /etc/instant-replay/config.toml
  fi
fi

sudo cp "$PKG_ROOT/systemd/replay-engine.service" /etc/systemd/system/
sudo cp "$PKG_ROOT/systemd/instant-replay-kiosk.service" /etc/systemd/system/

if [ -f "$PKG_ROOT/replay-engine.default" ]; then
  sudo cp "$PKG_ROOT/replay-engine.default" /etc/default/replay-engine
fi
if ! grep -q INSTANT_REPLAY_V4L2_IO_MODE /etc/default/replay-engine 2>/dev/null; then
  echo 'INSTANT_REPLAY_V4L2_IO_MODE=dmabuf' | sudo tee -a /etc/default/replay-engine >/dev/null
fi

for desk in \
  "$PKG_ROOT/packaging/pi/instant-replay.desktop" \
  "$PKG_ROOT/share/instant-replay.desktop"; do
  if [ -f "$desk" ]; then
    sudo cp "$desk" /usr/share/instant-replay/instant-replay.desktop
    break
  fi
done

if [ -f "$PKG_ROOT/scripts/start-instant-replay-ui.sh" ]; then
  sudo cp "$PKG_ROOT/scripts/start-instant-replay-ui.sh" /usr/local/bin/start-instant-replay-ui
  sudo chmod +x /usr/local/bin/start-instant-replay-ui
fi

ENABLE_SCRIPT=""
for s in "$PKG_ROOT/scripts/enable-appliance-autostart.sh" "$DIR/scripts/enable-appliance-autostart.sh"; do
  if [ -f "$s" ]; then
    ENABLE_SCRIPT="$s"
    sudo cp "$s" /usr/local/bin/enable-instant-replay-autostart
    sudo chmod +x /usr/local/bin/enable-instant-replay-autostart
    break
  fi
done

echo "==> Binaries and systemd units installed"
if [ -n "$ENABLE_SCRIPT" ]; then
  "$ENABLE_SCRIPT" "$RUN_USER"
else
  echo "Run autostart setup: ./scripts/enable-appliance-autostart.sh $RUN_USER"
fi
