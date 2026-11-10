"""EI public entry points."""

from __future__ import annotations

import subprocess
from pathlib import Path


def test_ei_public_commands() -> None:
  """Each public command resolves its installed implementation."""
  root = Path(__file__).resolve().parents[2]
  for command, expected in (
    ("ei", "EI decision runtime"),
    ("ei-config", "usage: ei-config"),
  ):
    result = subprocess.run(
      [str(root / "bin" / command), "--help"],
      check=True,
      capture_output=True,
      text=True,
    )
    assert expected in result.stdout
