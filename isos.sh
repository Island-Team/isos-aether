#!/usr/bin/env bash
# Run local builds in dev mode or the installed product in product mode.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SESSION="$(date -u +%Y%m%dT%H%M%SZ)-$$"
MODE="${ISOS_MODE:-$(python3 - "$ROOT/config/isos/launch.toml" <<'PY'
import pathlib
import sys
import tomllib

path = pathlib.Path(sys.argv[1])
print(tomllib.loads(path.read_text()).get('launch', {}).get('mode', 'product') if path.exists() else 'product')
PY
)}"
if [[ "$MODE" == dev ]]; then
  LOG_ROOT="${ISOS_HOME:-$ROOT}/data/isos/log"
  [[ "$LOG_ROOT" == /* ]] || { echo 'ISOS_HOME must be absolute' >&2; exit 2; }
  LAUNCH_LOG="$LOG_ROOT/launch/dev/launcher.jsonl"
else
  LOG_ROOT="$ROOT/data/isos/log"
  LAUNCH_LOG="$LOG_ROOT/launch/$SESSION/launcher.jsonl"
fi
export ISOS_LAUNCH_LOG_ROOT="$LOG_ROOT"
mkdir -p "$(dirname "$LAUNCH_LOG")"
chmod 700 "$(dirname "$LAUNCH_LOG")"
python3 "$ROOT/scripts/jsonl_log.py" "$LAUNCH_LOG" info isos.launcher@aether-v0.2.1 "launcher_starting command=${1:-default}"
trap 'status=$?; python3 "$ROOT/scripts/jsonl_log.py" "$LAUNCH_LOG" error isos.launcher@aether-v0.2.1 "launcher_failed status=$status"' ERR
case "$MODE" in
  dev)
    BIN="${CARGO_TARGET_DIR:-$ROOT/target}"
    [[ "$BIN" == /* ]] || BIN="$ROOT/$BIN"
    BIN="$BIN/debug/isos"
    [[ -x "$BIN" ]] || { echo "Local IsOS build missing at $BIN; run cargo build --workspace" >&2; exit 127; }
    export ISOS_MODE=dev ISOS_HOME="${ISOS_HOME:-$ROOT}"
    export ISOS_CONFIG="${ISOS_CONFIG:-$ROOT/config/isos/isos.toml}"
    export ISOSUI_CONFIG="${ISOSUI_CONFIG:-$ROOT/config/isosui.toml}"
    export ISOS_AETHER_BIN_DIR="${ISOS_AETHER_BIN_DIR:-$(dirname "$BIN")}"
    export ISOS_EI_BIN_DIR="${ISOS_EI_BIN_DIR:-$ROOT/ei/bin}"
    export ISOSUI_DIST_DIR="${ISOSUI_DIST_DIR:-$ROOT/ui/dist/local}"
    export ISOSUI_TUI_ENTRY="${ISOSUI_TUI_ENTRY:-$ROOT/ui/dist/tui/index.js}"
    python3 "$ROOT/scripts/jsonl_log.py" "$LAUNCH_LOG" info isos.launcher@aether-v0.2.1 "launcher_exec mode=dev binary=$BIN"
    exec "$BIN" "$@" 2> >(while IFS= read -r line; do
      python3 "$ROOT/scripts/jsonl_log.py" "$LAUNCH_LOG" error isos.launcher@aether-v0.2.1 "child_stderr $line"
      printf '%s\n' "$line" >&2
    done)
    ;;
  product) export ISOS_MODE=product ;;
  *) echo 'IsOS mode must be dev or product' >&2; exit 2 ;;
esac
PREFIX="${ISOS_INSTALL_ROOT:-}"
if [[ -z "$PREFIX" ]]; then
  PREFIX="$(python3 - "$ROOT/config/isos/install.toml" <<'PY'
from pathlib import Path
import os
import sys
import tomllib

with Path(sys.argv[1]).open('rb') as source:
  prefix = tomllib.load(source)['install']['prefix']
print(os.path.expanduser(prefix))
PY
)"
fi
if [[ "$PREFIX" != /* ]]; then
  echo 'IsOS install prefix must be absolute' >&2
  exit 2
fi
LAUNCHER="$PREFIX/bin/isos"
if [[ ! -x "$LAUNCHER" ]]; then
  echo "IsOS is not installed at $LAUNCHER; run ./install.sh first" >&2
  exit 127
fi
if [[ "${ISLAND_MODE:-}" != system && -z "${ISOS_HOME:-}" ]]; then
  export ISOS_HOME="$ROOT"
  export ISOS_CONFIG="${ISOS_CONFIG:-$ROOT/config/isos/isos.toml}"
  export ISOSUI_CONFIG="${ISOSUI_CONFIG:-$ROOT/config/isosui.toml}"
fi
python3 "$ROOT/scripts/jsonl_log.py" "$LAUNCH_LOG" info isos.launcher@aether-v0.2.1 "launcher_exec binary=$LAUNCHER"
exec "$LAUNCHER" "$@" 2> >(while IFS= read -r line; do
  python3 "$ROOT/scripts/jsonl_log.py" "$LAUNCH_LOG" error isos.launcher@aether-v0.2.1 "child_stderr $line"
  printf '%s\n' "$line" >&2
done)
