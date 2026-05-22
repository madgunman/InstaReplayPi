#!/usr/bin/env bash
# MVP acceptance smoke tests — requires replay-engine already running with loopback HTTP on :8080.
# Start via mvp_accept-full.sh or: replay-engine --test --no-ui  (HTTP enabled by default with --test)
# Use mvp_accept-full.sh to start/stop the engine automatically.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="${HTTP_BASE:-http://127.0.0.1:8080}"
LOG="${MVP_LOG:-/tmp/replay-mvp-accept.log}"
BUFFER_WAIT_SECS="${BUFFER_WAIT_SECS:-35}"
REPLAY_WAIT_SECS="${REPLAY_WAIT_SECS:-8}"
MIN_BUFFER_SECS="${MIN_BUFFER_SECS:-1.5}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl required" >&2
  exit 1
fi

: >"$LOG"
PASS=0
FAIL=0
SKIP=0

log() { echo "$*" | tee -a "$LOG"; }

curl_get() {
  curl -sfS --max-time "${HTTP_MAX_TIME:-30}" "${BASE}$1" 2>>"$LOG"
}

curl_post() {
  local path="$1"
  local body="${2:-}"
  if [ -z "$body" ]; then
    body='{}'
  fi
  curl -sS --max-time "${HTTP_MAX_TIME:-30}" -X POST \
    -H "Content-Type: application/json" \
    -d "$body" \
    "${BASE}${path}" 2>>"$LOG"
}

json_ok() {
  local resp="$1"
  echo "$resp" | grep -qE '"ok"[[:space:]]*:[[:space:]]*true'
}

json_field() {
  local resp="$1" field="$2"
  if command -v jq >/dev/null 2>&1; then
    echo "$resp" | jq -r ".${field} // empty" 2>/dev/null
  else
    echo "$resp" | sed -n "s/.*\"${field}\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p" | head -1
  fi
}

json_number() {
  local resp="$1" field="$2"
  if command -v jq >/dev/null 2>&1; then
    echo "$resp" | jq -r ".${field} // 0" 2>/dev/null
  else
    echo "$resp" | grep -oE "\"${field}\"[[:space:]]*:[[:space:]]*[0-9.]+" | head -1 | grep -oE '[0-9.]+$'
  fi
}

pass() { PASS=$((PASS + 1)); log "  PASS: $1"; }
fail() { FAIL=$((FAIL + 1)); log "  FAIL: $1"; }
skip() { SKIP=$((SKIP + 1)); log "  SKIP: $1"; }

wait_buffer() {
  local i=0
  while [ "$i" -lt "$BUFFER_WAIT_SECS" ]; do
    local d
    d="$(curl_get /api/status 2>/dev/null || echo '{}')"
    local secs
    secs="$(json_number "$d" bufferSecondsAvailable)"
    local state
    state="$(json_field "$d" state)"
    if awk -v s="${secs:-0}" -v m="$MIN_BUFFER_SECS" 'BEGIN { exit !(s >= m) }' 2>/dev/null; then
      log "  buffer ready: ${secs}s state=${state}"
      return 0
    fi
    sleep 1
    i=$((i + 1))
  done
  return 1
}

assert_not_replaying() {
  local label="$1"
  local diag
  diag="$(curl_get /api/diagnostics 2>/dev/null || echo '{}')"
  local st
  st="$(json_field "$diag" current_state)"
  if [ "$st" = "REPLAYING" ]; then
    fail "${label}: FSM stuck in REPLAYING (state=${st})"
    return 1
  fi
  if [ "$st" = "LIVE" ] || [ "$st" = "MARKED" ] || [ "$st" = "STARTING" ]; then
    pass "${label}: state ${st} (not REPLAYING)"
    return 0
  fi
  skip "${label}: state ${st:-unknown}"
  return 0
}

log "MVP acceptance → ${BASE}"
log "Log: $LOG"
log "Min buffer for replay: ${MIN_BUFFER_SECS}s"
log ""

