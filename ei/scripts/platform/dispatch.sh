#!/usr/bin/env bash
# Verify the supported host CPU architecture before the shared EI build recipe.
set -euo pipefail

SCRIPT="$(readlink -f "${BASH_SOURCE[0]}")"
ROOT="$(cd "$(dirname "$SCRIPT")/../.." && pwd)"
ACTION="${1:-}"

case "$ACTION" in
  build)
    shift
    ;;
  *)
    echo "ei-platform: expected action 'build'" >&2
    exit 2
    ;;
esac

case "$(uname -m)" in
  aarch64|arm64|x86_64|amd64)
    ;;
  *)
    echo "ei-platform: unsupported architecture '$(uname -m)'; expected arm64 or x86_64" >&2
    exit 2
    ;;
esac

export EI_ROOT="$ROOT"
exec "$ROOT/scripts/build.sh" "$@"
