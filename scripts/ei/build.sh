#!/usr/bin/env bash
# Build the EI wheel from the source checkout on supported Linux architectures.
set -euo pipefail

if ! command -v uv >/dev/null 2>&1; then
  echo "ei-build: uv is required; install it from https://docs.astral.sh/uv/" >&2
  exit 127
fi

cd "$EI_ROOT"
if [[ -f "$EI_ROOT/../contract/node/isos_node_v1.h" ]]; then
  cmp "$EI_ROOT/native/isos_node_v1.h" "$EI_ROOT/../contract/node/isos_node_v1.h"
fi
cc -std=c11 -O2 -fPIC -shared -I "$EI_ROOT/native" \
  "$EI_ROOT/native/demo_stage.c" -o "$EI_ROOT/native/libei_demo_stage.so"
PYTHON_REQUEST="${EI_PYTHON:-${EI_PYTHON_VERSION:-3.12}}"
uv sync --python "$PYTHON_REQUEST" --all-extras --no-install-project
uv sync --python "$PYTHON_REQUEST" --all-extras --no-build-isolation
exec uv build --no-build-isolation "$@"
