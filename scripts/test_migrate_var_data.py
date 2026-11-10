"""Focused checks for the offline var-to-data migration."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from migrate_var_data import migrate, migrate_system, migrate_xdg


class MigrationTests(unittest.TestCase):
  def test_populated_and_empty_source(self) -> None:
    with tempfile.TemporaryDirectory() as temporary:
      root = Path(temporary)
      source = root / "var/lib/eos"
      (source / "custom-store").mkdir(parents=True)
      (source / "custom-store/notes.json").write_text("user", encoding="utf-8")
      (source / "attachments").mkdir()
      (source / "attachments/photo.bin").write_bytes(b"image")
      (source / "primitive-commands.jsonl").write_text("state", encoding="utf-8")
      (root / "var/lib/ei").mkdir()
      migrate(root)
      self.assertEqual((root / "data/eos/user/custom-store/notes.json").read_text(), "user")
      self.assertEqual((root / "data/eos/attachments/photo.bin").read_bytes(), b"image")
      self.assertEqual((root / "data/eos/state/primitive-commands.jsonl").read_text(), "state")
      self.assertTrue((root / "data/ei/state").is_dir())
      self.assertTrue((source / "custom-store/notes.json").exists())
      migrate(root)

  def test_conflict_retains_source_and_destination(self) -> None:
    with tempfile.TemporaryDirectory() as temporary:
      root = Path(temporary)
      source = root / "var/lib/ei"
      source.mkdir(parents=True)
      (source / "memory.db").write_text("old", encoding="utf-8")
      target = root / "data/ei/state"
      target.mkdir(parents=True)
      (target / "memory.db").write_text("new", encoding="utf-8")
      with self.assertRaises(FileExistsError):
        migrate(root)
      self.assertEqual((source / "memory.db").read_text(), "old")
      self.assertEqual((target / "memory.db").read_text(), "new")

  def test_symlink_and_interrupted_copy_are_rejected(self) -> None:
    with tempfile.TemporaryDirectory() as temporary:
      root = Path(temporary)
      source = root / "var/lib/ei"
      source.mkdir(parents=True)
      (source / "link").symlink_to(root)
      with self.assertRaises(ValueError):
        migrate(root)
      (source / "link").unlink()
      pending = root / "data/ei/state.migration-pending"
      pending.mkdir(parents=True)
      with self.assertRaises(FileExistsError):
        migrate(root)
      self.assertTrue(pending.is_dir())

  def test_existing_isos_home_data_is_copied(self) -> None:
    with tempfile.TemporaryDirectory() as temporary:
      root = Path(temporary)
      old = root / "data/generated/default"
      old.mkdir(parents=True)
      (old / "selection.json").write_text("{}", encoding="utf-8")
      old_ei = root / "data/state/default"
      old_ei.mkdir(parents=True)
      (old_ei / "memory.db").write_text("memory", encoding="utf-8")
      migrate(root)
      target = root / "data/isos/state/generated/default/selection.json"
      self.assertEqual(target.read_text(), "{}")
      self.assertTrue((old / "selection.json").exists())
      self.assertEqual((root / "data/ei/state/default/memory.db").read_text(), "memory")

  def test_system_layout_copies_data_and_logs_in_place(self) -> None:
    with tempfile.TemporaryDirectory() as temporary:
      root = Path(temporary)
      owner = root / "var/lib/island/eos"
      (owner / "custom-store").mkdir(parents=True)
      (owner / "custom-store/settings.json").write_text("user", encoding="utf-8")
      (owner / "primitive-commands.jsonl").write_text("state", encoding="utf-8")
      old_log = root / "var/log/island/eos"
      old_log.mkdir(parents=True)
      (old_log / "events.jsonl").write_text("log", encoding="utf-8")
      migrate_system(root)
      self.assertEqual((owner / "user/custom-store/settings.json").read_text(), "user")
      self.assertEqual((owner / "state/primitive-commands.jsonl").read_text(), "state")
      self.assertEqual((owner / "log/events.jsonl").read_text(), "log")
      migrate_system(root)

  def test_system_conflict_retains_both_copies(self) -> None:
    with tempfile.TemporaryDirectory() as temporary:
      root = Path(temporary)
      owner = root / "var/lib/island/ei"
      owner.mkdir(parents=True)
      (owner / "memory.db").write_text("old", encoding="utf-8")
      (owner / "state").mkdir()
      (owner / "state/memory.db").write_text("new", encoding="utf-8")
      with self.assertRaises(FileExistsError):
        migrate_system(root)
      self.assertEqual((owner / "memory.db").read_text(), "old")
      self.assertEqual((owner / "state/memory.db").read_text(), "new")

  def test_simulator_symlink_is_rejected(self) -> None:
    with tempfile.TemporaryDirectory() as temporary:
      root = Path(temporary)
      outside = root / "outside"
      outside.mkdir()
      source = root / "var/lib/eos"
      source.mkdir(parents=True)
      (source / "simulator").symlink_to(outside)
      with self.assertRaises(ValueError):
        migrate(root)
      system = root / "var/lib/island/eos"
      system.mkdir(parents=True)
      (system / "simulator").symlink_to(outside)
      with self.assertRaises(ValueError):
        migrate_system(root)

  def test_xdg_default_isos_data_is_copied(self) -> None:
    with tempfile.TemporaryDirectory() as temporary:
      root = Path(temporary)
      old = root / "isos/packages/example"
      old.mkdir(parents=True)
      (old / "receipt.json").write_text("{}", encoding="utf-8")
      migrate_xdg(root)
      target = root / "island/isos/state/packages/example/receipt.json"
      self.assertEqual(target.read_text(), "{}")
      self.assertTrue((old / "receipt.json").exists())
      migrate_xdg(root)


if __name__ == "__main__":
  unittest.main()
