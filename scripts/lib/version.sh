#!/usr/bin/env bash
# Shared version helpers — source from other scripts (do not execute directly).
get_workspace_version() {
  local root="${1:-}"
  if [[ -z "$root" ]]; then
    root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  fi
  grep '^version' "$root/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)"/\1/'
}
