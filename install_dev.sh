#!/usr/bin/env bash
# Build this checkout for local development without installing a product release.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ "${1:-}" == --help && $# -eq 1 ]]; then
  echo 'usage: ./install_dev.sh'
  exit 0
fi
if (( $# )); then
  echo 'usage: ./install_dev.sh' >&2
  exit 2
fi
CARGO_BIN="$(command -v cargo || true)"
if [[ -z "$CARGO_BIN" ]]; then
  CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/cargo"
fi
[[ -x "$CARGO_BIN" ]] || { echo 'install_dev: cargo is required' >&2; exit 127; }
command -v uv >/dev/null || { echo 'install_dev: uv is required' >&2; exit 127; }
command -v npm >/dev/null || { echo 'install_dev: npm is required' >&2; exit 127; }
"$CARGO_BIN" build --locked --workspace --manifest-path "$ROOT/Cargo.toml"
( cd "$ROOT/ei" && uv sync --python "${EI_PYTHON_VERSION:-3.12}" --all-extras )
if [[ ! -d "$ROOT/ui/node_modules" ]]; then
  ( cd "$ROOT/ui" && npm ci )
fi
( cd "$ROOT/ui" && npm run build )
mkdir -p "$ROOT/config/isos"
MODE_FILE="$ROOT/config/isos/launch.toml"
TEMP_MODE="$(mktemp "$ROOT/config/isos/.launch.XXXXXXXX")"
trap 'rm -f -- "$TEMP_MODE"' EXIT
printf '# Local source build; ignored by Git.\n[launch]\nmode = "dev"\n' > "$TEMP_MODE"
mv -f -- "$TEMP_MODE" "$MODE_FILE"
trap - EXIT
printf 'Development build ready. Run ./isos web; restart an existing manager to use rebuilt binaries.\n'
