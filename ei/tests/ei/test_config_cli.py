"""The IsOS adapter validates and renders without starting EI."""

from __future__ import annotations

import io
import json
import sys
import tomllib
from pathlib import Path
from typing import Any, cast

import grpc
import pytest
from grpc_health.v1 import health, health_pb2, health_pb2_grpc

from ei.config_cli import main
from ei.core.link import EosClient
from ei.lib.config import GatewayConfig, load_mode_config, resolve_etc_root


def test_ei_etc_and_explicit_root(monkeypatch, tmp_path: Path) -> None:
  ei = tmp_path / "ei"
  explicit = tmp_path / "explicit"
  for root in (ei, explicit):
    root.mkdir()
  monkeypatch.setenv("EI_ETC", str(ei))
  assert resolve_etc_root(None) == ei
  assert resolve_etc_root(str(explicit)) == explicit


def _invoke(monkeypatch, capsys, operation: str, payload: dict | None = None):
  monkeypatch.setattr(sys, "stdin", io.StringIO(json.dumps(payload or {})))
  status = main([operation, "--json"])
  return status, json.loads(capsys.readouterr().out)


def test_adapter_exposes_versioned_profile_and_defaults(monkeypatch, capsys) -> None:
  _, described = _invoke(monkeypatch, capsys, "describe")
  assert described["data"]["component"] == "ei"
  assert described["data"]["version"]
  assert described["data"]["requires"] == [
    {"name": "island.eos.v1.GatewayService", "version": "1.0.0"},
    {"name": "island.eos.v2.CapabilityService", "version": "2.0.0"},
    {"name": "island.eos.v1.SkillService", "version": "1.0.0"},
  ]
  _, profiles = _invoke(monkeypatch, capsys, "profiles")
  assert profiles["data"][0]["id"] == "ei.agent"
  assert profiles["data"][0]["inputs"][0]["name"] == "eos"
  assert profiles["data"][0]["inputs"][0]["contracts"] == described["data"]["requires"]
  fields = profiles["data"][0]["settings_fields"]
  assert [field["name"] for field in fields] == [
    "server.host",
    "server.port",
    "server.status_frequency_hz",
    "gateway.transport",
    "gateway.host",
    "gateway.grpc_port",
    "gateway.path",
  ]
  assert fields[1]["min"] == 1
  assert fields[1]["max"] == 65535
  assert fields[3]["options"] == ["uds", "tcp"]
  assert fields[6]["type"] == "absolute_path"
  _, defaults = _invoke(monkeypatch, capsys, "defaults")
  assert defaults["data"]["server"]["port"] == 16060
  assert defaults["data"]["gateway"]["host"] == "127.0.0.1"
  assert defaults["data"]["gateway"]["grpc_port"] == 16200


def test_adapter_round_trips_nested_fields_and_rejects_unknown_settings(
  monkeypatch, capsys
) -> None:
  values = {
    "server": {"host": "127.0.0.1", "port": 16101, "status_frequency_hz": 2.5},
    "gateway": {"transport": "tcp", "host": "localhost", "grpc_port": 16201},
  }
  payload = {"schema_version": 1, "values": values}
  status, validated = _invoke(monkeypatch, capsys, "validate", payload)
  assert status == 0
  for section, fields in values.items():
    for name, expected in fields.items():
      assert validated["data"]["values"][section][name] == expected

  status, rendered = _invoke(monkeypatch, capsys, "render", payload)
  assert status == 0
  server, _, gateway = rendered["data"]["files"]
  assert tomllib.loads(server["content"])["server"]["port"] == 16101
  assert tomllib.loads(server["content"])["server"]["status_frequency_hz"] == 2.5
  assert (
    tomllib.loads(gateway["content"])["modes"]["isos"]["gateway"]["grpc_port"] == 16201
  )

  values["server"]["unused"] = True
  status, rejected = _invoke(monkeypatch, capsys, "validate", payload)
  assert status == 2
  assert rejected["diagnostics"][0]["path"] == "values.server.unused"


def test_adapter_renders_uds_overlay_that_runtime_loader_accepts(
  monkeypatch, capsys, tmp_path
) -> None:
  socket_path = tmp_path / "eos.sock"
  payload = {
    "schema_version": 1,
    "values": {"gateway": {"transport": "uds", "path": str(socket_path)}},
  }
  status, response = _invoke(monkeypatch, capsys, "render", payload)
  assert status == 0
  root = tmp_path / "managed"
  for item in response["data"]["files"]:
    parsed = tomllib.loads(item["content"])
    assert isinstance(parsed, dict)
    target = root / item["path"]
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(item["content"], encoding="utf-8")
  preset = load_mode_config(
    root / "ei", defaults=(Path(__file__).parents[3] / "share/ei/defaults/ei").resolve()
  )
  assert preset.gateway.endpoint == f"unix://{socket_path}"


def test_adapter_reports_field_address_for_unsafe_uds_path(monkeypatch, capsys) -> None:
  status, response = _invoke(
    monkeypatch,
    capsys,
    "validate",
    {
      "schema_version": 1,
      "values": {"gateway": {"transport": "uds", "path": "../eos.sock"}},
    },
  )
  assert status == 2
  assert response["ok"] is False
  assert response["diagnostics"][0]["path"] == "values.gateway"


def _setup_request(op: str, endpoints: dict | None = None) -> dict:
  return {
    "schema_version": 2,
    "op": op,
    "core": "ei",
    "revision": "sha256:test-revision",
    "graph": {"version": "0.2.0", "robot": {}, "cores": {"ei": {"layers": {}}}},
    "endpoints": endpoints or {},
    "packages": [],
  }


