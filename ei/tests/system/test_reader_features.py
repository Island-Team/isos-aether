"""The selected EI launcher reports capabilities without starting EI."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path


def _launcher() -> Path:
  return Path(__file__).resolve().parents[2] / "bin/ei"


def _probe(
  prefix: Path, *, python: str | None = None
) -> subprocess.CompletedProcess[str]:
  env = dict(os.environ)
  env["ISLAND_PREFIX"] = str(prefix)
  env.pop("ISLAND_PYTHON", None)
  env.pop("PYTHONPATH", None)
  if python is None:
    env.pop("EI_PYTHON", None)
  else:
    env["EI_PYTHON"] = python
  return subprocess.run(
    [str(_launcher()), "--capabilities"],
    cwd=prefix,
    env=env,
    capture_output=True,
    text=True,
    timeout=10,
    check=False,
  )


def test_real_selected_launcher_and_setup_identity_agree(tmp_path: Path) -> None:
  root = Path(__file__).resolve().parents[2]
  runtime = _probe(root, python=sys.executable)
  setup = subprocess.run(
    [str(root / "bin/ei-config"), "setup", "--json"],
    input='{"schema_version":2,"op":"identity","core":"ei"}',
    cwd=tmp_path,
    env={**os.environ, "EI_PYTHON": sys.executable},
    capture_output=True,
    text=True,
    timeout=10,
    check=True,
  )
  assert runtime.returncode == 0
  assert runtime.stderr == ""
  runtime_reply = json.loads(runtime.stdout)
  setup_reply = json.loads(setup.stdout)
  identity = setup_reply["identity"]
  assert identity["core"] == "ei"
  assert identity["product_version"] == "0.2.1"
  assert identity["graph_version"] == "0.2.0"
  assert identity["node_type_version"] == "0.2"
  assert identity["supported_graph_versions"] == ["0.2.0"]
  assert identity["supported_node_type_versions"] == ["0.2"]
  assert len(identity["supported_node_types"]) == 6
  assert runtime_reply == {
    "binary": "ei",
    "product_version": setup_reply["identity"]["product_version"],
    "features": [
      "managed-config-v1",
      "eos-gateway-client-v2",
      "ei-evidence-scheduler-v1",
      "vision-evidence-ingress-v1",
      "vision-inspection-uds-v1",
    ],
  }
  assert len(setup_reply["identity"]["supported_node_types"]) == 6
  assert setup_reply["identity"]["supported_node_types"] == sorted(
    setup_reply["identity"]["supported_node_types"]
  )
  assert setup_reply["identity"]["features"] == [
    "setup-v2",
    "catalog",
    "graph-0.2-ei-projection",
    "health-only-owner-v1",
    "activation-owner-v1",
    "arm-monitor-owner-v1",
  ]
  assert not any(key.startswith("supported_") for key in runtime_reply)


def test_probe_module_does_not_import_runtime_or_transport(tmp_path: Path) -> None:
  root = Path(__file__).resolve().parents[2]
  script = """import ei.reader_features as probe
