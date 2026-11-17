#!/usr/bin/env python3
"""Check the local UI build against its source inputs for install_dev.sh."""
from __future__ import annotations

import hashlib
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
UI = ROOT / "ui"
INPUTS = (
  "package.json", "package-lock.json", "webpack.config.js", "tsconfig.json",
  "scripts/build-tui.mjs", "scripts/write-manifest.mjs", "scripts/verify-bundles.mjs",
)


def fingerprint() -> str:
  digest = hashlib.sha256()
  digest.update(subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT))
  digest.update(subprocess.check_output(["node", "--version"]))
  digest.update(subprocess.check_output(["npm", "--version"]))
  digest.update(os.environ.get("ISOSUI_API_ORIGIN", "http://127.0.0.1:16000").encode())
  files = [UI / name for name in INPUTS]
  files += sorted(path for directory in (UI / "src", UI / "public") for path in directory.rglob("*") if path.is_file())
  for path in files:
    digest.update(str(path.relative_to(UI)).encode())
    digest.update(b"\0")
    digest.update(path.read_bytes())
    digest.update(b"\0")
  return digest.hexdigest()


def output_fingerprint(mode: str) -> str:
  digest = hashlib.sha256()
  targets = ["local", "tui"] if mode == "dev" else ["local", "hosted", "tui"]
  for target in targets:
    directory = UI / "dist" / target
    if not directory.is_dir():
      return ""
    for path in sorted(item for item in directory.rglob("*") if item.is_file()):
      digest.update(str(path.relative_to(UI / "dist")).encode())
      digest.update(b"\0")
      digest.update(path.read_bytes())
      digest.update(b"\0")
  return digest.hexdigest()


def main() -> int:
  if len(sys.argv) != 3 or sys.argv[1] not in {"dev", "release"} or sys.argv[2] not in {"check", "write"}:
    print("usage: ui_build_cache.py dev|release check|write", file=sys.stderr)
    return 2
  mode, action = sys.argv[1:]
  stamp = UI / f"dist/.{mode}-inputs.sha256"
  value = fingerprint()
  if action == "write":
    stamp.parent.mkdir(parents=True, exist_ok=True)
    stamp.write_text(value + "\n" + output_fingerprint(mode) + "\n")
    return 0
  outputs = [UI / "dist/local/index.html", UI / "dist/local/manifest.json", UI / "dist/tui/index.js"]
  if mode == "release":
    outputs += [UI / "dist/hosted/index.html", UI / "dist/hosted/manifest.json"]
  expected = value + "\n" + output_fingerprint(mode)
  return 0 if all(path.is_file() for path in outputs) and stamp.is_file() and stamp.read_text().strip() == expected else 1


if __name__ == "__main__":
  raise SystemExit(main())
