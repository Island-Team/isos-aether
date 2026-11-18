"""Native System 1 fixture through the verified isos_node_v1 host."""

from __future__ import annotations

import hashlib
import shutil
import subprocess
from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest
from ei.core.system.isolated import IsolatedNode
from ei.core.system.node_runtime import NativeNode


def test_native_system1_uses_real_v1_abi_and_stops(tmp_path: Path) -> None:
  if shutil.which("cc") is None:
    pytest.skip("C compiler required for native ABI fixture")
  now = datetime(2026, 1, 1, tzinfo=UTC)
  source_time = now.isoformat()
  expires_at = (now + timedelta(seconds=2)).isoformat()
  motion = (
    '{"port":"output","payload":{"source_id":"fixture",'
    f'"source_time":"{source_time}","sequence":1,"frame_id":"base_link",'
    '"proposal_id":"p1","target_id":"arm-1","resource_id":"joint-1",'
    f'"horizon_ms":100,"delta_rad":[0.1],"expires_at":"{expires_at}",'
    '"model_version":"native-fixture-v1"}}'
  )
  code = (
    '#include "isos_node_v1.h"\n'
    "#include <stdlib.h>\n#include <string.h>\n"
    "static const isos_node_host_v1 *host;\n"
    "static int32_t create(const uint8_t *b,size_t n,const isos_node_host_v1 *h,isos_node_handle **out) {(void)b;(void)n;host=h;*out=(isos_node_handle *)malloc(1);return *out?0:-1;}\n"
    "static int32_t start(isos_node_handle *n) {(void)n;return 0;}\n"
    f'static const char output[] = "{motion.replace(chr(34), chr(92) + chr(34))}";\n'
    "static int32_t input(isos_node_handle *n,const uint8_t *b,size_t z) {(void)n;(void)b;(void)z;return host->emit(host->context,(const uint8_t *)output,strlen(output));}\n"
    "static int32_t health(isos_node_handle *n) {(void)n;return 1;}\n"
    "static int32_t stop(isos_node_handle *n) {(void)n;return 0;}\n"
    "static void destroy(isos_node_handle *n) {free(n);}\n"
    "static const isos_node_api_v1 api={1,sizeof(isos_node_api_v1),create,start,input,health,stop,destroy};\n"
    "const isos_node_api_v1 *isos_node_v1(void) {return &api;}\n"
  )
  (tmp_path / "node.c").write_text(code)
  library = tmp_path / "system1.so"
  header = Path(__file__).parents[3] / "contract/node"
  subprocess.run(
    [
      "cc",
      "-shared",
      "-fPIC",
      "-I",
      str(header),
      "-o",
      str(library),
      str(tmp_path / "node.c"),
    ],
    check=True,
  )
  digest = hashlib.sha256(library.read_bytes()).hexdigest()
  emitted: list[dict] = []
  native = NativeNode(
    str(tmp_path),
    {"library": "system1.so"},
    {"run_id": "run-1", "revision": "r1"},
    emitted.append,
  )
  assert native.ready()

  def provider(resource_id: str, value: dict) -> dict:
    assert resource_id == "system1-native-fixture"
    native.input(value)
    return emitted[-1]["payload"]

  node = IsolatedNode(
    "ei.cognition.policy.system1_native@v0.2",
    {"model_resource_id": "system1-native-fixture"},
    "run-1",
    provider=provider,
    verified_model=("system1-native-fixture", str(library), digest),
    now=lambda: now,
  )
  goal = {
    "source_id": "fixture",
    "source_time": source_time,
    "sequence": 1,
    "frame_id": "base_link",
    "goal_id": "g1",
    "target_id": "arm-1",
    "confidence": 0.9,
    "expires_at": expires_at,
    "model_version": "fixture-v1",
  }
  assert node.input("input", "grounded_goal@v0.2", goal)[0]["payload"]["delta_rad"] == [
    0.1
  ]
  assert node.stop()
  native.stop()
  with pytest.raises(ValueError, match="stopped"):
    node.input("input", "grounded_goal@v0.2", goal)