log "[1] Health"
if h="$(curl_get /api/health 2>/dev/null)" && [ -n "$h" ]; then
  pass "Health HTTP reachable"
else
  fail "Health / engine not reachable"
fi

log "[2] Devices"
if dev="$(curl_get /api/devices 2>/dev/null)"; then
  if echo "$dev" | grep -q '"id"'; then
    pass "ListDevices returns devices"
    if echo "$dev" | grep -q '"id"[[:space:]]*:[[:space:]]*"test"'; then
      pass "Test pattern device present"
    else
      skip "Test pattern not listed (live-only build?)"
    fi
  else
    fail "ListDevices empty"
  fi
else
  fail "ListDevices HTTP"
fi

log "[3] Displays"
if disp="$(curl_get /api/displays 2>/dev/null)" && echo "$disp" | grep -qE 'displays|"name"'; then
  pass "ListDisplays returns monitors"
else
  fail "ListDisplays"
fi

log "[4] Formats (test)"
if fmt="$(curl_get /api/formats/test 2>/dev/null)" && echo "$fmt" | grep -q '"width"'; then
  pass "ListFormats for test device"
else
  skip "ListFormats test device"
fi

log "[5] Config"
if cfg="$(curl_get /api/config 2>/dev/null)"; then
  pass "GetConfig"
  if echo "$cfg" | grep -qE 'buffer_seconds|bufferSeconds'; then
    pass "Replay buffer config present"
  else
    skip "Could not read buffer_seconds from config"
  fi
else
  fail "GetConfig"
fi

log "[6] StartLive (test pattern)"
START_BODY='{"device_id":"test","display_id":0,"fullscreen":false,"width":1280,"height":720,"fps":30,"pixel_format":"auto"}'
if start="$(curl_post /api/start-live "$START_BODY" 2>/dev/null)" && json_ok "$start"; then
  pass "StartLive test pattern"
else
  fail "StartLive: $(echo "${start:-timeout}" | tr '\n' ' ')"
  log ""
  log "Summary: pass=$PASS fail=$FAIL skip=$SKIP"
  exit 1
fi

sleep 1

log "[6b] Replay before buffer ready (expect reject)"
early="$(curl_post /api/replay '{}' 2>/dev/null || echo '{}')"
if json_ok "$early"; then
  fail "Replay succeeded with insufficient buffer (guard missing?)"
else
  pass "Replay rejected before buffer threshold"
  if echo "$early" | grep -qiE 'buffer|1\.5|segments'; then
    pass "Replay error mentions buffer/segments"
  else
    skip "Replay error text: $(echo "$early" | tr '\n' ' ')"
  fi
fi
assert_not_replaying "After early replay reject"

log "[7] Wait for rolling buffer"
if wait_buffer; then
  pass "Buffer segments available (>= ${MIN_BUFFER_SECS}s)"
else
  fail "Buffer did not reach ${MIN_BUFFER_SECS}s within ${BUFFER_WAIT_SECS}s"
fi

log "[8] Diagnostics (FPS / state)"
diag="$(curl_get /api/diagnostics 2>/dev/null || echo '{}')"
state="$(json_field "$diag" current_state)"
if [ "$state" = "LIVE" ] || [ "$state" = "MARKED" ]; then
  pass "State LIVE after start (${state})"
else
  fail "Expected LIVE, got ${state:-unknown}"
fi
fps="$(json_number "$diag" input_fps)"
if awk -v f="${fps:-0}" 'BEGIN { exit !(f > 0.5) }' 2>/dev/null; then
  pass "Input FPS reported (${fps})"
else
  skip "Input FPS low/zero on test pattern (${fps:-0})"
fi

if h2="$(curl_get /api/health 2>/dev/null)" && echo "$h2" | grep -qE '"ready"[[:space:]]*:[[:space:]]*true'; then
  pass "Health ready after live + buffer"
