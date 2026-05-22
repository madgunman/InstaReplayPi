#!/usr/bin/env bash
# Start replay-engine (--test), run mvp_accept.sh over HTTP, then stop.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export HTTP_BASE="${HTTP_BASE:-http://127.0.0.1:8080}"
# shellcheck source=/dev/null
source "$ROOT/packaging/lib/gstreamer-env.sh" 2>/dev/null || true

if [[ -x "${ROOT}/target/release/replay-engine" ]]; then
  ENGINE="${ROOT}/target/release/replay-engine"
elif [[ -x "${ROOT}/target/debug/replay-engine" ]]; then
  ENGINE="${ROOT}/target/debug/replay-engine"
else
  echo "Building replay-engine..."
  (cd "$ROOT" && cargo build -p replay-engine --release)
  ENGINE="${ROOT}/target/release/replay-engine"
fi

lsof -ti:8080 | xargs kill -9 2>/dev/null || true
sleep 1

USE_XVFB=false
if [[ "$(uname -s)" == "Linux" ]] && [ -z "${DISPLAY:-}" ]; then
  if command -v xvfb-run >/dev/null 2>&1; then
    USE_XVFB=true
  fi
fi

log() { echo "$*"; }

if $USE_XVFB; then
  log "Starting engine under xvfb-run (--test)"
  xvfb-run -a "$ENGINE" --test &
else
  log "Starting engine (--test)"
  "$ENGINE" --test &
fi
ENGINE_PID=$!
trap 'kill "$ENGINE_PID" 2>/dev/null || true' EXIT

health_check() {
  curl -sfS --max-time 2 "${HTTP_BASE}/api/health" >/dev/null 2>&1
}

for _ in $(seq 1 45); do
  if health_check; then
    break
  fi
  sleep 1
done

if ! health_check; then
  echo "Engine failed to start HTTP on ${HTTP_BASE}" >&2
  exit 1
fi

log "Engine up (pid $ENGINE_PID), running acceptance..."
"$ROOT/scripts/mvp_accept.sh"
exit $?
