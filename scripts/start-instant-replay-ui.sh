#!/usr/bin/env bash
# Open touch UI; ensure engine is running (for desktop shortcut / manual start).
set -euo pipefail

if ! systemctl is-active --quiet replay-engine 2>/dev/null; then
  sudo systemctl start replay-engine
  sleep 2
fi

CHROMIUM=""
for c in chromium chromium-browser; do
  if command -v "$c" >/dev/null 2>&1; then
    CHROMIUM="$c"
    break
  fi
done

if [ -z "$CHROMIUM" ]; then
  echo "Install chromium: sudo apt install chromium" >&2
  exit 1
fi

exec "$CHROMIUM" --app=http://127.0.0.1:8080/ --start-fullscreen
