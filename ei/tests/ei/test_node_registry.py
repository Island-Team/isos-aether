"""Boundary tests for package EI nodes and their owner lifecycle."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
from pathlib import Path

import pytest
from ei.core.system.node_registry import NodeRegistryError, load_claims, load_registry
from ei.core.system.node_runtime import ExternalNode, NativeNode


@pytest.fixture
def native_package(tmp_path: Path) -> tuple[Path, dict]:
  if shutil.which("cc") is None:
    pytest.skip("C compiler required for native ABI fixture")
  source = tmp_path / "node.c"
  source.write_text(
    '#include "isos_node_v1.h"\n'
    "#include <stdlib.h>\n"
    "static const isos_node_host_v1 *host;\n"
    "static int32_t create(const uint8_t *b, size_t n, const isos_node_host_v1 *h, isos_node_handle **out) {(void)b;(void)n;host=h;*out=(isos_node_handle *)malloc(1);return 0;}\n"
    "static int32_t start(isos_node_handle *n) {(void)n;return 0;}\n"
    "static int32_t input(isos_node_handle *n,const uint8_t *b,size_t z) {(void)n;return host->emit(host->context,b,z);}\n"
    "static int32_t health(isos_node_handle *n) {(void)n;return 1;}\n"
    "static int32_t stop(isos_node_handle *n) {(void)n;return 0;}\n"
    "static void destroy(isos_node_handle *n) {free(n);}\n"
    "static const isos_node_api_v1 api={1,sizeof(isos_node_api_v1),create,start,input,health,stop,destroy};\n"
    "const isos_node_api_v1 *isos_node_v1(void) {return &api;}\n"
  )
  library = tmp_path / "node.so"
  header = Path(__file__).parents[3] / "contract/node"
  subprocess.run(
    ["cc", "-shared", "-fPIC", "-I", str(header), "-o", str(library), str(source)],
    check=True,
  )
  node = {
    "id": "ei.cognition.custom.echo@v0.2",
    "core": "ei",
    "layer": "cognition",
    "group": "custom",
    "implementation": {
      "kind": "native_plugin",
      "abi": "isos_node_v1",
      "library": "node.so",
    },
    "inputs": [{"name": "data"}],
    "outputs": [{"name": "data"}],
    "settings_fields": [],
  }
  release = {
    "id": "sample/ei",
    "version": "1.0.0",
    "node_contributions": {
      "node_types": [node],
      "libraries": [
        {"path": "node.so", "sha256": hashlib.sha256(library.read_bytes()).hexdigest()}
      ],
    },
  }
  (tmp_path / "release.json").write_text(json.dumps(release))
  return tmp_path, node


def test_native_package_registry_and_run(native_package: tuple[Path, dict]) -> None:
  root, node = native_package
  registry = load_registry([{"id": "sample/ei", "version": "1.0.0", "path": str(root)}])
  spec = registry[node["id"]]
  outputs: list[dict] = []
  running = NativeNode(
    str(root),
    spec["implementation"],
    {"run_id": "run", "revision": "r"},
    outputs.append,
  )
  assert running.ready()
  running.input({"kind": "sample", "value": 1})
  assert outputs == [{"kind": "sample", "value": 1}]
  running.stop()


def test_changed_library_and_duplicate_type_rejected(
  native_package: tuple[Path, dict],
) -> None:
  root, _ = native_package
  ref = {"id": "sample/ei", "version": "1.0.0", "path": str(root)}
  with pytest.raises(NodeRegistryError, match="duplicate"):
    load_registry([ref, ref])
  (root / "node.so").write_bytes(b"changed")
  with pytest.raises(NodeRegistryError, match="digest changed"):
    load_registry([ref])


def test_route_can_join_nodes_from_distinct_verified_packages(
  native_package: tuple[Path, dict], tmp_path: Path
) -> None:
  first_root, first = native_package
  second_root = tmp_path / "second"
  second_root.mkdir()
  shutil.copy(first_root / "node.so", second_root / "node.so")
  second = {**first, "id": "ei.cognition.custom.sink@v0.2"}
  second_release = {
    "id": "sample/sink",
    "version": "1.0.0",
    "node_contributions": {
      "node_types": [second],
      "libraries": [
        {
          "path": "node.so",
          "sha256": hashlib.sha256((second_root / "node.so").read_bytes()).hexdigest(),
        }
      ],
    },
  }
  (second_root / "release.json").write_text(json.dumps(second_release))
  first_release = json.loads((first_root / "release.json").read_text())
  route = {
    "source_type": first["id"],
    "source_port": "data",
    "target_type": second["id"],
    "target_port": "data",
    "transport": "in_process",
    "delivery_mode": "event",
    "contract": {"version": "1"},
  }
  first_release["node_contributions"]["port_routes"] = [route]
  (first_root / "release.json").write_text(json.dumps(first_release))
  refs = [
    {"id": "sample/ei", "version": "1.0.0", "path": str(first_root)},
    {"id": "sample/sink", "version": "1.0.0", "path": str(second_root)},
  ]
  registry = load_registry(refs)
  assert load_claims(refs, registry)[0] == [route]
  with pytest.raises(NodeRegistryError, match="route requires EI or EOS endpoint"):
    load_claims(refs[:1], load_registry(refs[:1]))


def test_external_socket_lifecycle(tmp_path: Path) -> None:
  program = tmp_path / "node"
  program.write_text(
    "#!/usr/bin/env python3\n"
    "import json, os, socket\n"
    "s=socket.socket(socket.AF_UNIX);s.bind(os.environ['ISOS_NODE_SOCKET']);s.listen()\n"
    "while True:\n"
    " c,_=s.accept()\n"
    " with c:\n"
    "  req=json.loads(c.makefile('rb').readline())\n"
    "  op=req['op']\n"
    "  reply={'version':1,'ok':True,'ready':True,'quiesced':True,'outputs':[req.get('data',{})]}\n"
    "  c.sendall((json.dumps(reply)+'\\n').encode())\n"
    " if op=='stop': break\n"
  )
  program.chmod(0o700)
  outputs: list[dict] = []
  running = ExternalNode(
    str(tmp_path), "node", {"run_id": "run", "revision": "r"}, outputs.append
  )
  assert running.ready()
  running.input({"value": 2})
  assert outputs == [{"value": 2}]
  running.stop()


def test_payload_root_and_local_route(
  native_package: tuple[Path, dict], tmp_path: Path
) -> None:
  from ei.core.system.owner_service import _Handler, _OwnerServer

  store, spec = native_package
  payload = tmp_path / "payload"
  payload.mkdir()
  shutil.copy(store / "node.so", payload / "node.so")
  (store / "node.so").unlink()
  registry = load_registry(
    [
      {
        "id": "sample/ei",
        "version": "1.0.0",
        "path": str(store),
        "payload_root": str(payload),
      }
    ]
  )
  assert registry[spec["id"]]["_package_root"] == str(payload)

  class Echo:
    def __init__(self) -> None:
      self.received: list[dict] = []

    def input(self, value: dict) -> None:
      self.received.append(value)

  server = _OwnerServer(str(tmp_path / "owner.sock"), _Handler)
  try:
    target = Echo()
    server.custom_nodes = {"ei/cognition/custom/sink": target}  # type: ignore[assignment]
    server.node_outputs = {"ei/cognition/custom/source": []}
    server.node_edges = {
      "ei/cognition/custom/source": [("ei/cognition/custom/sink", "input", "output")]
    }
    server.emit_node(
      "ei/cognition/custom/source", {"port": "output", "data": {"value": 7}}
    )
    assert target.received == [{"port": "input", "payload": {"value": 7}}]
  finally:
    server.custom_nodes.clear()
    server.server_close()


def test_custom_graph_requires_exact_package_pin(
  native_package: tuple[Path, dict],
) -> None:
  from ei.config_cli import SetupDiagnostic, _validate_setup_request

  root, spec = native_package
  request = {
    "schema_version": 3,
    "op": "validate",
    "core": "ei",
    "revision": "revision",
    "packages": [{"id": "sample/ei", "version": "1.0.0", "path": str(root)}],
    "endpoints": {},
    "graph": {
      "version": "0.2.1",
      "cores": {
        "ei": {
          "layers": {
            "cognition": {
              "groups": {
                "custom": {
                  "nodes": {
                    "echo": {
                      "id": "echo",
                      "node_type": spec["id"],
                      "package": {"id": "sample/ei", "version": "1.0.0"},
                      "settings": {},
                      "inputs": {},
                    }
                  }
                }
              }
            }
          }
        }
      },
    },
  }
  _validate_setup_request(request)
  request["graph"]["cores"]["ei"]["layers"]["cognition"]["groups"]["custom"]["nodes"][
    "echo"
  ]["package"]["version"] = "2.0.0"
  with pytest.raises(SetupDiagnostic, match="package pin"):
    _validate_setup_request(request)


def test_owner_cross_core_input_checks_exact_source(
  native_package: tuple[Path, dict], tmp_path: Path
) -> None:
  from ei.core.system.owner_service import _Handler, _OwnerServer

  root, spec = native_package
  server = _OwnerServer(str(tmp_path / "owner.sock"), _Handler)
  server.run_id = "run"
  server.revision = "r"
  graph = {
    "cores": {
      "ei": {
        "layers": {
          "cognition": {
            "groups": {
              "custom": {
                "nodes": {
                  "echo": {
                    "node_type": spec["id"],
                    "inputs": {
                      "data": {"node": "eos/skill/custom/source", "output": "data"}
                    },
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  server.start_nodes(
    graph, [{"id": "sample/ei", "version": "1.0.0", "path": str(root)}], "run", "r"
  )
  handler = object.__new__(_Handler)
  handler.server = server
  request = {
    "schema_version": 1,
    "core": "ei",
    "op": "input",
    "run_id": "run",
    "revision": "r",
    "node": "ei/cognition/custom/echo",
    "port": "data",
    "payload": {"value": 3},
    "source_node": "eos/skill/custom/source",
    "source_port": "data",
  }
  try:
    status, reply = handler._input(request)
    assert status == 200 and reply["outputs"][0]["output"] == {
      "port": "data",
      "payload": {"value": 3},
    }
    assert handler._input({**request, "source_node": "eos/other"})[0] == 422
    assert handler._input({**request, "revision": "stale"})[0] == 409
  finally:
    server.server_close()


def test_owner_cross_core_output_is_fenced(tmp_path: Path, monkeypatch) -> None:
  from ei.core.system import owner_service
  from ei.core.system.owner_service import _Handler, _OwnerServer

  server = _OwnerServer(str(tmp_path / "owner.sock"), _Handler)
  server.run_id = "run"
  server.revision = "r"
  server.eos_owner_path = str(tmp_path / "eos.sock")
  server.node_outputs["ei/cognition/custom/source"] = []
  server.cross_outputs["ei/cognition/custom/source"] = [
    ("eos/skill/custom/target", "input", "output")
  ]
  calls: list[dict] = []

  def reply(_path: str, route: str, request: dict) -> tuple[int, dict]:
    assert route == "/v1/input"
    calls.append(request)
    return 200, {"ok": True, "run_id": "run", "revision": "r"}

  monkeypatch.setattr(owner_service, "_post_owner", reply)
  try:
    server.emit_node(
      "ei/cognition/custom/source", {"port": "output", "payload": {"value": 4}}
    )
    assert calls == [
      {
        "schema_version": 1,
        "core": "eos",
        "op": "input",
        "run_id": "run",
        "revision": "r",
        "node": "eos/skill/custom/target",
        "port": "input",
        "source_node": "ei/cognition/custom/source",
        "source_port": "output",
        "payload": {"value": 4},
      }
    ]
  finally:
    server.server_close()
