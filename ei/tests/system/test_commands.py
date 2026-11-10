"""Managed EI command smoke tests from outside the checkout."""

from __future__ import annotations

import os
import pty
import subprocess
from pathlib import Path

import pytest


@pytest.mark.parametrize("machine", ["aarch64", "x86_64"])
def test_ei_build_recipe_uses_synchronized_environment(tmp_path: Path, machine: str) -> None:
  root = Path(__file__).resolve().parents[2]
  commands = tmp_path / "commands"
  commands.mkdir()
  log = tmp_path / "uv.log"
  uname = commands / "uname"
  uname.write_text(f"#!/bin/sh\nprintf '%s\\n' '{machine}'\n")
  uname.chmod(0o755)
  uv = commands / "uv"
  uv.write_text(f"#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{log}'\n")
  uv.chmod(0o755)
  environment = {**os.environ, "PATH": f"{commands}:{os.environ['PATH']}"}
  subprocess.run(
    [str(root / "scripts/platform/dispatch.sh"), "build", "--wheel"],
    cwd=tmp_path,
    env=environment,
    check=True,
  )
  assert log.read_text().splitlines() == [
    "sync --python 3.12 --all-extras --no-install-project",
    "sync --python 3.12 --all-extras --no-build-isolation",
    "build --no-build-isolation --wheel",
  ]


def test_ei_tty_invokes_managed_core_without_private_console(tmp_path: Path) -> None:
  root = Path(__file__).resolve().parents[2]
  prefix = tmp_path / "prefix"
  (prefix / "share/contracts").mkdir(parents=True)
  (prefix / "libexec").mkdir()
  fake_python = tmp_path / "python"
  called = tmp_path / "called"
  fake_python.write_text(
    "#!/bin/sh\n"
    'printf "%s\\n" "$@" > "$EI_TEST_ARGS"\n',
    encoding="utf-8",
  )
  fake_python.chmod(0o755)
  master, slave = pty.openpty()
  try:
    subprocess.run(
      [str(root / "bin/ei"), "--managed"],
      cwd=tmp_path,
      env={
        **os.environ,
        "ISLAND_PREFIX": str(prefix),
        "EI_PYTHON": str(fake_python),
        "EI_TEST_ARGS": str(called),
      },
      stdout=slave,
      stderr=subprocess.PIPE,
      text=True,
      check=True,
    )
  finally:
    os.close(master)
    os.close(slave)
  assert called.read_text(encoding="utf-8").splitlines() == ["-m", "ei.main", "--managed"]
