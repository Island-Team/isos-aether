"""Unit checks for the unified product release gate."""
from __future__ import annotations

import importlib.util
import pathlib
import tempfile
import unittest
from unittest.mock import patch

ROOT = pathlib.Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("product_release", ROOT / "scripts/verify_product_release.py")
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ProductReleaseGateTest(unittest.TestCase):
  def setUp(self) -> None:
    self.temp = tempfile.TemporaryDirectory()
    self.addCleanup(self.temp.cleanup)
    self.root = pathlib.Path(self.temp.name)
    (self.root / "ei").mkdir()
    (self.root / "ui").mkdir()
    (self.root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.2.1"\n')
    (self.root / "ei/pyproject.toml").write_text('[project]\nversion = "0.2.1"\n')
    (self.root / "ui/package.json").write_text('{"version":"0.2.1"}')

  def test_matching_tag_and_versions_pass(self) -> None:
    with patch.object(MODULE, "ROOT", self.root), patch.object(MODULE, "git", return_value="abc"):
      MODULE.verify("v0.2.1")

  def test_mismatched_versions_or_master_reject(self) -> None:
    with patch.object(MODULE, "ROOT", self.root), patch.object(MODULE, "git", return_value="abc"):
      with self.assertRaises(ValueError):
        MODULE.verify("v0.2.2")
    with patch.object(MODULE, "ROOT", self.root), patch.object(MODULE, "git", side_effect=["abc", "abc", "def"]):
      with self.assertRaises(ValueError):
        MODULE.verify("v0.2.1")


if __name__ == "__main__":
  unittest.main()
