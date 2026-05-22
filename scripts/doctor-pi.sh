#!/usr/bin/env bash
# Pi 5 pre-match health check — run on the appliance before a tournament.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="${HTTP_BASE:-http://127.0.0.1:8080}"
BUFFER="${BUFFER_PATH:-/var/lib/instant-replay/buffer}"
FAIL=0

warn() { echo "WARN: $*"; }
fail() { echo "FAIL: $*"; FAIL=1; }
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
else
  warn "No /etc/instant-replay/config.toml — using built-in defaults"
fi

if systemctl is-active replay-engine >/dev/null 2>&1; then
  ok "replay-engine.service active"
else
  warn "replay-engine.service not active (start with: sudo systemctl start replay-engine)"
fi

if curl -sfS --max-time 3 "${BASE}/api/health" >/dev/null 2>&1; then
  ok "HTTP touch API reachable at ${BASE}"
  status="$(curl -sfS "${BASE}/api/status" 2>/dev/null || echo '{}')"
  echo "  status: $status"
else
  fail "HTTP not reachable at ${BASE} (engine running with [http] enabled?)"
fi

if [ -x "$ROOT/scripts/mvp_accept-full.sh" ] || [ -x "$ROOT/target/release/replay-engine" ]; then
  echo ""
  echo "Optional: run automated smoke (test pattern, ~2 min):"
  echo "  cd $ROOT && ./scripts/mvp_accept-full.sh"
fi

echo ""
if [ "$FAIL" -eq 0 ]; then
  echo "Doctor: PASS"
  exit 0
fi
echo "Doctor: FAIL (see above)"
exit 1
