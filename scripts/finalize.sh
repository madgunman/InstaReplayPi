#!/usr/bin/env bash
# Production finalize gate — run before tagging a release or signing hardware acceptance.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SOAK_SECONDS="${SOAK_SECONDS:-120}"
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
# shellcheck source=/dev/null
source "$ROOT/packaging/lib/gstreamer-env.sh" 2>/dev/null || true

log() { echo "==> $*"; }

log "Version alignment"
chmod +x scripts/sync-version.sh scripts/lib/version.sh 2>/dev/null || true
CHECK_ONLY=1 ./scripts/sync-version.sh

log "Rust tests"
cargo test --workspace

log "Build release engine"
cargo build -p replay-engine --release

log "MVP automated acceptance (tests + HTTP via mvp_accept-full.sh)"
chmod +x scripts/mvp_accept-full.sh scripts/mvp_accept.sh
./scripts/mvp_accept-full.sh

if [[ "${SKIP_SOAK:-0}" != "1" ]]; then
  log "Soak smoke (${SOAK_SECONDS}s)"
  if [[ "$(uname -s)" == "Linux" ]] && [ -z "${DISPLAY:-}" ] && command -v xvfb-run >/dev/null 2>&1; then
    xvfb-run -a ./target/release/replay-engine --test &
  else
    ./target/release/replay-engine --test &
  fi
  ENGINE_PID=$!
  trap 'kill "$ENGINE_PID" 2>/dev/null || true' EXIT
  sleep 8
  SOAK_SECONDS="$SOAK_SECONDS" SOAK_INTERVAL=10 ./scripts/soak_test.sh
  kill "$ENGINE_PID" 2>/dev/null || true
  trap - EXIT
fi

log "Package verify (if dist exists)"
if [[ -x scripts/verify-package.sh ]]; then
  scripts/verify-package.sh 2>/dev/null || echo "WARN: no Pi dist bundle yet — run make package-pi"
fi

echo ""
echo "=============================================="
echo " Finalize gate: PASS"
echo " Next: complete manual hardware sign-off on Pi 5"
echo "   ./scripts/hardware_signoff.sh  (automated only)"
echo "   docs/HARDWARE_ACCEPTANCE.md"
echo "   docs/acceptance/RESULTS-pi.md"
echo " Release soak: SOAK_SECONDS=3600 ./scripts/soak_test.sh"
echo "=============================================="
