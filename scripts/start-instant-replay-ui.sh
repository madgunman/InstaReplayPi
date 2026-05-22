#!/usr/bin/env bash
# Legacy helper — operator UI is built into replay-engine since v0.2.
set -euo pipefail
echo "Native operator UI runs inside replay-engine (no separate browser step)."
echo "  sudo systemctl start replay-engine"
echo "  journalctl -u replay-engine -f"
