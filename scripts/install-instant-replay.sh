#!/usr/bin/env bash
# One-shot Pi 5 installer — download release, install to /opt/instant-replay, enable autostart.
#
#   curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
#
# Env:
#   INSTANT_REPLAY_TAG=v0.2.0   — GitHub release tag (default: latest)
#   INSTANT_REPLAY_USER=admin   — service user (default: logname)
set -euo pipefail

REPO_SLUG="madgunman/InstaReplayPi"
RAW_BRANCH="${INSTANT_REPLAY_BRANCH:-main}"
TAG="${INSTANT_REPLAY_TAG:-}"
RUN_USER="${INSTANT_REPLAY_USER:-$(logname 2>/dev/null || echo "${USER:-admin}")}"

log() { echo "==> $*"; }

if [ "$(uname -m)" != "aarch64" ]; then
  echo "This installer is for Raspberry Pi 5 (aarch64)." >&2
  exit 1
fi

if [ "$(id -u)" -eq 0 ]; then
  echo "Run as your normal user (e.g. admin), not root. The script uses sudo when needed." >&2
  exit 1
fi

log "System packages (GStreamer, OpenGL/EGL for native UI)..."
sudo apt-get update -qq
sudo apt-get install -y \
  curl ca-certificates \
  gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
  gstreamer1.0-tools \
  libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 \
  libegl1 libgles2 \
  v4l-utils

api="https://api.github.com/repos/${REPO_SLUG}/releases"
if [ -n "$TAG" ]; then
  api="${api}/tags/${TAG}"
else
  api="${api}/latest"
fi

log "GitHub release..."
json="$(curl -fsSL "$api")" || {
  echo "No release found. Try: INSTANT_REPLAY_TAG=v0.2.0" >&2
  exit 1
}

asset_url="$(echo "$json" | grep -oE 'https://[^"]+pi5-aarch64\.tar\.gz' | head -1)"
if [ -z "$asset_url" ]; then
  echo "No pi5-aarch64.tar.gz in release." >&2
  exit 1
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

log "Download $(basename "$asset_url")"
curl -fsSL -o "$tmpdir/pkg.tar.gz" "$asset_url"
tar -xzf "$tmpdir/pkg.tar.gz" -C "$tmpdir"
pkg="$(find "$tmpdir" -maxdepth 1 -type d -name 'InstantReplay-*' | head -1)"
if [ -z "$pkg" ] || [ ! -x "$pkg/bin/replay-engine" ]; then
  echo "Invalid package (missing bin/replay-engine)." >&2
  exit 1
fi

log "Latest install-on-pi.sh from GitHub ($RAW_BRANCH)"
curl -fsSL \
  "https://raw.githubusercontent.com/madgunman/InstaReplayPi/${RAW_BRANCH}/scripts/install-on-pi.sh" \
  -o "$pkg/install-on-pi.sh"
chmod +x "$pkg/install-on-pi.sh"

log "Install to /opt/instant-replay (user: $RUN_USER)"
(cd "$pkg" && sudo ./install-on-pi.sh "$RUN_USER")

cat <<EOF

==============================================
 Instant Replay is installed.

  Operator: native window on Pi touch (starts with replay-engine)
  Config:   sudo nano /etc/instant-replay/config.toml
  Status:   systemctl status replay-engine

  One-time: Desktop Autologin → $RUN_USER, then sudo reboot
==============================================
EOF

if systemctl is-active --quiet replay-engine 2>/dev/null; then
  echo "Service: active"
else
  echo "Service: check journalctl -u replay-engine -n 30"
fi
