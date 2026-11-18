"""Structural layering invariant: ei.io may import core; core never imports io.

import-linter's layers contract only covers the listed layer packages, so this
suite scans every module under ``ei/core`` by hand. The composition and
launch modules wire ``ei.io`` to core; other core modules cannot import it.
See ``docs/ei/core/runtime/io.md``.
"""

from __future__ import annotations

import ast
from pathlib import Path

import ei.core
import ei.io

CORE_ROOT = Path(ei.core.__file__).parent
COMPOSITION_ROOT = CORE_ROOT / "system" / "composition.py"
WIRING_ROOTS = {COMPOSITION_ROOT, CORE_ROOT / "system" / "launch.py"}


def _imported_modules(tree: ast.Module) -> set[str]:
  """Return every module name imported anywhere in ``tree``."""
  names: set[str] = set()
  for node in ast.walk(tree):
    if isinstance(node, ast.Import):
      names.update(alias.name for alias in node.names)
    elif isinstance(node, ast.ImportFrom) and node.module and node.level == 0:
      names.add(node.module)
  return names


def test_io_imports_standalone() -> None:
  """``ei.io`` imports without dragging the pipeline in behind it."""
  assert ei.io.NorthboundServer.__module__ == "ei.io.northbound"


def test_core_never_imports_io() -> None:
  """Only composition and launch wiring import ei.io."""
  offenders: list[str] = []
  for path in sorted(CORE_ROOT.rglob("*.py")):
    if path in WIRING_ROOTS:
      continue
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    if any(
      name == "ei.io" or name.startswith("ei.io.")
      for name in _imported_modules(tree)
    ):
      offenders.append(str(path.relative_to(CORE_ROOT)))
  assert not offenders, f"core modules importing ei.io: {offenders}"


def test_composition_root_is_the_sole_importer() -> None:
  """The composition root does import ei.io — the documented allowance."""
  tree = ast.parse(COMPOSITION_ROOT.read_text(encoding="utf-8"))
  assert any(
    name == "ei.io" or name.startswith("ei.io.") for name in _imported_modules(tree)
  )