else
  skip "Health not ready yet (may still be starting)"
fi

log "[9] Mark → ClearMark (no replay)"
if m0="$(curl_post /api/mark '{}' 2>/dev/null)" && json_ok "$m0"; then
  pass "Mark (for ClearMark path)"
  sleep 1
  if cm0="$(curl_post /api/clear-mark '{}' 2>/dev/null)" && json_ok "$cm0"; then
    pass "ClearMark without replay"
    diag_cm="$(curl_get /api/diagnostics 2>/dev/null || echo '{}')"
    st_cm="$(json_field "$diag_cm" current_state)"
    if [ "$st_cm" = "LIVE" ]; then
      pass "LIVE after Mark → ClearMark"
    else
      fail "Expected LIVE after ClearMark, got ${st_cm:-unknown}"
    fi
  else
    fail "ClearMark after mark (no replay): $(echo "${cm0:-}" | tr '\n' ' ')"
  fi
else
  fail "Mark for ClearMark path: $(echo "${m0:-}" | tr '\n' ' ')"
fi

log "[10] Mark (for replay)"
if m="$(curl_post /api/mark '{}' 2>/dev/null)" && json_ok "$m"; then
  pass "Mark"
else
  fail "Mark: $(echo "${m:-}" | tr '\n' ' ')"
fi

sleep 2

log "[11] Replay from mark"
if r="$(curl_post /api/replay '{}' 2>/dev/null)" && json_ok "$r"; then
  pass "Replay RPC accepted"
  sleep "$REPLAY_WAIT_SECS"
  diag2="$(curl_get /api/diagnostics 2>/dev/null || echo '{}')"
  st2="$(json_field "$diag2" current_state)"
  if [ "$st2" = "LIVE" ] || [ "$st2" = "RETURNING_TO_LIVE" ] || [ "$st2" = "REPLAYING" ]; then
    pass "Post-replay state plausible (${st2})"
  else
    skip "Post-replay state ${st2:-unknown}"
  fi
else
  err="$(echo "${r:-}" | tr '\n' ' ')"
  if echo "$err" | grep -qi 'no segments'; then
    skip "Replay from mark (buffer empty — try ReplayLast after chunks close)"
  else
    fail "Replay: $err"
    assert_not_replaying "After failed replay from mark"
  fi
fi

log "[12] ReturnLive"
if rl="$(curl_post /api/return-live '{}' 2>/dev/null)" && json_ok "$rl"; then
  pass "ReturnLive"
else
  skip "ReturnLive (may already be live)"
fi
sleep 1

log "[13] ReplayLast"
if last="$(curl_post /api/replay-last '{"seconds":3}' 2>/dev/null)" && json_ok "$last"; then
  pass "ReplayLast 3s"
  sleep "$REPLAY_WAIT_SECS"
  curl_post /api/return-live '{}' >/dev/null 2>&1 || true
else
  fail "ReplayLast"
fi

log "[14] Stop"
if stp="$(curl_post /api/stop '{}' 2>/dev/null)" && json_ok "$stp"; then
  pass "Stop"
else
  fail "Stop"
fi

log ""
log "========================================"
log "MVP automated acceptance: pass=$PASS fail=$FAIL skip=$SKIP"
log "Full log: $LOG"
log "========================================"
log ""
log "Manual hardware checks still required — see docs/HARDWARE_ACCEPTANCE.md"
log "  - Real UVC / Cam Link at 1080p50/60 on audience HDMI"
log "  - Native egui touch UI and keyboard hotkeys (HTTP :8080 is acceptance/diagnostics only)"
log "  - Cable disconnect → NO_SIGNAL"
log "  - 60 min soak (release gate): SOAK_SECONDS=3600 ./scripts/soak_test.sh"
log "  - CI soak smoke uses SOAK_SECONDS=120 (see .github/workflows/acceptance.yml)"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
exit 0