import sys
probe.main()
assert 'ei.main' not in sys.modules
assert 'ei.core' not in sys.modules
assert 'grpc' not in sys.modules
"""
  result = subprocess.run(
    [
      sys.executable,
      "-B",
      "-c",
      script,
    ],
    cwd=tmp_path,
    env={**os.environ, "PYTHONPATH": str(root / "src")},
    capture_output=True,
    text=True,
    timeout=10,
    check=False,
  )
  assert result.returncode == 0, result.stderr
  assert json.loads(result.stdout)["binary"] == "ei"
  assert list(tmp_path.iterdir()) == []


def test_missing_default_interpreter_never_bootstraps(tmp_path: Path) -> None:
  source = tmp_path / "source"
  (source / "src/ei").mkdir(parents=True)
  (source / "pyproject.toml").write_text("[project]\nname='ei'\n")
  fake_bin = tmp_path / "bin"
  fake_bin.mkdir()
  uv_called = tmp_path / "uv-called"
  uv = fake_bin / "uv"
  uv.write_text(f"#!/bin/sh\ntouch '{uv_called}'\nexit 99\n")
  uv.chmod(0o755)
  env = dict(os.environ)
  env["ISLAND_PREFIX"] = str(source)
  env["PATH"] = f"{fake_bin}:/usr/bin:/bin"
  for name in ("EI_PYTHON", "ISLAND_PYTHON", "PYTHONPATH"):
    env.pop(name, None)
  result = subprocess.run(
    [str(_launcher()), "--capabilities"],
    cwd=source,
    env=env,
    capture_output=True,
    text=True,
    timeout=10,
    check=False,
  )
  assert result.returncode == 127
  assert result.stdout == ""
  assert "interpreter" in result.stderr
  assert not uv_called.exists()
  assert not (source / ".venv").exists()


def test_probe_uses_selected_interpreter_and_installed_module_tree(
  tmp_path: Path,
) -> None:
  prefix = tmp_path / "portable"
  module = prefix / "lib/island/ei/reader_features.py"
  module.parent.mkdir(parents=True)
  (prefix / "share/contracts").mkdir(parents=True)
  (prefix / "libexec").mkdir()
  (module.parent / "__init__.py").write_text("")
  module.write_text('print("selected-module-v1")\n')
  interpreter = tmp_path / "selected-python"
  called = tmp_path / "interpreter-called"
  interpreter.write_text(
    f"#!/bin/sh\nprintf '%s\\n' \"$*\" > '{called}'\nexec '{sys.executable}' \"$@\"\n"
  )
  interpreter.chmod(0o755)

  first = _probe(prefix, python=str(interpreter))
  assert first.returncode == 0
  assert first.stdout.strip() == "selected-module-v1"
  assert called.read_text().strip() == "-B -m ei.reader_features"
  assert not list(prefix.rglob("__pycache__"))

  module.write_text('print("selected-module-v2-updated")\n')
  second = _probe(prefix, python=str(interpreter))
  assert second.returncode == 0
  assert second.stdout.strip() == "selected-module-v2-updated"
  assert not list(prefix.rglob("__pycache__"))


def test_setup_and_runtime_use_same_selected_installed_module_root(
  tmp_path: Path,
) -> None:
  prefix = tmp_path / "portable"
  (prefix / "share/contracts").mkdir(parents=True)
  (prefix / "libexec").mkdir()
  selected_lib = tmp_path / "selected-lib"
  package = selected_lib / "ei"
  package.mkdir(parents=True)
  (package / "__init__.py").write_text("")
  (package / "reader_features.py").write_text('print("runtime-selected")\n')
  (package / "config_cli.py").write_text('print("setup-selected")\n')
  env = dict(os.environ)
  env.update(
    {
      "ISLAND_PREFIX": str(prefix),
      "ISLAND_LIB": str(selected_lib),
      "EI_PYTHON": sys.executable,
    }
  )
  env.pop("PYTHONPATH", None)
  env.pop("ISLAND_PYTHON", None)
  for command, expected in (
    ([_launcher(), "--capabilities"], "runtime-selected"),
    ([_launcher().with_name("ei-config"), "setup", "--json"], "setup-selected"),
  ):
    result = subprocess.run(
      [str(part) for part in command],
      cwd=prefix,
      env=env,
      capture_output=True,
      text=True,
      timeout=10,
      check=False,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == expected
  assert not list(selected_lib.rglob("__pycache__"))


def test_missing_explicit_interpreter_fails_without_side_effects(
  tmp_path: Path,
) -> None:
  result = _probe(tmp_path, python=str(tmp_path / "missing-python"))
  assert result.returncode == 127
  assert result.stdout == ""
  assert list(tmp_path.iterdir()) == []


def test_capabilities_rejects_launch_arguments_before_bootstrap(
  tmp_path: Path,
) -> None:
  env = {**os.environ, "ISLAND_PREFIX": str(tmp_path)}
  env.pop("EI_PYTHON", None)
  result = subprocess.run(
    [str(_launcher()), "--managed", "--capabilities"],
    cwd=tmp_path,
    env=env,
    capture_output=True,
    text=True,
    timeout=10,
    check=False,
  )
  assert result.returncode == 2
  assert result.stdout == ""
  assert list(tmp_path.iterdir()) == []
