#!/usr/bin/env bash
# Hardware sign-off helper — automated checks + reminder to complete RESULTS-pi.md.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Instant Replay — hardware sign-off helper"
echo "=========================================="
echo ""

if [ -x "$ROOT/scripts/doctor-pi.sh" ]; then
  "$ROOT/scripts/doctor-pi.sh" || true
else
  echo "WARN: doctor-pi.sh not found"
fi

echo ""
if curl -sfS --max-time 2 "http://127.0.0.1:8080/api/health" >/dev/null 2>&1; then
  echo "Engine HTTP API reachable — running mvp_accept.sh..."
  if [ -x "$ROOT/scripts/mvp_accept.sh" ]; then
    "$ROOT/scripts/mvp_accept.sh" || echo "WARN: mvp_accept.sh reported failures"
  fi
else
  echo "Engine not running on :8080 — skip HTTP acceptance."
  echo "  Start: sudo systemctl start replay-engine"
  echo "  Or CI-style: replay-engine --test --no-ui"
fi

echo ""
echo "Manual sign-off (required for venue release):"
echo "  1. Edit: docs/acceptance/RESULTS-pi.md"
echo "  2. Run on Pi: SOAK_SECONDS=3600 ./scripts/soak_test.sh"
echo "  3. Checklist: docs/HARDWARE_ACCEPTANCE.md"
echo ""
