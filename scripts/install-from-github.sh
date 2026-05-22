#!/usr/bin/env bash
# Install Instant Replay on a Raspberry Pi 5 from GitHub.
#
# Usage (on the Pi, as user pi):
#   curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-from-github.sh | bash
#   # or after clone:
#   ./scripts/install-from-github.sh [--build | --release [TAG]]
#
# --release   Download GitHub Release tarball (default if a release exists)
# --build     git pull + build on device (needs Rust toolchain from install-deps)
# TAG         Optional release tag, e.g. v0.1.0 (default: latest)
set -euo pipefail

REPO="https://github.com/madgunman/InstaReplayPi.git"
REPO_SLUG="madgunman/InstaReplayPi"
INSTALL_DIR="${INSTANT_REPLAY_INSTALL_DIR:-$HOME/InstaReplayPi}"
MODE="release"
TAG=""

while [ $# -gt 0 ]; do
  case "$1" in
    --build) MODE="build"; shift ;;
    --release) MODE="release"; shift ;;
    -h|--help)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    v*) TAG="$1"; shift ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

log() { echo "==> $*"; }

if [ "$(uname -m)" != "aarch64" ]; then
  echo "This installer is for Raspberry Pi 5 (aarch64). Detected: $(uname -m)" >&2
  exit 1
fi

install_release() {
  local api="https://api.github.com/repos/${REPO_SLUG}/releases"
  if [ -n "$TAG" ]; then
    api="${api}/tags/${TAG}"
  else
    api="${api}/latest"
  fi
  log "Fetching release metadata from GitHub"
  local json
  json="$(curl -fsSL "$api")" || {
    echo "No release found. Push a tag (v*) and wait for CI, or run: $0 --build" >&2
    exit 1
  }
  local asset_url
  asset_url="$(echo "$json" | grep -oE 'https://[^"]+pi5-aarch64\.tar\.gz' | head -1)"
  if [ -z "$asset_url" ]; then
    echo "Release has no pi5-aarch64.tar.gz asset. Use --build or check Actions." >&2
    exit 1
  fi
  local tmp
  tmp="$(mktemp -d)"
  log "Downloading $asset_url"
  curl -fsSL -o "$tmp/InstantReplay.tar.gz" "$asset_url"
  tar -xzf "$tmp/InstantReplay.tar.gz" -C "$tmp"
  local dir
  dir="$(find "$tmp" -maxdepth 1 -type d -name 'InstantReplay-*' | head -1)"
  if [ -z "$dir" ]; then
    echo "Tarball layout unexpected" >&2
    exit 1
  fi
  log "Running install-on-pi.sh"
  (cd "$dir" && ./install-on-pi.sh)
  rm -rf "$tmp"
}

install_build() {
  if [ -d "$INSTALL_DIR/.git" ]; then
    log "Updating $INSTALL_DIR"
    git -C "$INSTALL_DIR" pull --ff-only
  else
    log "Cloning $REPO → $INSTALL_DIR"
    git clone "$REPO" "$INSTALL_DIR"
  fi
  if [ -x "$INSTALL_DIR/scripts/install-deps-raspberry-pi.sh" ]; then
    log "Installing Pi dependencies"
    "$INSTALL_DIR/scripts/install-deps-raspberry-pi.sh"
  fi
  log "Building and packaging on Pi"
  (cd "$INSTALL_DIR" && ./scripts/package-pi.sh)
  local dir
  dir="$(find "$INSTALL_DIR/dist" -maxdepth 1 -type d -name 'InstantReplay-*-pi5-aarch64' | head -1)"
  (cd "$dir" && ./install-on-pi.sh)
}

case "$MODE" in
  release) install_release ;;
  build) install_build ;;
esac

echo ""
echo "Next steps:"
echo "  sudo nano /etc/instant-replay/config.toml"
echo "  sudo systemctl start replay-engine"
echo "  sudo systemctl enable --now instant-replay-kiosk   # optional touch UI"
if [ -x "$INSTALL_DIR/scripts/doctor-pi.sh" ]; then
  echo "  $INSTALL_DIR/scripts/doctor-pi.sh"
else
  echo "  git clone $REPO && ./scripts/doctor-pi.sh"
fi
echo "  Touch UI: http://127.0.0.1:8080"
