#!/usr/bin/env bash
# Build the EI wheel from the source checkout on supported Linux architectures.
set -euo pipefail

if ! command -v uv >/dev/null 2>&1; then
  echo "ei-build: uv is required; install it from https://docs.astral.sh/uv/" >&2
  exit 127
fi

cd "$EI_ROOT"
PYTHON_REQUEST="${EI_PYTHON:-${EI_PYTHON_VERSION:-3.12}}"
uv sync --python "$PYTHON_REQUEST" --all-extras --no-install-project
uv sync --python "$PYTHON_REQUEST" --all-extras --no-build-isolation
exec uv build --no-build-isolation "$@"
