#!/usr/bin/env bash
# Long-run soak harness: requires replay-engine running (use --test for headless).
#
# SOAK_SECONDS:
#   - Default 3600 (1 hour) — release / hardware sign-off gate on Pi 5.
#   - CI smoke: SOAK_SECONDS=120 (see .github/workflows/acceptance.yml).
# SOAK_INTERVAL: seconds between mark/replay/return-live cycles (default 15).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="${HTTP_BASE:-http://127.0.0.1:8080}"
DURATION="${SOAK_SECONDS:-3600}"
INTERVAL="${SOAK_INTERVAL:-15}"
LOG="${SOAK_LOG:-/tmp/replay-soak.log}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl required" >&2
  exit 1
fi

echo "Soak test: ${DURATION}s, cycle every ${INTERVAL}s → ${BASE}" | tee "$LOG"
echo "Release gate: SOAK_SECONDS=3600 | CI smoke: SOAK_SECONDS=120" | tee -a "$LOG"
echo "Log: $LOG" | tee -a "$LOG"

curl_post() {
  local path="$1"
  local body="${2:-}"
  if [ -z "$body" ]; then
    body='{}'
  fi
  curl -sfS --max-time 30 -X POST -H "Content-Type: application/json" \
    -d "$body" "${BASE}${path}" >/dev/null 2>&1
}

curl_get() {
  curl -sfS --max-time 10 "${BASE}$1" 2>/dev/null
}

end=$((SECONDS + DURATION))
count=0
fail=0

while [ "$SECONDS" -lt "$end" ]; do
  count=$((count + 1))
  ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  if ! health="$(curl_get /api/health)"; then
    echo "[$ts] #$count FAIL health (engine down?)" | tee -a "$LOG"
    fail=$((fail + 1))
    sleep "$INTERVAL"
    continue
  fi

  mem_kb=""
  if command -v pgrep >/dev/null 2>&1; then
    pid="$(pgrep -x replay-engine 2>/dev/null | head -1 || true)"
    if [ -n "$pid" ] && ps -o rss= -p "$pid" >/dev/null 2>&1; then
      mem_kb="$(ps -o rss= -p "$pid" | tr -d ' ')"
    fi
  fi

  case $((count % 4)) in
    0) curl_post /api/mark '{}' || fail=$((fail + 1)) ;;
    1) curl_post /api/replay '{}' || fail=$((fail + 1)) ;;
    2) curl_post /api/replay-last '{"seconds":3}' || fail=$((fail + 1)) ;;
    3) curl_post /api/return-live '{}' || true ;;
  esac

  diag="$(curl_get /api/diagnostics || echo '{}')"
  echo "[$ts] #$count health ok mem_kb=${mem_kb:-?} diag=${diag}" | tee -a "$LOG"
  sleep "$INTERVAL"
done

echo "Soak complete: ${count} cycles, ${fail} failures" | tee -a "$LOG"
[ "$fail" -eq 0 ] || exit 1
