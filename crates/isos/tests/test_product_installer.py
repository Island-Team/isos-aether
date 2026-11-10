"""Stubbed integration checks for the root product installer."""
from __future__ import annotations

import pathlib
import shutil
import subprocess
import tempfile
import os
import json
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]


class ProductInstallerTest(unittest.TestCase):
  def setUp(self) -> None:
    self.temp = tempfile.TemporaryDirectory()
    self.addCleanup(self.temp.cleanup)
    self.root = pathlib.Path(self.temp.name)
    self.checkout = self.root / "checkout"
    self.prefix = self.root / "prefix"
    self.checkout.mkdir()
    shutil.copy2(ROOT / "install.sh", self.checkout / "install.sh")
    shutil.copy2(ROOT / "isos.sh", self.checkout / "isos.sh")
    for name in ("Cargo.toml", "LICENSE"):
      shutil.copy2(ROOT / name, self.checkout / name)
    for item in ("ui/package.json", "ui/dist/local/index.html", "ui/dist/tui/index.js", "ei/pyproject.toml"):
      file = self.checkout / item
      file.parent.mkdir(parents=True, exist_ok=True)
      file.write_text("stub")
    (self.checkout / "share/eos").mkdir(parents=True)
    (self.checkout / "share/eos/contracts.manifest").write_text("stub")
    for directory in ("ei/bin", "ei/libexec", "ei/dist", "crates/isos"):
      (self.checkout / directory).mkdir(parents=True)
    (self.checkout / "config/ei").mkdir(parents=True)
    (self.checkout / "config/ei/island.toml").write_text('share = ["../share"]\n')
    factory = self.checkout / "share/ei/defaults/ei/modes.toml"
    factory.parent.mkdir(parents=True)
    product_defaults = self.checkout / "share/isos/defaults/island.toml"
    product_defaults.parent.mkdir(parents=True)
    product_defaults.write_text('name = "IsOS"\nshare = ["share"]\n')
    (self.checkout / "config/ei/modes.toml").write_text("[modes]\n")
    factory.symlink_to("../../../../config/ei/modes.toml")
    ei = self.checkout / "ei/bin/ei"
    ei.write_text("#!/bin/sh\nexit 0\n")
    ei.chmod(0o755)
    for item in ("config/isos/isos.toml", "config/isos/isos.example.toml", "config/isos/install.toml", "config/isos/launch.toml", "config/isosui.toml"):
      file = self.checkout / item
      file.parent.mkdir(parents=True, exist_ok=True)
      shutil.copy2(ROOT / item, file)
    (self.checkout / "ei/dist/ei-0.2.1-py3-none-any.whl").write_bytes(b"stub")
    fake_bin = self.root / "fake-bin"
    fake_bin.mkdir()
    uv = fake_bin / "uv"
    uv.write_text("#!/bin/sh\nexit 0\n")
    uv.chmod(0o755)
    self.env = {**os.environ, "PATH": f"{fake_bin}:{os.environ.get('PATH', '')}"}
    binary = self.checkout / "target/release/isos"
    binary.parent.mkdir(parents=True)
    binary.write_text("#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'isos 0.2.1'; else env | grep ^ISOS; fi\n")
    binary.chmod(0o755)
    for name in ("aether-profile", "aether-runtime", "aether-http-bridge", "aether-config-server"):
      program = binary.parent / name
      program.write_text("#!/bin/sh\nexit 0\n")
      program.chmod(0o755)

  def run_install(self, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
      ["bash", str(self.checkout / "install.sh"), "--prefix", str(self.prefix), *args],
      text=True, capture_output=True, check=False, env=self.env,
    )

  def test_source_install_and_bundle_roundtrip(self) -> None:
    archive = self.root / "release.tar.gz"
    result = self.run_install("--no-build", "--bundle-output", str(archive))
    self.assertEqual(result.returncode, 0, result.stderr)
    self.assertTrue((self.prefix / "current/MANIFEST.sha256").is_file())
    for name in ("aether-profile", "aether-runtime", "aether-http-bridge", "aether-config-server"):
      self.assertTrue((self.prefix / "current/libexec" / name).is_file())
    env_reply = subprocess.run([str(self.prefix / "bin/isos"), "--dump-env"], cwd=self.root, text=True, capture_output=True, check=True)
    self.assertIn("ISOS_AETHER_BIN_DIR=", env_reply.stdout)
    self.assertIn("ISOS_MODE=product", env_reply.stdout)
    self.assertIn("ISOSUI_TUI_ENTRY=", env_reply.stdout)
    workspace_reply = subprocess.run(
      [str(self.prefix / "bin/isos"), "--dump-env"], cwd=self.checkout,
      text=True, capture_output=True, check=True, env=self.env,
    )
    self.assertIn(f"ISOS_HOME={self.checkout}", workspace_reply.stdout)
    self.assertIn(f"ISOS_CONFIG={self.checkout / 'config/isos/isos.toml'}", workspace_reply.stdout)
    self.assertNotIn("ISOS_AETHER_BIN_DIR=", workspace_reply.stdout)
    script_reply = subprocess.run(
      [str(self.checkout / "isos.sh"), "--dump-env"], cwd=self.root,
      text=True, capture_output=True, check=True,
      env={**self.env, "ISOS_INSTALL_ROOT": str(self.prefix), "ISOS_MODE": "product"},
    )
    self.assertIn(f"ISOS_HOME={self.checkout}", script_reply.stdout)
    self.assertIn(f"ISOS_CONFIG={self.checkout / 'config/isos/isos.toml'}", script_reply.stdout)
    script_override = subprocess.run(
      [str(self.checkout / "isos.sh"), "--dump-env"], cwd=self.root,
      text=True, capture_output=True, check=True,
      env={**self.env, "ISOS_INSTALL_ROOT": str(self.prefix), "ISOS_HOME": str(self.root / "custom-home"), "ISOS_MODE": "product"},
    )
    self.assertIn(f"ISOS_HOME={self.root / 'custom-home'}", script_override.stdout)

    self.assertIn(f"ISOS_CONFIG={(self.prefix / 'current').resolve() / 'share/isos/config/isos.toml'}", script_override.stdout)
    override = self.root / "custom-isos.toml"
    override.write_text("# custom\n")
    configured_reply = subprocess.run(
      [str(self.prefix / "bin/isos"), "--dump-env"], cwd=self.checkout,
      text=True, capture_output=True, check=True,
      env={**self.env, "ISOS_CONFIG": str(override)},
    )
    self.assertIn(f"ISOS_HOME={self.checkout}", configured_reply.stdout)
    self.assertIn(f"ISOS_CONFIG={override}", configured_reply.stdout)
    explicit_reply = subprocess.run(
      [str(self.prefix / "bin/isos"), "--dump-env"], cwd=self.checkout,
      text=True, capture_output=True, check=True,
      env={**self.env, "ISOS_HOME": str(self.root / "explicit-home")},
    )
    self.assertIn(f"ISOS_HOME={self.root / 'explicit-home'}", explicit_reply.stdout)
    self.assertIn("ISOS_AETHER_BIN_DIR=", explicit_reply.stdout)
    self.assertTrue((self.prefix / "current/share/isos/eos/contracts.manifest").is_file())
    self.assertEqual((self.prefix / "current/share/isos/LICENSE").read_bytes(), (ROOT / "LICENSE").read_bytes())
    installed_factory = self.prefix / "current/share/isos/ei/share/defaults/ei/modes.toml"
    self.assertTrue(installed_factory.is_file())
    self.assertFalse(installed_factory.is_symlink())
    self.assertTrue((self.prefix / "current/share/isos/ei/share/defaults/island.toml").is_file())
    self.assertIn('share = ["share", ".."]', (self.prefix / "current/share/isos/ei/etc/island.toml").read_text())
    self.assertTrue(archive.is_file())
    release = json.loads((self.prefix / "current/RELEASE.json").read_text())
    self.assertEqual(release["product_version"], "0.2.1")
    self.assertEqual(release["schema_version"], "1")
    checksum = subprocess.run(["sha256sum", "-c", archive.name + ".sha256"], cwd=self.root, text=True, capture_output=True)
    self.assertEqual(checksum.returncode, 0, checksum.stderr)
    second = self.root / "second"
    result = self.run_install("--prefix", str(second), "--bundle", str(archive))
    self.assertEqual(result.returncode, 0, result.stderr)
    self.assertEqual((second / "bin/isos").resolve().read_text().splitlines()[-1], 'exec "$RELEASE_ROOT/libexec/isos" "$@"')

  def test_checkout_launcher_uses_toml_dev_mode(self) -> None:
    binary = self.checkout / "target/debug/isos"
    binary.parent.mkdir(parents=True, exist_ok=True)
    binary.write_text("#!/bin/sh\nenv | grep ^ISOS\n")
    binary.chmod(0o755)
    reply = subprocess.run(
      [str(self.checkout / "isos.sh"), "--dump-env"], cwd=self.root,
      text=True, capture_output=True, check=True, env=self.env,
    )
    self.assertIn("ISOS_MODE=dev", reply.stdout)
    self.assertIn(f"ISOS_AETHER_BIN_DIR={binary.parent}", reply.stdout)

  def test_build_finds_cargo_in_home_when_path_omits_it(self) -> None:
    home = self.root / "home"
    cargo = home / ".cargo/bin/cargo"
    cargo.parent.mkdir(parents=True)
    called = self.root / "cargo-called"
    cargo.write_text(f"#!/bin/sh\nprintf '%s\\n' \"$*\" > '{called}'\n")
    cargo.chmod(0o755)
    npm = self.root / "fake-bin/npm"
    npm.write_text("#!/bin/sh\nexit 0\n")
    npm.chmod(0o755)
    recipe = self.checkout / "ei/scripts/platform/dispatch.sh"
    recipe.parent.mkdir(parents=True)
    recipe.write_text("#!/bin/sh\nexit 0\n")
    recipe.chmod(0o755)
    self.env["HOME"] = str(home)
    self.env.pop("CARGO_HOME", None)
    self.env["PATH"] = f"{self.root}/fake-bin:/usr/bin:/bin"
    self.assertNotIn("cargo", subprocess.run(["/usr/bin/env", "sh", "-c", "command -v cargo"],
                                             env=self.env, text=True, capture_output=True).stdout)
    result = self.run_install()
    self.assertEqual(result.returncode, 0, result.stderr)
    self.assertIn("build --locked --release --workspace", called.read_text())

  def test_missing_eos_binary_does_not_switch_current(self) -> None:
    self.assertEqual(self.run_install("--no-build").returncode, 0)
    before = (self.prefix / "current").resolve()
    (self.checkout / "target/release/aether-profile").unlink()
    result = self.run_install("--no-build")
    self.assertNotEqual(result.returncode, 0)
    self.assertEqual((self.prefix / "current").resolve(), before)

  def test_factory_default_link_outside_component_fails_before_switch(self) -> None:
    self.assertEqual(self.run_install("--no-build").returncode, 0)
    before = (self.prefix / "current").resolve()
    factory = self.checkout / "share/ei/defaults/ei/modes.toml"
    factory.unlink()
    factory.symlink_to(self.root / "outside.toml")
    (self.root / "outside.toml").write_text("[modes]\n")
    result = self.run_install("--no-build")
    self.assertNotEqual(result.returncode, 0)
    self.assertIn("escapes config/ei", result.stderr)
    self.assertEqual((self.prefix / "current").resolve(), before)

  def test_bad_bundle_does_not_switch_current(self) -> None:
    self.assertEqual(self.run_install("--no-build").returncode, 0)
    before = (self.prefix / "current").resolve()
    bad = self.root / "bad.tar.gz"
    bad.write_bytes(b"corrupt")
    result = self.run_install("--bundle", str(bad))
    self.assertNotEqual(result.returncode, 0)
    self.assertEqual((self.prefix / "current").resolve(), before)


if __name__ == "__main__":
  unittest.main()
