#!/usr/bin/env bash
# Sanity-check dist/ artifacts after packaging (run locally after make package-*).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/version.sh
source "$ROOT/scripts/lib/version.sh"
VERSION="$(get_workspace_version "$ROOT")"
FAIL=0

check() {
  if [[ -e "$1" ]]; then
    echo "  OK $1"
  else
    echo "  MISSING $1" >&2
    FAIL=1
  fi
}

echo "Verifying packages for version ${VERSION}..."

shopt -s nullglob
for zip in "$ROOT"/dist/InstantReplay-macos-*.zip; do
  echo "macOS: $zip"
  unzip -l "$zip" | grep -q 'OPERATOR.md' || { echo "  MISSING OPERATOR.md in zip" >&2; FAIL=1; }
  unzip -l "$zip" | grep -q 'replay-engine' || { echo "  MISSING engine in zip" >&2; FAIL=1; }
  unzip -l "$zip" | grep -q 'Start Instant Replay.command' || { echo "  MISSING launcher" >&2; FAIL=1; }
done

for tgz in "$ROOT"/dist/InstantReplay-*-linux-*.tar.gz; do
  echo "Linux: $tgz"
  tar -tzf "$tgz" | grep -q 'bin/replay-engine' || FAIL=1
  tar -tzf "$tgz" | grep -q 'OPERATOR.md\|share/doc' || FAIL=1
done

for tgz in "$ROOT"/dist/InstantReplay-*-pi5-aarch64.tar.gz; do
  echo "Pi: $tgz"
  tar -tzf "$tgz" | grep -q 'install-on-pi.sh' || FAIL=1
  tar -tzf "$tgz" | grep -q 'bin/replay-engine' || FAIL=1
done

for win in "$ROOT"/dist/InstantReplay-windows-*.zip; do
  echo "Windows: $win"
  unzip -l "$win" | grep -q 'replay-engine.exe' || FAIL=1
  unzip -l "$win" | grep -q 'OPERATOR.md' || FAIL=1
  unzip -l "$win" | grep -q 'Start Instant Replay.bat' || FAIL=1
done

if [[ "$FAIL" -eq 0 ]]; then
  echo "Package verification passed."
else
  echo "Package verification failed." >&2
  exit 1
fi
