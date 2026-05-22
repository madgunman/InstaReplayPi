#!/usr/bin/env bash
# Verify workspace version (Pi-only; no Flutter pubspec).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/version.sh
source "$ROOT/scripts/lib/version.sh"

VERSION="$(get_workspace_version "$ROOT")"
CARGO="$ROOT/Cargo.toml"

if [[ ! -f "$CARGO" ]]; then
  echo "Cargo.toml not found" >&2
  exit 1
fi

if [[ "${CHECK_ONLY:-}" == "1" ]]; then
  if grep -q "^version = \"${VERSION}\"" "$CARGO"; then
    exit 0
  fi
  echo "Cargo workspace version mismatch (expected ${VERSION})" >&2
  exit 1
fi

echo "Workspace version: ${VERSION}"
