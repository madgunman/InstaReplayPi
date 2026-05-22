#!/usr/bin/env bash
# Raspberry Pi 5 (aarch64) tarball — build natively on the Pi or on ubuntu-24.04-arm CI.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/version.sh
source "$ROOT/scripts/lib/version.sh"
VERSION="$(get_workspace_version "$ROOT")"
STAMP="${VERSION}-pi5-aarch64"
DIST_NAME="InstantReplay-${STAMP}"
DIST="$ROOT/dist/${DIST_NAME}"

if [[ "$(uname -m)" != "aarch64" ]]; then
  echo "Pi package is intended to be built ON aarch64 (Pi 5 or ubuntu-24.04-arm CI)."
  echo "For cross-compile from x86_64, install rustup target aarch64-unknown-linux-gnu."
fi

echo "==> Building replay-engine (release)"
(cd "$ROOT" && cargo build -p replay-engine --release)

rm -rf "$DIST"
mkdir -p "$DIST/bin" "$DIST/scripts" "$DIST/assets/touch" "$DIST/systemd" \
  "$DIST/share/doc/instant-replay" "$DIST/etc/instant-replay"

cp "$ROOT/target/release/replay-engine" "$DIST/bin/"
cp "$ROOT/packaging/lib/gstreamer-env.sh" "$DIST/scripts/"
cp "$ROOT/systemd/replay-engine.service" "$DIST/systemd/"
cp "$ROOT/systemd/instant-replay-kiosk.service" "$DIST/systemd/"
cp "$ROOT/config/default.toml" "$DIST/etc/instant-replay/config.toml.example"
cp -R "$ROOT/assets/touch/." "$DIST/assets/touch/"
cp "$ROOT/packaging/linux/replay-engine.default" "$DIST/replay-engine.default" 2>/dev/null || true
cp "$ROOT/docs/PI_DEPLOYMENT.md" "$DIST/share/doc/instant-replay/OPERATOR-PI.md"
cp "$ROOT/docs/OPERATOR.md" "$DIST/share/doc/instant-replay/"
cp "$ROOT/docs/CONFIG.md" "$DIST/share/doc/instant-replay/"
cp "$ROOT/docs/PI_ONLY.md" "$DIST/share/doc/instant-replay/"
cp "$ROOT/docs/acceptance/RESULTS-pi.md" "$DIST/share/doc/instant-replay/" 2>/dev/null || true
cp "$ROOT/docs/OPERATOR.md" "$DIST/OPERATOR.md"
cp "$ROOT/docs/CONFIG.md" "$DIST/CONFIG.md"
cp "$ROOT/scripts/doctor-pi.sh" "$DIST/scripts/"
cp "$ROOT/scripts/enable-appliance-autostart.sh" "$DIST/scripts/"
cp "$ROOT/scripts/start-instant-replay-ui.sh" "$DIST/scripts/"
cp "$ROOT/scripts/install-instant-replay.sh" "$DIST/scripts/"
cp "$ROOT/scripts/install-on-pi.sh" "$DIST/"
cp "$ROOT/packaging/pi/instant-replay.desktop" "$DIST/share/"
mkdir -p "$DIST/packaging/pi"
cp "$ROOT/packaging/pi/instant-replay.desktop" "$DIST/packaging/pi/"
echo "$VERSION" >"$DIST/VERSION"

cat > "$DIST/bin/instant-replay" <<'LAUNCHER'
#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=/dev/null
source "$DIR/scripts/gstreamer-env.sh"
exec "$DIR/bin/replay-engine" "$@"
LAUNCHER
chmod +x "$DIST/bin/instant-replay" "$DIST/bin/replay-engine" \
  "$DIST/scripts/doctor-pi.sh" \
  "$DIST/scripts/enable-appliance-autostart.sh" \
  "$DIST/scripts/start-instant-replay-ui.sh" \
  "$DIST/install-on-pi.sh"

cat > "$DIST/README.txt" <<EOF
Instant Replay ${STAMP} (Raspberry Pi 5) version ${VERSION}

GitHub: https://github.com/madgunman/InstaReplayPi

Single daemon: replay-engine --appliance
Touch UI: http://127.0.0.1:8080 (enable instant-replay-kiosk.service for Chromium fullscreen)

One-command install on Pi:
  curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash

Or from tarball: ./install-on-pi.sh [username]

Autostart (Option B): included via install-on-pi.sh
  Set Desktop Autologin for your user, then reboot.

Mount USB3 SSD at /var/lib/instant-replay for buffer storage.
EOF

TARBALL="$ROOT/dist/${DIST_NAME}.tar.gz"
tar -C "$ROOT/dist" -czf "$TARBALL" "$DIST_NAME"
echo "Pi tarball: $TARBALL"
