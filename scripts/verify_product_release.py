#!/usr/bin/env python3
"""Require one tagged, version-aligned source commit for a product release."""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]


def git(*arguments: str) -> str:
  return subprocess.check_output(["git", "-C", str(ROOT), *arguments], text=True).strip()


def verify(tag: str) -> None:
  if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag):
    raise ValueError("release tag must be vMAJOR.MINOR.PATCH")
  version = tag[1:]
  cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
  ei = tomllib.loads((ROOT / "ei/pyproject.toml").read_text())["project"]["version"]
  ui = json.loads((ROOT / "ui/package.json").read_text())["version"]
  if (cargo, ei, ui) != (version, version, version):
    raise ValueError(f"product versions differ: tag={version}, Rust={cargo}, EI={ei}, UI={ui}")
  head = git("rev-parse", "HEAD")
  target = git("rev-parse", f"refs/tags/{tag}^{{commit}}")
  master = git("rev-parse", "refs/remotes/origin/master")
  if head != target or head != master:
    raise ValueError("release tag, checked-out commit, and origin/master must be identical")
  print(f"Verified IsOS {version} release at {head}")


def main() -> None:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("tag")
  args = parser.parse_args()
  verify(args.tag)


if __name__ == "__main__":
  main()
