"""Build the Linux node ABI module and mark EI wheels as platform-specific."""

from __future__ import annotations

import subprocess
from pathlib import Path

from setuptools import find_packages, setup
from setuptools.command.build_py import build_py
from wheel.bdist_wheel import bdist_wheel


class BuildPythonWithNode(build_py):
  def run(self) -> None:
    super().run()
    root = Path(__file__).parent
    target = Path(self.build_lib) / "ei/native/libei_demo_stage.so"
    target.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
      ["cc", "-std=c11", "-O2", "-fPIC", "-shared", "-I", str(root / "native"),
       str(root / "native/demo_stage.c"), "-o", str(target)],
      check=True,
    )


class BinaryWheel(bdist_wheel):
  def finalize_options(self) -> None:
    super().finalize_options()
    self.root_is_pure = False

  def get_tag(self) -> tuple[str, str, str]:
    return "py3", "none", super().get_tag()[2]


EI_SUBPACKAGES = {"contracts", "core", "io", "lib"}
packages = ["ei", "ei.generated"]
packages.extend(
  f"ei.{name}" for name in find_packages(where=".")
  if name.split(".")[0] in EI_SUBPACKAGES
)
packages.extend(
  f"ei.{name}" for name in find_packages(where=".") if name.startswith("generated.")
)
setup(
  packages=packages,
  package_dir={"ei": "."},
  cmdclass={"build_py": BuildPythonWithNode, "bdist_wheel": BinaryWheel},
)
