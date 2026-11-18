#!/usr/bin/env python3
"""Check whether a local EI wheel still matches its build inputs."""
from __future__ import annotations

import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
EI = ROOT / "ei"


def wheel() -> Path | None:
  version = tomllib.loads((EI / "pyproject.toml").read_text())["project"]["version"]
  matches = list((EI / "dist").glob(f"ei-{version}-py3-none-linux_*.whl"))
  return matches[0] if len(matches) == 1 else None


def fingerprint() -> str:
  digest = hashlib.sha256()
  python_request = os.environ.get("EI_PYTHON", os.environ.get("EI_PYTHON_VERSION", "3.12"))
  digest.update(python_request.encode())
  digest.update(subprocess.check_output(["uv", "--version"]))
  digest.update(subprocess.check_output(["uv", "python", "find", python_request], stderr=subprocess.STDOUT))
  paths = [path for directory in (EI, ROOT / "scripts/ei", ROOT / "contract/node") for path in directory.rglob("*")
           if path.is_file() and not any(part in {".venv", "dist", "__pycache__", ".pytest_cache", ".ruff_cache", ".mypy_cache", ".import_linter_cache", "ei.egg-info", "build"} for part in path.parts)
           and path.suffix not in {".pyc", ".so"}]
  for path in sorted(paths):
    digest.update(str(path.relative_to(ROOT)).encode() + b"\0")
    digest.update(path.read_bytes() + b"\0")
  return digest.hexdigest()


def main() -> int:
  if len(sys.argv) != 2 or sys.argv[1] not in {"check", "write"}:
    print("usage: ei_build_cache.py check|write", file=sys.stderr)
    return 2
  output = wheel()
  stamp = EI / "dist/.isos-ei-inputs.sha256"
  if output is None:
    return 1
  value = fingerprint() + "\n" + hashlib.sha256(output.read_bytes()).hexdigest()
  if sys.argv[1] == "check":
    return 0 if stamp.is_file() and stamp.read_text().strip() == value else 1
  stamp.write_text(value + "\n")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
