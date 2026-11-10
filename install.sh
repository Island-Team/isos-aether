#!/usr/bin/env bash
# Build and install one IsOS product release from this checkout.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SESSION="$(date -u +%Y%m%dT%H%M%SZ)-$$"
INSTALL_LOG_DIR="$ROOT/data/isos/log/install/$SESSION"
mkdir -p "$INSTALL_LOG_DIR"
chmod 700 "$INSTALL_LOG_DIR"
INSTALL_LOG="$INSTALL_LOG_DIR/install.jsonl"
log_install() { python3 "$ROOT/scripts/jsonl_log.py" "$INSTALL_LOG" "$1" isos.install@aether-v0.2.1 "$2"; }
log_install info install_starting
trap 'status=$?; log_install error "install_failed status=$status line=$LINENO"' ERR
PREFIX="${ISOS_INSTALL_ROOT:-}"
BUILD=1
BUNDLE=''
BUNDLE_OUTPUT=''
usage() { echo 'usage: ./install.sh [--prefix /absolute/path] [--no-build] [--bundle /absolute/archive.tar.gz] [--bundle-output /absolute/archive.tar.gz]'; }
while (( $# )); do
  case "$1" in
    --prefix) (( $# >= 2 )) || { usage >&2; exit 2; }; PREFIX="$2"; shift 2 ;;
    --no-build) BUILD=0; shift ;;
    --bundle) (( $# >= 2 )) || { usage >&2; exit 2; }; BUNDLE="$2"; shift 2 ;;
    --bundle-output) (( $# >= 2 )) || { usage >&2; exit 2; }; BUNDLE_OUTPUT="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
done
if [[ -z "$PREFIX" ]]; then
  SETTINGS="$ROOT/config/isos/install.toml"
  [[ -f "$SETTINGS" ]] || SETTINGS="$ROOT/config/install.toml"
  if [[ -f "$SETTINGS" ]]; then
    PREFIX="$(python3 - "$SETTINGS" <<'PY'
import os
import sys
import tomllib
with open(sys.argv[1], "rb") as source:
  value = tomllib.load(source)["install"]["prefix"]
print(os.path.expanduser(value))
PY
)"
  else
    PREFIX="${HOME}/.local/isos"
  fi
fi
[[ "$PREFIX" == /* ]] || { echo 'install prefix must be absolute' >&2; exit 2; }
[[ -z "$BUNDLE" || "$BUNDLE" == /* ]] || { echo 'bundle path must be absolute' >&2; exit 2; }
[[ -z "$BUNDLE_OUTPUT" || "$BUNDLE_OUTPUT" == /* ]] || { echo 'bundle output must be absolute' >&2; exit 2; }
if [[ -z "$BUNDLE" ]]; then
  [[ -f "$ROOT/Cargo.toml" && -f "$ROOT/ui/package.json" && -f "$ROOT/ei/pyproject.toml" ]] || {
    echo 'install.sh requires a complete IsOS checkout' >&2; exit 2;
  }
fi
for command_name in python3.12 sha256sum; do
  command -v "$command_name" >/dev/null || { echo "missing build tool: $command_name" >&2; exit 127; }
done
if [[ -z "$BUNDLE" ]] && (( BUILD )); then
  CARGO_BIN="$(command -v cargo || true)"
  if [[ -z "$CARGO_BIN" ]]; then
    CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/cargo"
  fi
  [[ -x "$CARGO_BIN" ]] || { echo "missing build tool: cargo (checked PATH and $CARGO_BIN)" >&2; exit 127; }
  command -v npm >/dev/null || { echo 'missing build tool: npm' >&2; exit 127; }
fi
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
[[ "$TARGET_DIR" == /* ]] || TARGET_DIR="$ROOT/$TARGET_DIR"
if [[ -z "$BUNDLE" ]] && (( BUILD )); then
  log_install info build_starting
  log_install info build_cargo
  "$CARGO_BIN" build --locked --release --workspace --manifest-path "$ROOT/Cargo.toml" 2>&1 | python3 "$ROOT/scripts/jsonl_log.py" "$INSTALL_LOG" info isos.install.build@aether-v0.2.1 --stdin
  log_install info build_ei
  ( cd "$ROOT/ei" && ./scripts/platform/dispatch.sh build ) 2>&1 | python3 "$ROOT/scripts/jsonl_log.py" "$INSTALL_LOG" info isos.install.build@aether-v0.2.1 --stdin
  log_install info build_ui
  ( cd "$ROOT/ui" && npm ci && npm run build ) 2>&1 | python3 "$ROOT/scripts/jsonl_log.py" "$INSTALL_LOG" info isos.install.build@aether-v0.2.1 --stdin
  log_install info build_completed
fi
mkdir -p "$PREFIX/releases" "$PREFIX/bin"
STAGE="$(mktemp -d "$PREFIX/releases/.stage.XXXXXX")"
cleanup() { [[ -z "${STAGE:-}" ]] || rm -rf -- "$STAGE"; }
trap cleanup EXIT
if [[ -n "$BUNDLE" ]]; then
  python3 - "$BUNDLE" "$STAGE" <<'PY'
import pathlib
import sys
import tarfile

archive, root = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
with tarfile.open(archive, "r:gz") as source:
  for item in source:
    path = pathlib.PurePosixPath(item.name)
    if path.is_absolute() or ".." in path.parts or not (item.isfile() or item.isdir()):
      raise SystemExit(f"unsafe bundle entry: {item.name}")
    target = root.joinpath(*path.parts)
    if item.isdir():
      target.mkdir(parents=True, exist_ok=True)
    else:
      target.parent.mkdir(parents=True, exist_ok=True)
      with source.extractfile(item) as content, target.open("wb") as output:
        output.write(content.read())
      target.chmod(item.mode & 0o755)
PY
  [[ -x "$STAGE/bin/isos" && -x "$STAGE/libexec/isos" && -f "$STAGE/MANIFEST.sha256" ]] || {
    echo 'bundle is missing required IsOS files' >&2; exit 1;
  }
  for program in aether-profile aether-runtime aether-http-bridge aether-config-server; do
    [[ -x "$STAGE/libexec/$program" ]] || { echo "bundle is missing EOS program: $program" >&2; exit 1; }
  done
  [[ -f "$STAGE/share/isos/ui/web/index.html" && -f "$STAGE/share/isos/ui/tui/index.js" && -f "$STAGE/share/isos/config/isos.toml" && -x "$STAGE/share/isos/ei/bin/ei" ]] || {
    echo 'bundle is missing required UI, config, or EI files' >&2; exit 1;
  }
  ( cd "$STAGE" && sha256sum -c MANIFEST.sha256 >/dev/null )
  VERSION="$(cat "$STAGE/VERSION")"
  python3 - "$STAGE/RELEASE.json" "$VERSION" <<'PY'
import json
import pathlib
import platform
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text())
if manifest.get("schema_version") != "1" or manifest.get("product_version") != sys.argv[2]:
  raise SystemExit("bundle release identity mismatch")
if manifest.get("os") != "linux" or manifest.get("architecture") != platform.machine():
  raise SystemExit("bundle platform does not match this host")
PY
else
  [[ -x "$TARGET_DIR/release/isos" && -f "$ROOT/ui/dist/local/index.html" ]] || {
    echo 'IsOS binary or Web UI build is missing' >&2; exit 1;
  }
  [[ -f "$ROOT/ui/dist/tui/index.js" ]] || { echo 'TUI build is missing' >&2; exit 1; }
  VERSION="$($TARGET_DIR/release/isos --version | awk '{print $2}')"
mkdir -p "$STAGE/bin" "$STAGE/libexec" "$STAGE/share/isos/ui" "$STAGE/share/isos/config" "$STAGE/share/isos/ei"
cp "$TARGET_DIR/release/isos" "$STAGE/libexec/isos"
for program in aether-profile aether-runtime aether-http-bridge aether-config-server; do
  [[ -x "$TARGET_DIR/release/$program" ]] || { echo "required EOS binary missing: $program" >&2; exit 1; }
  cp "$TARGET_DIR/release/$program" "$STAGE/libexec/$program"
done
cp -a "$ROOT/ui/dist/local" "$STAGE/share/isos/ui/web"
cp -a "$ROOT/ui/dist/tui" "$STAGE/share/isos/ui/tui"
cp -a "$ROOT/share/eos" "$STAGE/share/isos/eos"
cp -a "$ROOT/config/isos/." "$STAGE/share/isos/config/"
cp "$ROOT/config/isosui.toml" "$STAGE/share/isos/config/isosui.toml"
cp "$ROOT/LICENSE" "$STAGE/share/isos/LICENSE"
cp "$ROOT/install.sh" "$STAGE/share/isos/install.sh"
for part in bin libexec; do cp -a "$ROOT/ei/$part" "$STAGE/share/isos/ei/$part"; done
mkdir -p "$STAGE/share/isos/ei/etc/ei"
cp -a "$ROOT/config/ei/." "$STAGE/share/isos/ei/etc/ei/"
mv "$STAGE/share/isos/ei/etc/ei/island.toml" "$STAGE/share/isos/ei/etc/island.toml"
# The source checkout has one root share tree; releases retain the existing EI
# product overlay so older installed resource locations remain unchanged.
python3 - "$STAGE/share/isos/ei/etc/island.toml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
source = 'share = ["../share"]'
if text.count(source) != 1:
  raise SystemExit(f"unexpected EI source resource roots: {path}")
path.write_text(text.replace(source, 'share = ["share", ".."]'))
PY
# Source factory defaults may link to canonical config/ei files; release manifest
# verification requires regular files at both runtime paths.
python3 - "$ROOT/share/ei/defaults/ei" "$ROOT/config/ei" <<'PY'
from pathlib import Path
import sys

share, etc = (Path(value).resolve() for value in sys.argv[1:])
for item in share.rglob('*'):
  if item.is_symlink() and (not item.resolve().is_file() or not item.resolve().is_relative_to(etc)):
    raise SystemExit(f"EI factory default link escapes config/ei: {item}")
PY
mkdir -p "$STAGE/share/isos/ei/share/defaults"
cp -aL "$ROOT/share/ei/defaults/ei" "$STAGE/share/isos/ei/share/defaults/ei"
cp "$ROOT/share/isos/defaults/island.toml" "$STAGE/share/isos/ei/share/defaults/island.toml"
if [[ -f "$ROOT/ei/dist/ei-${VERSION}-py3-none-any.whl" ]]; then
  command -v uv >/dev/null || { echo 'uv is required to install EI dependencies' >&2; exit 127; }
  uv pip install --target "$STAGE/share/isos/ei/lib/island" --python "$(command -v python3.12)" "$ROOT/ei/dist/ei-${VERSION}-py3-none-any.whl"
else
  echo "EI wheel for ${VERSION} is missing" >&2; exit 1
fi
cat > "$STAGE/bin/isos" <<'LAUNCHER'
#!/usr/bin/env bash
set -euo pipefail
RELEASE_ROOT="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
WORKSPACE_CHECKOUT=0
if [[ -z "${ISOS_HOME:-}" && "${ISLAND_MODE:-}" != system ]]; then
  WORKSPACE_ROOT="$(pwd -P)"
  if [[ -f "$WORKSPACE_ROOT/Cargo.toml" && -f "$WORKSPACE_ROOT/config/isos/isos.toml" && -d "$WORKSPACE_ROOT/crates/isos" ]]; then
    WORKSPACE_CHECKOUT=1
    export ISOS_HOME="$WORKSPACE_ROOT"
    export ISOS_CONFIG="${ISOS_CONFIG:-$WORKSPACE_ROOT/config/isos/isos.toml}"
    export ISOSUI_CONFIG="${ISOSUI_CONFIG:-$WORKSPACE_ROOT/config/isosui.toml}"
  fi
fi
export ISOS_CONFIG="${ISOS_CONFIG:-$RELEASE_ROOT/share/isos/config/isos.toml}"
export ISOSUI_CONFIG="${ISOSUI_CONFIG:-$RELEASE_ROOT/share/isos/config/isosui.toml}"
export ISOS_MODE="${ISOS_MODE:-product}"
if [[ "$WORKSPACE_CHECKOUT" == 1 ]]; then
  export ISOSUI_TUI_ENTRY="${ISOSUI_TUI_ENTRY:-$WORKSPACE_ROOT/ui/dist/tui/index.js}"
else
  export ISOSUI_DIST_DIR="${ISOSUI_DIST_DIR:-$RELEASE_ROOT/share/isos/ui/web}"
  export ISOSUI_TUI_ENTRY="${ISOSUI_TUI_ENTRY:-$RELEASE_ROOT/share/isos/ui/tui/index.js}"
  export ISOS_AETHER_BIN_DIR="${ISOS_AETHER_BIN_DIR:-$RELEASE_ROOT/libexec}"
  export ISOS_EI_BIN_DIR="${ISOS_EI_BIN_DIR:-$RELEASE_ROOT/share/isos/ei/bin}"
  export ISOS_EI_DEFAULTS_DIR="${ISOS_EI_DEFAULTS_DIR:-$RELEASE_ROOT/share/isos/ei/etc/ei}"
  export ISOS_EI_FACTORY_DEFAULTS_DIR="${ISOS_EI_FACTORY_DEFAULTS_DIR:-$RELEASE_ROOT/share/isos/ei/share/defaults/ei}"
fi
exec "$RELEASE_ROOT/libexec/isos" "$@"
LAUNCHER
chmod 755 "$STAGE/bin/isos"
printf '%s\n' "$VERSION" > "$STAGE/VERSION"
python3 - "$STAGE/RELEASE.json" "$VERSION" "$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || true)" <<'PY'
import json
import pathlib
import platform
import sys

path, version, commit = sys.argv[1:]
pathlib.Path(path).write_text(json.dumps({
  "schema_version": "1",
  "product_version": version,
  "os": "linux",
  "architecture": platform.machine(),
  "source_commit": commit or None,
}, sort_keys=True) + "\n")
PY
( cd "$STAGE" && find bin libexec share VERSION RELEASE.json -type f -print0 | sort -z | xargs -0 sha256sum ) > "$STAGE/MANIFEST.sha256"
( cd "$STAGE" && sha256sum -c MANIFEST.sha256 >/dev/null )
fi
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'invalid IsOS version' >&2; exit 1; }
if [[ -n "$BUNDLE_OUTPUT" ]]; then
  ( cd "$STAGE" && tar -czf "$BUNDLE_OUTPUT" bin libexec share VERSION RELEASE.json MANIFEST.sha256 )
  ( cd "$(dirname "$BUNDLE_OUTPUT")" && sha256sum "$(basename "$BUNDLE_OUTPUT")" > "$(basename "$BUNDLE_OUTPUT").sha256" )
fi
RELEASE="${VERSION}-$(date -u +%Y%m%dT%H%M%SZ)-$$"
FINAL="$PREFIX/releases/$RELEASE"
mv -- "$STAGE" "$FINAL"
STAGE=''
ln -s "$FINAL" "$PREFIX/.current.new.$$"
mv -Tf "$PREFIX/.current.new.$$" "$PREFIX/current"
ln -sfn "$PREFIX/current/bin/isos" "$PREFIX/bin/isos"
"$PREFIX/bin/isos" --version
printf 'Installed IsOS %s at %s\n' "$VERSION" "$PREFIX/bin/isos"
log_install info "install_completed version=$VERSION prefix=$PREFIX"
