#!/usr/bin/env python3
"""Generate local folder navigation in the English and Chinese documentation trees."""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
START = "<!-- doc-navigation:start -->"
END = "<!-- doc-navigation:end -->"
TREES = (ROOT / "docs", ROOT / "docs_cn")


def title(path: Path) -> str:
  """Use the document's first heading as its navigation label."""
  for line in path.read_text(encoding="utf-8").splitlines():
    match = re.match(r"^# (.+?)\s*$", line)
    if match:
      return re.sub(r"\[([^]]+)\]\([^)]*\)", r"\1", match.group(1))
  return path.stem.replace("_cn", "").replace("-", " ")


def link(source: Path, target: Path, label: str) -> str:
  relative = os.path.relpath(target, source.parent)
  if target.is_dir() and not relative.endswith("/"):
    relative += "/"
  return f"[{label}]({relative})"


def index(folder: Path) -> Path | None:
  for name in ("README.md", "README_cn.md"):
    candidate = folder / name
    if candidate.is_file():
      return candidate
  return None


def render(path: Path, tree: Path) -> str:
  chinese = tree.name == "docs_cn"
  folder = path.parent
  parent = folder.parent
  if folder == tree:
    parent_target = ROOT / "README.md"
    parent_label = "仓库" if chinese else "Repository"
  else:
    parent_target = index(parent) or parent
    parent_label = "上级目录" if chinese else "Parent folder"
  siblings = sorted((p for p in folder.glob("*.md") if p != path), key=lambda p: p.name)
  children = sorted((p for p in folder.iterdir() if p.is_dir() and any(p.rglob("*.md"))), key=lambda p: p.name)
  current = folder.relative_to(tree).as_posix()
  current = tree.name if current == "." else current
  summary = f"目录：{current}" if chinese else f"Folder: {current}"
  intro = "浏览本目录的文档及下级目录。" if chinese else "Browse documents and subfolders here."
  lines = [START, "<details>", f"<summary>{summary}</summary>", "", intro, "", f"- {link(path, parent_target, parent_label)}"]
  if siblings:
    lines.append("- 同级文档：" if chinese else "- Documents in this folder:")
    lines.extend(f"  - {link(path, sibling, title(sibling))}" for sibling in siblings)
  if children:
    lines.append("- 下级目录：" if chinese else "- Subfolders:")
    for child in children:
      target = index(child) or child
      lines.append(f"  - {link(path, target, child.name + '/')}")
  return "\n".join([*lines, "", "</details>", END])


def update(path: Path, tree: Path, *, check: bool) -> bool:
  source = path.read_text(encoding="utf-8")
  block = render(path, tree)
  if START in source or END in source:
    if source.count(START) != 1 or source.count(END) != 1:
      raise ValueError(f"Unbalanced navigation markers: {path}")
    result = re.sub(re.escape(START) + r".*?" + re.escape(END), lambda _: block, source, flags=re.S)
  else:
    lines = source.splitlines(keepends=True)
    heading = next((i for i, line in enumerate(lines) if line.startswith("# ")), None)
    if heading is None:
      lines.insert(0, block + "\n\n")
    else:
      lines.insert(heading + 1, "\n" + block + "\n")
    result = "".join(lines)
  if result == source:
    return False
  if not check:
    path.write_text(result, encoding="utf-8")
  return True


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--check", action="store_true", help="report stale navigation without writing")
  args = parser.parse_args()
  changed = []
  for tree in TREES:
    for path in sorted(tree.rglob("*.md")):
      if update(path, tree, check=args.check):
        changed.append(path.relative_to(ROOT))
  print(f"{'Stale' if args.check else 'Updated'} navigation: {len(changed)} of {sum(1 for tree in TREES for _ in tree.rglob('*.md'))} pages")
  for path in changed:
    print(path)
  return 1 if args.check and changed else 0


if __name__ == "__main__":
  sys.exit(main())
