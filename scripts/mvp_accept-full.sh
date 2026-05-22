#!/usr/bin/env bash
# MVP acceptance: unit/integration tests + headless engine + HTTP API smoke (mvp_accept.sh).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT/packaging/lib/gstreamer-env.sh" 2>/dev/null || true

log() { echo "==> $*"; }

log "Rust tests (replay-core + replay-engine)"
(cd "$ROOT" && cargo test -p replay-core -p replay-engine)

if [[ -x "${ROOT}/target/release/replay-engine" ]]; then
  ENGINE="${ROOT}/target/release/replay-engine"
elif [[ -x "${ROOT}/target/debug/replay-engine" ]]; then
  ENGINE="${ROOT}/target/debug/replay-engine"
else
  log "Building replay-engine (release)"
  (cd "$ROOT" && cargo build -p replay-engine --release)
  ENGINE="${ROOT}/target/release/replay-engine"
fi

wait_http() {
  local i=0
  while [ "$i" -lt 30 ]; do
    if curl -sfS --max-time 2 "http://127.0.0.1:8080/api/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
    i=$((i + 1))
  done
  echo "HTTP API did not become ready on :8080" >&2
  return 1
}

log "Headless engine with loopback HTTP (--test --no-ui)"
if [[ "$(uname -s)" == "Linux" ]] && command -v xvfb-run >/dev/null 2>&1; then
  xvfb-run -a "$ENGINE" --test --no-ui &
else
  "$ENGINE" --test --no-ui &
fi
ENGINE_PID=$!
trap 'kill "$ENGINE_PID" 2>/dev/null || true; wait "$ENGINE_PID" 2>/dev/null || true' EXIT

if ! wait_http; then
  exit 1
fi

log "HTTP MVP acceptance (mvp_accept.sh)"
chmod +x "$ROOT/scripts/mvp_accept.sh"
"$ROOT/scripts/mvp_accept.sh"

kill "$ENGINE_PID" 2>/dev/null || true
wait "$ENGINE_PID" 2>/dev/null || true
trap - EXIT

log "MVP accept-full: PASS"
