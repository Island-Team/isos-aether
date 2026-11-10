#!/usr/bin/env python3
"""Copy durable files from a legacy source or portable var tree into data.

Run only after stopping IsOS, EOS, and EI. The source is never removed.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import stat
import sys
from pathlib import Path


def plain_ancestors(path: Path) -> None:
  """Reject symlinked ancestors before reading or publishing a path."""
  for ancestor in reversed(path.parents):
    if ancestor.is_symlink():
      raise ValueError(f"symlinked parent: {ancestor}")


def digest(path: Path) -> str:
  """Hash a file without loading large attachments into memory."""
  value = hashlib.sha256()
  with path.open("rb") as stream:
    for block in iter(lambda: stream.read(1024 * 1024), b""):
      value.update(block)
  return value.hexdigest()


def inventory(root: Path) -> dict[str, str]:
  """Hash regular files, rejecting links and special files."""
  plain_ancestors(root)
  result: dict[str, str] = {}
  if not root.exists():
    return result
  if not stat.S_ISDIR(root.lstat().st_mode):
    raise ValueError(f"not a plain directory: {root}")
  for base, dirs, files in os.walk(root, followlinks=False):
    for name in dirs + files:
      path = Path(base) / name
      mode = path.lstat().st_mode
      if stat.S_ISLNK(mode) or not (stat.S_ISDIR(mode) or stat.S_ISREG(mode)):
        raise ValueError(f"unsupported file: {path}")
      if stat.S_ISREG(mode):
        result[str(path.relative_to(root))] = digest(path)
  return result


def copy_tree(source: Path, target: Path) -> None:
  """Publish one verified copy without replacing an existing destination."""
  plain_ancestors(target)
  expected = inventory(source)
  if not source.exists():
    return
  if target.exists() or target.is_symlink():
    if not target.is_symlink() and inventory(target) == expected:
      return
    raise FileExistsError(f"destination already exists with different contents: {target}")
  target.parent.mkdir(parents=True, exist_ok=True)
  staging = target.with_name(target.name + ".migration-pending")
  if staging.exists() or staging.is_symlink():
    raise FileExistsError(f"previous incomplete migration: {staging}")
  try:
    shutil.copytree(source, staging, symlinks=True)
    if inventory(staging) != expected or inventory(source) != expected:
      raise RuntimeError(f"source changed during migration: {source}")
    staging.rename(target)
  except Exception:
    if staging.exists():
      shutil.rmtree(staging)
    raise


def copy_file(source: Path, target: Path) -> None:
  """Publish one verified regular file without replacing a destination."""
  plain_ancestors(source)
  plain_ancestors(target)
  if not stat.S_ISREG(source.lstat().st_mode):
    raise ValueError(f"not a plain file: {source}")
  if target.exists() or target.is_symlink():
    if target.is_file() and not target.is_symlink() and digest(target) == digest(source):
      return
    raise FileExistsError(f"destination already exists with different contents: {target}")
  target.parent.mkdir(parents=True, exist_ok=True)
  staging = target.with_name(target.name + ".migration-pending")
  if staging.exists() or staging.is_symlink():
    raise FileExistsError(f"previous incomplete migration: {staging}")
  expected = digest(source)
  try:
    shutil.copy2(source, staging, follow_symlinks=False)
    if digest(staging) != expected:
      raise RuntimeError(f"source changed during migration: {source}")
    if digest(source) != expected:
      raise RuntimeError(f"source changed during migration: {source}")
    staging.rename(target)
  except Exception:
    staging.unlink(missing_ok=True)
    raise


def migrate(root: Path) -> None:
  """Copy durable owner files; caches and temporary files are rebuilt."""
  if root.is_symlink():
    raise ValueError(f"root is a symlink: {root}")
  legacy = root / "var" / "lib"
  destination = root / "data"
  if destination.is_symlink() or legacy.is_symlink():
    raise ValueError("legacy or destination root is a symlink")
  for owner in ("isos", "eos", "ei"):
    source = legacy / owner
    if not source.exists():
      continue
    if source.is_symlink() or not source.is_dir():
      raise ValueError(f"not a plain owner directory: {source}")
    if owner == "eos":
      copy_tree(source / "attachments", destination / owner / "attachments")
      copy_tree(source / "custom-store", destination / owner / "user" / "custom-store")
      simulator = source / "simulator"
      if simulator.is_symlink() or (simulator.exists() and not simulator.is_dir()):
        raise ValueError(f"not a plain simulator directory: {simulator}")
      for name in ("library", "maps"):
        copy_tree(simulator / name, destination / owner / "user" / "simulator" / name)
      for child in source.iterdir():
        if child.name in {"attachments", "custom-store", "simulator"}:
          continue
        target = destination / owner / "state" / child.name
        if child.is_dir() and not child.is_symlink():
          copy_tree(child, target)
        else:
          copy_file(child, target)
      if simulator.exists():
        for child in simulator.iterdir():
          if child.name in {"library", "maps"}:
            continue
          target = destination / owner / "state" / "simulator" / child.name
          if child.is_dir() and not child.is_symlink():
            copy_tree(child, target)
          else:
            copy_file(child, target)
    else:
      copy_tree(source, destination / owner / "state")
  # ISOS_HOME used data/ directly, independently of the legacy var tree.
  migrate_isos_data(root / "data", root / "data")


def migrate_isos_data(old_isos: Path, new_data: Path) -> None:
  """Copy the previous IsOS data layout, including its managed EI state."""
  if old_isos.is_symlink() or new_data.is_symlink():
    raise ValueError("old or new IsOS data root is a symlink")
  for child in old_isos.iterdir() if old_isos.exists() else ():
    if old_isos == new_data and child.name in {"isos", "eos", "ei"}:
      continue
    if child.name == "state":
      target = new_data / "ei" / "state"
    elif child.name in {"logs", "log"}:
      target = new_data / "isos" / "log"
    elif child.name in {"cache", "tmp", "run"}:
      continue
    else:
      target = new_data / "isos" / "state" / child.name
    if child.is_dir() and not child.is_symlink():
      copy_tree(child, target)
    else:
      copy_file(child, target)


def migrate_xdg(root: Path) -> None:
  """Copy the former XDG_DATA_HOME/isos tree to island owner roots."""
  migrate_isos_data(root / "isos", root / "island")


def migrate_system(root: Path) -> None:
  """Copy old system owner data in place into owner-first role folders."""
  old_base = root / "var/lib/island"
  if old_base.is_symlink():
    raise ValueError(f"legacy root is a symlink: {old_base}")
  for owner in ("isos", "eos", "ei"):
    source = old_base / owner
    if not source.exists():
      continue
    if source.is_symlink() or not source.is_dir():
      raise ValueError(f"not a plain owner directory: {source}")
    # Snapshot names before creating role directories inside the same owner.
    children = list(source.iterdir())
    for child in children:
      if child.name in {"state", "user", "cache", "log", "tmp", "run"}:
        continue
      if owner == "eos" and child.name == "attachments":
        # Already at its new location.
        inventory(child)
        continue
      if owner == "eos" and child.name == "custom-store":
        target = source / "user/custom-store"
      elif owner == "eos" and child.name == "simulator":
        if child.is_symlink() or not child.is_dir():
          raise ValueError(f"not a plain simulator directory: {child}")
        for entry in child.iterdir():
          role = "user" if entry.name in {"library", "maps"} else "state"
          target = source / role / "simulator" / entry.name
          if entry.is_dir() and not entry.is_symlink():
            copy_tree(entry, target)
          else:
            copy_file(entry, target)
        continue
      else:
        target = source / "state" / child.name
      if child.is_dir() and not child.is_symlink():
        copy_tree(child, target)
      else:
        copy_file(child, target)
    old_log = root / "var/log/island" / owner
    copy_tree(old_log, source / "log")


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("root", type=Path, help="source or portable installation root")
  mode = parser.add_mutually_exclusive_group()
  mode.add_argument("--system", action="store_true", help="migrate system paths below root")
  mode.add_argument("--xdg", action="store_true", help="treat root as XDG_DATA_HOME")
  args = parser.parse_args()
  try:
    selected_root = args.root.resolve(strict=True)
    if args.system:
      migrate_system(selected_root)
    elif args.xdg:
      migrate_xdg(selected_root)
    else:
      migrate(selected_root)
  except (OSError, ValueError, RuntimeError) as exc:
    print(f"migration failed: {exc}", file=sys.stderr)
    return 1
  print("durable files copied and verified; old files retained")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
