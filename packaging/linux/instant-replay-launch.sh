#!/usr/bin/env bash
# Pi launcher — GStreamer env + replay-engine
set -euo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=/dev/null
source "$DIR/scripts/gstreamer-env.sh" 2>/dev/null || source "$DIR/packaging/lib/gstreamer-env.sh"
exec "$DIR/bin/replay-engine" "$@"
