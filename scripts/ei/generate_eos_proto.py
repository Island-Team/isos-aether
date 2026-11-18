#!/usr/bin/env python3
"""Generate Python bindings from the unified repository's EOS contracts."""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import tempfile
from pathlib import Path, PurePosixPath

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from grpc_tools import protoc

from ei.lib.resources import repository_resources

EOS_CONTRACT_SOURCE = "contract"
PROTO_IMPORT = re.compile(r'^\s*import\s+"([^"]+)"\s*;', re.MULTILINE)


def _inputs(contract_root: Path) -> tuple[Path, ...]:
  """Return the transitive source closure of the three consumed EOS protos."""
  pending = [
    PurePosixPath("gateway.proto"),
    PurePosixPath("capabilities.proto"),
    PurePosixPath("skills.proto"),
  ]
  found: set[PurePosixPath] = set()
  while pending:
    relative = pending.pop()
    if relative in found:
      continue
    source = contract_root / relative
    if not source.is_file():
      # Well-known and google API imports come from grpcio-tools/common protos.
      continue
    found.add(relative)
    pending.extend(
      PurePosixPath(imported)
      for imported in PROTO_IMPORT.findall(source.read_text(encoding="utf-8"))
    )
  return tuple(contract_root / item for item in sorted(found))


def _files(root: Path) -> dict[str, bytes]:
  return {
    path.relative_to(root).as_posix(): path.read_bytes()
    for path in root.rglob("*")
    if path.is_file() and path.suffix in {".py", ".pyi"}
  } if root.is_dir() else {}


def main(argv: list[str] | None = None) -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument(
    "--check", action="store_true", help="verify checked-in bindings without writing"
  )
  args = parser.parse_args(argv)
  root = Path(__file__).resolve().parents[2] / "ei"
  resources = repository_resources(root)
  gateway = resources.require("proto/gateway.proto")
  contract_root = gateway.parent
  output = root / "generated"
  protos = _inputs(contract_root)
  if gateway not in protos:
    raise RuntimeError("EOS gateway contract is not in the generated source closure")
  try:
    import grpc_tools

    bundled = Path(grpc_tools.__file__).resolve().parent / "_proto"
  except (ImportError, TypeError) as exc:
    raise RuntimeError("grpcio-tools is required to generate EOS bindings") from exc

  with tempfile.TemporaryDirectory() as temporary_output:
    result = protoc.main(
      [
        "grpc_tools.protoc",
        f"-I{contract_root}",
        f"-I{bundled}",
        f"--python_out={temporary_output}",
        f"--pyi_out={temporary_output}",
        f"--grpc_python_out={temporary_output}",
        *(str(proto) for proto in protos),
      ]
    )
    if result != 0:
      return result

    generated = Path(temporary_output)
    if not (generated / "gateway_pb2.py").is_file():
      raise RuntimeError("protoc did not generate the gateway contract")
    # The repository carries Google API source imports for descriptor builds;
    # EI uses the installed google.api bindings rather than vendoring them.
    shutil.rmtree(generated / "google", ignore_errors=True)
    for path in (*generated.rglob("*.py"), *generated.rglob("*.pyi")):
      content = path.read_text(encoding="utf-8")
      content = re.sub(
        r"(?m)^from ((?:perception|robot|system|external|skills|workflow)(?:\.[a-z_]+)*) import ",
        r"from ei.generated.eos.\1 import ",
        content,
      )
      content = re.sub(
        r"(?m)^import (gateway|skills|capabilities)_pb2(\b)",
        r"from ei.generated.eos import \1_pb2\2",
        content,
      )
      for stem in ("gateway", "skills", "capabilities"):
        content = content.replace(
          f"{stem}_pb2_grpc.py", f"ei/generated/eos/{stem}_pb2_grpc.py"
        )
      content = f"# Generated from in-repository {EOS_CONTRACT_SOURCE}.\n" + content
      path.write_text(content, encoding="utf-8")
    for directory in (
      generated,
      *[path for path in generated.rglob("*") if path.is_dir()],
    ):
      (directory / "__init__.py").touch()
    destination = output / "eos"
    fresh_files = _files(generated)
    checked_in_files = _files(destination)
    if fresh_files == checked_in_files:
      return 0
    if args.check:
      print("EI EOS bindings differ; run scripts/ei/generate_eos_proto.py", file=sys.stderr)
      for name in sorted(fresh_files.keys() | checked_in_files.keys()):
        if fresh_files.get(name) != checked_in_files.get(name):
          print(f"  {name}", file=sys.stderr)
      return 1
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output) as temporary_stage:
      stage = Path(temporary_stage) / "eos"
      backup = Path(temporary_stage) / "previous-eos"
      shutil.copytree(generated, stage)
      if destination.exists():
        destination.rename(backup)
      try:
        stage.rename(destination)
      except Exception:
        if backup.exists():
          backup.rename(destination)
        raise
  return 0


if __name__ == "__main__":
  sys.exit(main())
