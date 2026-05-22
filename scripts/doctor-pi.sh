#!/usr/bin/env bash
# Pi 5 pre-match health check — run on the appliance before a tournament.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUFFER="${BUFFER_PATH:-/var/lib/instant-replay/buffer}"
FAIL=0

warn() { echo "WARN: $*"; }
fail() { echo "FAIL: $*"; }
ok() { echo "OK: $*"; }

echo "Instant Replay Pi doctor"
echo "========================"

if [ "$(uname -m)" != "aarch64" ]; then
  warn "Not aarch64 ($(uname -m)) — intended for Raspberry Pi 5"
fi

if command -v gst-inspect-1.0 >/dev/null 2>&1; then
  for plug in v4l2src splitmuxsink uridecodebin; do
    if gst-inspect-1.0 "$plug" >/dev/null 2>&1; then
      ok "GStreamer plugin $plug"
    else
      fail "Missing GStreamer plugin: $plug"
    fi
  done
else
  fail "gst-inspect-1.0 not found"
fi

if [ -e /dev/video0 ]; then
  ok "/dev/video0 present"
else
  warn "/dev/video0 missing (camera not connected?)"
fi

if mountpoint -q "$(dirname "$BUFFER")" 2>/dev/null; then
  ok "Buffer parent mounted: $(dirname "$BUFFER")"
elif [ -d "$(dirname "$BUFFER")" ]; then
  ok "Buffer directory exists: $(dirname "$BUFFER")"
else
  warn "Buffer parent not mounted — use USB3 SSD at /var/lib/instant-replay"
fi

mkdir -p "$BUFFER" 2>/dev/null || true
if touch "$BUFFER/.doctor-write" 2>/dev/null; then
  rm -f "$BUFFER/.doctor-write"
  ok "Buffer path writable: $BUFFER"
else
  fail "Cannot write to buffer path: $BUFFER"
fi

if [ -f /etc/instant-replay/config.toml ]; then
  ok "Config: /etc/instant-replay/config.toml"
  if grep -q '^\[operator\]' /etc/instant-replay/config.toml 2>/dev/null; then
    ok "Operator UI config present"
  else
    warn "Missing [operator] in config — add from config.toml.example"
  fi
else
  warn "No /etc/instant-replay/config.toml — using built-in defaults"
fi

if systemctl is-active replay-engine >/dev/null 2>&1; then
  ok "replay-engine.service active"
  if journalctl -u replay-engine -n 20 --no-pager 2>/dev/null | grep -q "Native operator UI"; then
    ok "Native operator UI started (see journal)"
  fi
else
  warn "replay-engine.service not active (start with: sudo systemctl start replay-engine)"
fi

if [ -x "$ROOT/scripts/mvp_accept-full.sh" ] || [ -x "$ROOT/target/release/replay-engine" ]; then
  echo ""
  echo "Optional: run automated smoke (test pattern):"
  echo "  cd $ROOT && ./scripts/mvp_accept-full.sh"
fi

echo ""
if [ "$FAIL" -eq 0 ]; then
  echo "Doctor: PASS"
  exit 0
fi
echo "Doctor: FAIL (see above)"
exit 1
