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

PROC_COUNT="$(pgrep -c -x replay-engine 2>/dev/null || echo 0)"
if [ "$PROC_COUNT" -eq 1 ]; then
  ok "Exactly one replay-engine process"
elif [ "$PROC_COUNT" -eq 0 ]; then
  warn "No replay-engine process (service may be stopped)"
else
  fail "Multiple replay-engine processes ($PROC_COUNT) — stop extras: sudo systemctl stop replay-engine; sudo pkill -x replay-engine"
  pgrep -a -x replay-engine 2>/dev/null || true
fi

if [ -f /run/instant-replay/replay-engine.lock ]; then
  ok "Engine lock file present"
else
  warn "No engine lock file (expected when service is running)"
fi

if [ "$(uname -m)" != "aarch64" ]; then
  warn "Not aarch64 ($(uname -m)) — intended for Raspberry Pi 5"
fi

if command -v gst-inspect-1.0 >/dev/null 2>&1; then
  for plug in v4l2src x264enc h264parse splitmuxsink glimagesink uridecodebin; do
    if gst-inspect-1.0 "$plug" >/dev/null 2>&1; then
      ok "GStreamer plugin $plug"
    else
      fail "Missing GStreamer plugin: $plug"
    fi
  done
  if gst-inspect-1.0 vah264enc >/dev/null 2>&1; then
    ok "Hardware buffer encoder vah264enc (preferred on Pi)"
  elif [ "$(uname -m)" = "aarch64" ]; then
    warn "vah264enc not found — buffer will use x264enc (higher CPU at 1080p60)"
  fi
else
  fail "gst-inspect-1.0 not found"
fi

echo ""
echo "USB / V4L2 capture devices:"
if command -v v4l2-ctl >/dev/null 2>&1; then
  v4l2-ctl --list-devices 2>/dev/null | grep -v "^$" | head -30 || warn "v4l2-ctl list failed"
else
  warn "v4l2-ctl not installed"
fi

if grep -q 'device_id = "auto"' /etc/instant-replay/config.toml 2>/dev/null \
  || ! grep -q '^\[input\]' /etc/instant-replay/config.toml 2>/dev/null; then
  ok "input.device_id auto-detect enabled"
else
  warn "input.device_id is manual — consider device_id = \"auto\" for plug-and-play"
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