def test_setup_identity_reports_product_and_contract_versions(
  monkeypatch, capsys
) -> None:
  request = {"schema_version": 2, "op": "identity", "core": "ei"}
  status, response = _invoke(monkeypatch, capsys, "setup", request)
  _, described = _invoke(monkeypatch, capsys, "describe")
  assert status == 0
  assert described["data"]["version"] == "0.2.1"
  assert response == {
    "schema_version": 2,
    "ok": True,
    "diagnostics": [],
    "files": [],
    "identity": {
      "core": "ei",
      "product_version": described["data"]["version"],
      "graph_version": "0.2.0",
      "node_type_version": "0.2",
      "supported_graph_versions": ["0.2.0"],
      "supported_node_type_versions": ["0.2"],
      "supported_node_types": [
        "ei.cognition.intent.forwarder@v0.2",
        "ei.cognition.state.arm_status@v0.2",
        "ei.governance.monitor.arm_status@v0.2",
        "ei.governance.safety.gate@v0.2",
        "ei.io.input.arm_status@v0.2",
        "ei.io.input.trigger@v0.2",
      ],
      "features": [
        "setup-v2",
        "catalog",
        "graph-0.2-ei-projection",
        "health-only-owner-v1",
        "activation-owner-v1",
        "arm-monitor-owner-v1",
      ],
      "setup_schema_version": 2,
    },
  }


@pytest.mark.parametrize("extra", [{"graph": {}}, {"revision": "stale"}])
def test_setup_identity_rejects_graph_fields(monkeypatch, capsys, extra) -> None:
  status, response = _invoke(
    monkeypatch,
    capsys,
    "setup",
    {"schema_version": 2, "op": "identity", "core": "ei", **extra},
  )
  assert status == 2
  assert response["ok"] is False
  assert response["files"] == []


def test_setup_catalog_is_available_without_runtime_gateway(
  monkeypatch, capsys
) -> None:
  status, response = _invoke(monkeypatch, capsys, "setup", _setup_request("catalog"))
  assert status == 0
  assert response["schema_version"] == 2
  assert response["catalog"]["profiles"][0]["id"] == "ei.agent"
  assert response["files"] == []


def test_setup_static_options_and_empty_gateway_default(monkeypatch, capsys) -> None:
  status, preview = _invoke(monkeypatch, capsys, "setup", _setup_request("options"))
  assert status == 0, preview
  assert preview["catalog"]["setting_options"] == []
  status, rendered = _invoke(monkeypatch, capsys, "setup", _setup_request("render"))
  assert status == 0, rendered
  assert len(rendered["files"]) == 3


def test_setup_renders_core_overlay_without_side_effects(monkeypatch, capsys) -> None:
  endpoints = {
    "ei_northbound": {"host": "127.0.0.1", "port": 16061},
    "eos_gateway": {"transport": "tcp", "host": "127.0.0.1", "port": 16022},
  }
  status, response = _invoke(
    monkeypatch, capsys, "setup", _setup_request("render", endpoints)
  )
  assert status == 0
  assert response["diagnostics"] == []
  files = response["files"]
  assert [file["path"] for file in files] == [
    "ei/ei.toml",
    "ei/modes.toml",
    "ei/profiles/isos.toml",
  ]
  assert tomllib.loads(files[0]["content"])["server"]["port"] == 16061
  assert (
    tomllib.loads(files[2]["content"])["modes"]["isos"]["gateway"]["grpc_port"] == 16022
  )


def test_setup_bounds_request_and_preserves_standalone_contract(
  monkeypatch, capsys
) -> None:
  request = _setup_request(
    "validate", {"eos_gateway": {"transport": "uds", "path": "../bad.sock"}}
  )
  status, response = _invoke(monkeypatch, capsys, "setup", request)
  assert status == 2
  assert response["files"] == []
  assert response["diagnostics"][0]["path"].startswith("endpoints.")
  status, legacy = _invoke(
    monkeypatch, capsys, "validate", {"schema_version": 1, "values": {}}
  )
  assert status == 0
  assert legacy["schema_version"] == 1


@pytest.mark.parametrize(
  "setup_payload",
  [
    {"schema_version": 1, "op": "identity", "core": "ei"},
    {
      **_setup_request("catalog"),
      "graph": {"version": "2.0.0", "cores": {"ei": {"layers": {}}}},
    },
  ],
)
def test_setup_rejects_pre_release_contract(monkeypatch, capsys, setup_payload) -> None:
  status, response = _invoke(monkeypatch, capsys, "setup", setup_payload)
  assert status == 2
  assert response["schema_version"] == 2
  assert response["files"] == []
  if setup_payload.get("graph", {}).get("version") == "2.0.0":
    assert response["diagnostics"][0]["path"] == "/graph/version"
    assert response["diagnostics"][0]["code"] == "unsupported_graph_version"


@pytest.mark.asyncio
async def test_eos_client_uses_direct_grpc_uds_target(tmp_path) -> None:
  socket_path = tmp_path / "eos.sock"
  server = grpc.aio.server()
  health_pb2_grpc.add_HealthServicer_to_server(health.HealthServicer(), server)
  assert server.add_insecure_port(f"unix://{socket_path}") == 1
  await server.start()
  client = EosClient(GatewayConfig(transport="uds", path=str(socket_path)))
  try:
    assert client.endpoint == f"unix://{socket_path}"
    async with grpc.aio.insecure_channel(client.endpoint) as channel:
      response = await cast(Any, health_pb2_grpc.HealthStub(channel)).Check(
        health_pb2.HealthCheckRequest(), timeout=1
      )
      assert response.status == health_pb2.HealthCheckResponse.SERVING
  finally:
    await server.stop(0)
