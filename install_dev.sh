#!/usr/bin/env bash
# Build this checkout for local development without installing a product release.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ "${1:-}" == --help && $# -eq 1 ]]; then
  echo 'usage: ./install_dev.sh [--no-source-refresh]'
  exit 0
fi
REFRESH_SOURCES=true
if [[ "${1:-}" == --no-source-refresh && $# -eq 1 ]]; then
  REFRESH_SOURCES=false
elif (( $# )); then
  echo 'usage: ./install_dev.sh [--no-source-refresh]' >&2
  exit 2
fi
CARGO_BIN="$(command -v cargo || true)"
if [[ -z "$CARGO_BIN" ]]; then
  CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/cargo"
fi
[[ -x "$CARGO_BIN" ]] || { echo 'install_dev: cargo is required' >&2; exit 127; }
command -v uv >/dev/null || { echo 'install_dev: uv is required' >&2; exit 127; }
command -v npm >/dev/null || { echo 'install_dev: npm is required' >&2; exit 127; }
if python3 "$ROOT/scripts/rust_build_cache.py" dev check; then
  printf 'Rust source unchanged; reusing local workspace build.\n'
else
  "$CARGO_BIN" build --locked --workspace --manifest-path "$ROOT/Cargo.toml"
  python3 "$ROOT/scripts/rust_build_cache.py" dev write
fi
( cd "$ROOT/ei" && uv sync --python "${EI_PYTHON_VERSION:-3.12}" --all-extras )
UI_LOCK_HASH="$(sha256sum "$ROOT/ui/package-lock.json" | cut -d' ' -f1)"
if [[ ! -d "$ROOT/ui/node_modules" || ! -f "$ROOT/ui/node_modules/.isos-lock-hash" || "$(cat "$ROOT/ui/node_modules/.isos-lock-hash")" != "$UI_LOCK_HASH" ]]; then
  ( cd "$ROOT/ui" && npm ci )
  printf '%s\n' "$UI_LOCK_HASH" > "$ROOT/ui/node_modules/.isos-lock-hash"
fi
if python3 "$ROOT/scripts/ui_build_cache.py" dev check; then
  printf 'UI source unchanged; reusing local Web and TUI builds.\n'
else
  ( cd "$ROOT/ui" && npm run build:local && npm run build:tui )
  python3 "$ROOT/scripts/ui_build_cache.py" dev write
fi
if [[ $REFRESH_SOURCES == true ]]; then
  ISOS_HOME="$ROOT" "$ROOT/target/debug/isos" pkg refresh-sources
fi
mkdir -p "$ROOT/config/isos"
MODE_FILE="$ROOT/config/isos/launch.toml"
TEMP_MODE="$(mktemp "$ROOT/config/isos/.launch.XXXXXXXX")"
trap 'rm -f -- "$TEMP_MODE"' EXIT
printf '# Local source build; ignored by Git.\n[launch]\nmode = "dev"\n' > "$TEMP_MODE"
mv -f -- "$TEMP_MODE" "$MODE_FILE"
trap - EXIT
printf 'Development build ready. Run ./isos web; restart an existing manager to use rebuilt binaries.\n'
