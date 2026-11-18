#!/usr/bin/env python3
"""Reuse a completed local Rust workspace build only for identical inputs."""
from __future__ import annotations

import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PROGRAMS = ("isos", "aether-profile", "aether-runtime", "aether-server", "libaether_runtime.so")


def target_dir() -> Path:
  path = Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target")))
  return path if path.is_absolute() else ROOT / path


def fingerprint(mode: str) -> str:
  digest = hashlib.sha256()
  for key in ("CARGO_TARGET_DIR", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "CARGO_BUILD_TARGET", "CARGO_PROFILE_RELEASE_LTO"):
    digest.update(f"{key}={os.environ.get(key, '')}\0".encode())
  cargo = shutil.which("cargo") or str(Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo"))) / "bin/cargo")
  rustc = shutil.which("rustc") or str(Path(cargo).with_name("rustc"))
  for command in ([cargo, "--version"], [rustc, "--version"]):
    digest.update(subprocess.check_output(command))
  digest.update(mode.encode())
  paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "config/isos/install.toml",
           ROOT / "config/isos/isos.example.toml", ROOT / "config/isosui.toml",
           ROOT / "config/version-support.toml"]
  paths += [path for directory in (ROOT / "crates", ROOT / "contract", ROOT / "templates", ROOT / ".cargo") if directory.exists()
            for path in directory.rglob("*") if path.is_file() and not any(part in {"target", "__pycache__", ".git"} for part in path.parts)]
  paths += [path for path in ROOT.glob("rust-toolchain*") if path.is_file()]
  for path in sorted(paths):
    digest.update(str(path.relative_to(ROOT)).encode() + b"\0")
    digest.update(path.read_bytes() + b"\0")
  return digest.hexdigest()


def output_fingerprint(outputs: list[Path]) -> str:
  digest = hashlib.sha256()
  for path in outputs:
    if not path.is_file():
      return ""
    digest.update(path.name.encode() + b"\0")
    with path.open("rb") as source:
      for chunk in iter(lambda: source.read(1024 * 1024), b""):
        digest.update(chunk)
  return digest.hexdigest()


def main() -> int:
  if len(sys.argv) != 3 or sys.argv[1] not in ("dev", "release") or sys.argv[2] not in ("check", "write"):
    print("usage: rust_build_cache.py dev|release check|write", file=sys.stderr)
    return 2
  mode, action = sys.argv[1:]
  directory = target_dir() / ("debug" if mode == "dev" else "release")
  stamp = target_dir() / f".isos-rust-{mode}-inputs.sha256"
  outputs = [directory / name for name in PROGRAMS]
  value = fingerprint(mode) + "\n" + output_fingerprint(outputs)
  if action == "check":
    return 0 if stamp.is_file() and stamp.read_text().strip() == value and all(path.is_file() for path in outputs) else 1
  if not all(path.is_file() for path in outputs):
    print("Rust build outputs missing; refusing to cache", file=sys.stderr)
    return 1
  stamp.parent.mkdir(parents=True, exist_ok=True)
  stamp.write_text(value + "\n")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
