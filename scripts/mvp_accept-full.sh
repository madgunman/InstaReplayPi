#!/usr/bin/env bash
# MVP acceptance: unit/integration tests + short headless engine smoke (--test --no-ui).
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

log "Headless engine smoke (--test --no-ui, 8s)"
"$ENGINE" --test --no-ui &
ENGINE_PID=$!
trap 'kill "$ENGINE_PID" 2>/dev/null || true' EXIT
sleep 8
if ! kill -0 "$ENGINE_PID" 2>/dev/null; then
  echo "Engine exited early during smoke test" >&2
  exit 1
fi
log "Engine smoke: OK (pid $ENGINE_PID)"
kill "$ENGINE_PID" 2>/dev/null || true
wait "$ENGINE_PID" 2>/dev/null || true

log "MVP accept-full: PASS"
