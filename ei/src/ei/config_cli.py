"""Side-effect-free IsOS configuration adapter for EI."""

from __future__ import annotations

import argparse
import json
import sys
from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field, ValidationError

from ei.lib.config import GatewayConfig, ServerConfig
from ei.reader_features import SETUP_FEATURES, product_version

SCHEMA_VERSION = 1
SETUP_SCHEMA_VERSION = 2
MAX_SETUP_INPUT_BYTES = 1024 * 1024
PROFILE_VERSION = "1.0.0"
SUPPORTED_GRAPH_VERSIONS = ("0.2.0",)
SUPPORTED_NODE_TYPE_VERSIONS = ("0.2",)
REQUIRED_CONTRACTS = [
  {"name": "island.eos.v1.GatewayService", "version": "1.0.0"},
  {"name": "island.eos.v2.CapabilityService", "version": "2.0.0"},
  {"name": "island.eos.v1.SkillService", "version": "1.0.0"},
]


class ManagedServer(ServerConfig):
  """Strictly validate the IsOS-editable EI listener fields."""

  model_config = ConfigDict(extra="forbid")


class ManagedValues(BaseModel):
  """Values IsOS may edit without owning EI's full TOML schema."""

  model_config = ConfigDict(extra="forbid")

  mode: Literal["isos"] = "isos"
  server: ManagedServer = Field(default_factory=ManagedServer)
  gateway: GatewayConfig = Field(
    default_factory=lambda: GatewayConfig(transport="tcp", host="127.0.0.1")
  )


class SetupDiagnostic(ValueError):
  """Carry a graph JSON pointer and stable error code to the setup envelope."""

  def __init__(self, path: str, code: str, message: str) -> None:
    super().__init__(message)
    self.path = path
    self.code = code


def _pointer_segment(value: str) -> str:
  return value.replace("~", "~0").replace("/", "~1")


def _validate_ei_graph(graph: Any, *, validate_edges: bool = True) -> None:
  """Validate the current IsOS EI graph projection and its local routes."""
  if not isinstance(graph, dict) or graph.get("version") != "0.2.0":
    raise SetupDiagnostic(
      "/graph/version", "unsupported_graph_version", "EI requires graph 0.2.0"
    )
  _validate_ei_projection(graph, validate_edges=validate_edges)


EI_NODE_TYPES = {
  "ei.io.input.trigger@v0.2": ("io", "input"),
  "ei.cognition.intent.forwarder@v0.2": ("cognition", "intent"),
  "ei.governance.safety.gate@v0.2": ("governance", "safety"),
  "ei.io.input.arm_status@v0.2": ("io", "input"),
  "ei.cognition.state.arm_status@v0.2": ("cognition", "state"),
  "ei.governance.monitor.arm_status@v0.2": ("governance", "monitor"),
}


def _activation_port(
  transport: str, owner: str, contract_id: str, *, required: bool
) -> dict[str, Any]:
  return {
    "name": "activation",
    "description": "General button activation event.",
    "data_type": "button_activation@v0.2",
    "data_class": "event",
    "transport": transport,
    "delivery_mode": "event",
    "cardinality": "one",
    "required": required,
    "contract": {"owner": owner, "id": contract_id, "version": "1"},
  }


def _activation_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  first = layer == "io"
  last = layer == "governance"
  stage = {
    "id": node_type,
    "core": "ei",
    "layer": layer,
    "group": group,
    "description": "Pass one activation through the EI input, intent, and safety path.",
    "kind": "software",
    "lifecycle": "internal",
    "capabilities": ["bridge"] if first or last else [],
    "settings_fields": [],
    "service_inputs": {},
    "inputs": [
      _activation_port(
        "unix_socket" if first else "in_process",
        "isos" if first else "ei",
        "button-activation" if first else "activation-local",
        required=True,
      )
    ],
    "outputs": [
      _activation_port(
        "unix_socket" if last else "in_process",
        "ei",
        "activation-to-eos" if last else "activation-local",
        required=False,
      )
    ],
  }
  if first or last:
    stage["bridge"] = {
      "adapter": "unix_socket",
      "inputs": ["activation"] if first else [],
      "outputs": ["activation"] if last else [],
    }
  return stage


def _monitor_port(
  transport: str, owner: str, contract_id: str, *, required: bool
) -> dict[str, Any]:
  return {
    "name": "observation",
    "description": "Read-only six-joint arm state.",
    "data_type": "arm_monitor_state@v0.2",
    "data_class": "observation",
    "transport": transport,
    "delivery_mode": "event",
    "cardinality": "one",
    "required": required,
    "contract": {"owner": owner, "id": contract_id, "version": "1"},
  }


def _monitor_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  first = layer == "io"
  last = layer == "governance"
  stage = {
    "id": node_type,
    "core": "ei",
    "layer": layer,
    "group": group,
    "description": "Pass a fenced read-only mock-arm observation toward IsOS.",
    "kind": "software",
    "lifecycle": "internal",
    "capabilities": ["bridge"] if first or last else [],
    "settings_fields": (
      [
        {
          "name": "forward_rate_hz",
          "data_type": "number",
          "label": "Monitor forward rate (Hz)",
          "description": "Maximum EOS arm-state reads per second.",
          "default": 20,
          "min": 1,
          "max": 20,
          "required": False,
        }
      ]
      if last
      else []
    ),
    "service_inputs": {},
    "inputs": [
      _monitor_port(
        "unix_socket" if first else "in_process",
        "eos" if first else "ei",
        "arm-monitor-to-ei" if first else "arm-monitor-local",
        required=True,
      )
    ],
    "outputs": [
      _monitor_port(
        "unix_socket" if last else "in_process",
        "ei",
        "arm-monitor-to-isos" if last else "arm-monitor-local",
        required=False,
      )
    ],
  }
  if first or last:
    stage["bridge"] = {
      "adapter": "unix_socket",
      "inputs": ["observation"] if first else [],
      "outputs": ["observation"] if last else [],
    }
  return stage


def _validate_ei_projection(graph: dict[str, Any], *, validate_edges: bool) -> None:
  """Admit the three activation stages and their declared input links."""
  cores = graph.get("cores")
  if not isinstance(cores, dict) or set(cores) != {"ei"}:
    raise SetupDiagnostic(
      "/graph/cores", "invalid_graph", "EI requires its isolated ei core"
    )
  core = cores["ei"]
  layers = core.get("layers") if isinstance(core, dict) else None
  if not isinstance(layers, dict):
    raise SetupDiagnostic(
      "/graph/cores/ei/layers", "invalid_graph", "EI layers must be an object"
    )
  for layer_id, layer in layers.items():
    layer_path = f"/graph/cores/ei/layers/{_pointer_segment(layer_id)}"
    groups = layer.get("groups") if isinstance(layer, dict) else None
    if not isinstance(groups, dict):
      raise SetupDiagnostic(layer_path, "invalid_graph", "EI layer requires groups")
    for group_id, group in groups.items():
      group_path = f"{layer_path}/groups/{_pointer_segment(group_id)}"
      nodes = group.get("nodes") if isinstance(group, dict) else None
      if not isinstance(nodes, dict):
        raise SetupDiagnostic(group_path, "invalid_graph", "EI group requires nodes")
      for node_id, node in nodes.items():
        node_path = f"{group_path}/nodes/{_pointer_segment(node_id)}"
        node_type = node.get("node_type") if isinstance(node, dict) else None
        placement = EI_NODE_TYPES.get(node_type) if isinstance(node_type, str) else None
        if placement is None:
          raise SetupDiagnostic(
            f"{node_path}/node_type",
            "unsupported_node_type_version",
            f"unsupported EI type {node_type!r}",
          )
        if placement != (layer_id, group_id):
          raise SetupDiagnostic(
            f"{node_path}/node_type",
            "invalid_node_placement",
            f"{node_type} belongs at ei/{placement[0]}/{placement[1]}",
          )
        settings = node.get("settings", {})
        monitor_egress = node_type == "ei.governance.monitor.arm_status@v0.2"
        valid_settings = isinstance(settings, dict) and (
          not settings
          if not monitor_egress
          else set(settings) <= {"forward_rate_hz"}
          and (
            "forward_rate_hz" not in settings
            or type(settings["forward_rate_hz"]) in (int, float)
            and 1 <= settings["forward_rate_hz"] <= 20
          )
        )
        if (
          set(node) - {"id", "label", "description", "node_type", "settings", "inputs"}
          or not valid_settings
        ):
          raise SetupDiagnostic(
            node_path, "invalid_node_settings", "EI node settings are invalid"
          )
        inputs = node.get("inputs", {})
        port_name = (
          "observation"
          if isinstance(node_type, str) and node_type.endswith(".arm_status@v0.2")
          else "activation"
        )
        if not isinstance(inputs, dict) or set(inputs) - {port_name}:
          raise SetupDiagnostic(
            f"{node_path}/inputs",
            "unsupported_node_port",
            f"EI stage accepts only its {port_name} input",
          )
        if port_name in inputs:
          link = inputs[port_name]
          if (
            not isinstance(link, dict)
            or not isinstance(link.get("node"), str)
            or link.get("output") != port_name
            or set(link) - {"node", "output", "priority", "frequency_hz"}
          ):
            raise SetupDiagnostic(
              f"{node_path}/inputs/{port_name}",
              "invalid_node_input",
              f"{port_name} requires a source node and {port_name} output",
            )


def _reply(
  data: Any = None, diagnostics: list[dict[str, str]] | None = None
) -> dict[str, Any]:
  issues = diagnostics or []
  return {
    "schema_version": SCHEMA_VERSION,
    "ok": not issues,
    "data": data,
    "diagnostics": issues,
  }


def _read_values() -> ManagedValues:
  try:
    payload = json.load(sys.stdin)
  except (ValueError, UnicodeDecodeError) as exc:
    raise ValueError(f"invalid JSON input: {exc}") from exc
  if not isinstance(payload, dict):
    raise TypeError("input must be a JSON object")
  if payload.get("schema_version") != SCHEMA_VERSION:
    raise ValueError(f"schema_version must be {SCHEMA_VERSION}")
  return ManagedValues.model_validate(payload.get("values", {}))


def _read_setup_request() -> dict[str, Any]:
  """Read one bounded graph setup request without touching runtime state."""
  raw = (
    sys.stdin.buffer.read(MAX_SETUP_INPUT_BYTES + 1)
    if hasattr(sys.stdin, "buffer")
    else sys.stdin.read(MAX_SETUP_INPUT_BYTES + 1).encode("utf-8")
  )
  if len(raw) > MAX_SETUP_INPUT_BYTES:
    raise ValueError("setup request exceeds 1 MiB")
  try:
    request = json.loads(raw)
  except (ValueError, UnicodeDecodeError) as exc:
    raise ValueError(f"invalid JSON input: {exc}") from exc
  if not isinstance(request, dict):
    raise TypeError("setup request must be an object")
  return _validate_setup_request(request)


def _validate_setup_request(request: dict[str, Any]) -> dict[str, Any]:
  """Validate one decoded setup request for either supported transport."""
  if request.get("op") == "identity":
    if set(request) != {"schema_version", "op", "core"}:
      raise ValueError("identity request fields must be schema_version, op, core")
    if request["schema_version"] != SETUP_SCHEMA_VERSION:
      raise ValueError(f"setup schema_version must be {SETUP_SCHEMA_VERSION}")
    if request["core"] != "ei":
      raise ValueError("setup core must be ei")
    return request
  if set(request) != {
    "schema_version",
    "op",
    "core",
    "revision",
    "graph",
    "endpoints",
    "packages",
  }:
    raise ValueError(
      "setup request fields must be schema_version, op, core, revision, graph, endpoints, packages"
    )
  if request["schema_version"] != SETUP_SCHEMA_VERSION:
    raise ValueError(f"setup schema_version must be {SETUP_SCHEMA_VERSION}")
  if request["op"] not in {"catalog", "options", "validate", "render"}:
    raise ValueError("setup op must be catalog, options, validate, or render")
  if request["core"] != "ei":
    raise ValueError("setup core must be ei")
  if (
    not isinstance(request["revision"], str) or not 1 <= len(request["revision"]) <= 128
  ):
    raise ValueError("setup revision must be a 1-128 character string")
  graph = request["graph"]
  _validate_ei_graph(graph, validate_edges=request["op"] not in {"catalog", "options"})
  if not isinstance(request["endpoints"], dict):
    raise TypeError("setup endpoints must be an object")
  if not isinstance(request["packages"], list) or len(request["packages"]) > 256:
    raise ValueError("setup packages must be an array of at most 256 entries")
  return request


def _setup_values(request: dict[str, Any]) -> ManagedValues:
  """Project EI settings from IsOS-resolved endpoints and its graph core."""
  endpoints = request["endpoints"]
  northbound = endpoints.get("ei_northbound", {})
  gateway = endpoints.get(
    "eos_gateway", {"transport": "tcp", "host": "127.0.0.1", "port": 16200}
  )
  if not isinstance(northbound, dict) or not isinstance(gateway, dict):
    raise TypeError("ei_northbound and eos_gateway endpoints must be objects")
  if gateway.get("transport", "tcp") not in {"tcp", "uds"}:
    raise ValueError("endpoints.eos_gateway.transport must be tcp or uds")
  server = {
    "host": northbound.get("host", "127.0.0.1"),
    "port": northbound.get("port", 16060),
  }
  if gateway.get("transport", "tcp") == "uds":
    gateway_values = {"transport": "uds", "path": gateway.get("path")}
  else:
    gateway_values = {
      "transport": "tcp",
      "host": gateway.get("host", "127.0.0.1"),
      "grpc_port": gateway.get("port", 16200),
    }
  return ManagedValues.model_validate({"server": server, "gateway": gateway_values})


def _setup_reply(request: dict[str, Any]) -> dict[str, Any]:
  op = request["op"]
  if op == "identity":
    return {
      "schema_version": SETUP_SCHEMA_VERSION,
      "ok": True,
      "diagnostics": [],
      "files": [],
      "identity": {
        "core": "ei",
        "product_version": product_version(),
        "graph_version": "0.2.0",
        "node_type_version": "0.2",
        "supported_graph_versions": list(SUPPORTED_GRAPH_VERSIONS),
        "supported_node_type_versions": list(SUPPORTED_NODE_TYPE_VERSIONS),
        "supported_node_types": sorted(EI_NODE_TYPES),
        "features": list(SETUP_FEATURES),
        "setup_schema_version": SETUP_SCHEMA_VERSION,
      },
    }
  if op == "catalog":
    return {
      "schema_version": SETUP_SCHEMA_VERSION,
      "ok": True,
      "diagnostics": [],
      "files": [],
      "catalog": {
        "profiles": [_profile()],
        "stage_types": [
          (
            _monitor_stage(node_type, layer, group)
            if node_type.endswith(".arm_status@v0.2")
            else _activation_stage(node_type, layer, group)
          )
          for node_type, (layer, group) in EI_NODE_TYPES.items()
        ],
        "routed_edges": [],
        "data_types": [],
        "defaults": ManagedValues().model_dump(mode="json"),
      },
    }
  if op == "options":
    return {
      "schema_version": SETUP_SCHEMA_VERSION,
      "ok": True,
      "diagnostics": [],
      "files": [],
      "catalog": {"setting_options": []},
    }
  values = _setup_values(request)
  return {
    "schema_version": SETUP_SCHEMA_VERSION,
    "ok": True,
    "diagnostics": [],
    "files": _render(values)["files"] if op == "render" else [],
  }


def _toml_string(value: str) -> str:
  return json.dumps(value, ensure_ascii=False)


def _render(values: ManagedValues) -> dict[str, Any]:
  gateway = values.gateway
  server = values.server
  server_content = (
    "# Managed by IsOS; other EI settings layer from factory configuration.\n"
    "[server]\n"
    f"host = {_toml_string(server.host)}\n"
    f"port = {server.port}\n"
    f"status_frequency_hz = {server.status_frequency_hz}\n"
  )
  gateway_content = "[modes.isos.gateway]\n"
  if gateway.transport == "uds":
    gateway_content += f'transport = "uds"\npath = {_toml_string(gateway.path or "")}\n'
  else:
    gateway_content += (
      'transport = "tcp"\n'
      f"host = {_toml_string(gateway.host or '')}\n"
      f"grpc_port = {gateway.grpc_port}\n"
    )
  return {
    "files": [
      {"path": "ei/ei.toml", "content": server_content},
      {"path": "ei/modes.toml", "content": 'default = "isos"\n'},
      {"path": "ei/profiles/isos.toml", "content": gateway_content},
    ]
  }


def _profile() -> dict[str, Any]:
  settings_fields = [
    {
      "name": "server.host",
      "type": "string",
      "label": "Listener host",
      "description": "Address for the EI northbound WebSocket listener.",
      "default": "127.0.0.1",
      "required": True,
    },
    {
      "name": "server.port",
      "type": "integer",
      "label": "Listener port",
      "description": "Port for the EI northbound WebSocket listener.",
      "default": 16060,
      "min": 1,
      "max": 65535,
      "required": True,
    },
    {
      "name": "server.status_frequency_hz",
      "type": "number",
      "label": "Status frequency (Hz)",
      "description": "Rate of health status updates; 0.5 sends one update every two seconds.",
      "default": 0.5,
      "exclusive_min": 0,
      "required": True,
    },
    {
      "name": "gateway.transport",
      "type": "enum",
      "label": "EOS transport",
      "description": "Connection type for the EOS gateway.",
      "default": "tcp",
      "options": ["uds", "tcp"],
      "required": True,
    },
    {
      "name": "gateway.host",
      "type": "string",
      "label": "EOS host",
      "description": "EOS gateway address when using TCP; omit for a Unix socket.",
      "default": "127.0.0.1",
    },
    {
      "name": "gateway.grpc_port",
      "type": "integer",
      "label": "EOS gRPC port",
      "description": "EOS gateway port when using TCP.",
      "default": 16200,
      "min": 1,
      "max": 65535,
    },
    {
      "name": "gateway.path",
      "type": "absolute_path",
      "label": "EOS socket path",
      "description": "Absolute Unix socket path when using UDS; omit for TCP.",
    },
  ]
  return {
    "id": "ei.agent",
    "version": PROFILE_VERSION,
    "template": "process",
    "inputs": [
      {
        "name": "eos",
        "type": "grpc",
        "required": True,
        "contracts": REQUIRED_CONTRACTS,
        "transports": ["uds", "tcp"],
      }
    ],
    "outputs": [],
    "settings_fields": settings_fields,
    "fields": {
      "server.host": {"type": "string", "default": "127.0.0.1"},
      "server.port": {"type": "integer", "default": 16060},
      "gateway.transport": {"type": "enum", "values": ["uds", "tcp"]},
      "gateway.path": {"type": "absolute_path"},
    },
    "provides": [],
    "requires": REQUIRED_CONTRACTS,
    "readiness": {
      "kind": "tcp",
      "host_field": "server.host",
      "port_field": "server.port",
    },
  }


def main(argv: list[str] | None = None) -> int:
  """Emit one versioned JSON reply; never create files or start runtime services."""
  public_name = "ei-config"
  parser = argparse.ArgumentParser(prog=public_name)
  parser.add_argument(
    "operation",
    choices=[
      "describe",
      "defaults",
      "profiles",
      "render",
      "validate",
      "setup",
      "owner",
    ],
  )
  parser.add_argument(
    "--json", action="store_true", help="JSON is the only output format"
  )
  parser.add_argument("--socket", help="Absolute private Unix socket for owner service")
  args = parser.parse_args(argv)
  operation = args.operation
  if operation == "owner":
    if not args.socket:
      parser.error("owner requires --socket")
    from ei.owner_service import serve

    return serve(args.socket)
  try:
    if operation == "setup":
      response = _setup_reply(_read_setup_request())
    elif operation == "describe":
      response = _reply(
        {
          "component": "ei",
          "version": product_version(),
          "provides": [],
          "requires": REQUIRED_CONTRACTS,
          "profile_version": PROFILE_VERSION,
        }
      )
    elif operation == "defaults":
      response = _reply(ManagedValues().model_dump(mode="json"))
    elif operation == "profiles":
      response = _reply([_profile()])
    else:
      values = _read_values()
      response = _reply(
        _render(values)
        if operation == "render"
        else {"values": values.model_dump(mode="json")}
      )
  except SetupDiagnostic as exc:
    response = {
      "schema_version": SETUP_SCHEMA_VERSION,
      "ok": False,
      "diagnostics": [{"path": exc.path, "code": exc.code, "message": str(exc)}],
      "files": [],
    }
  except ValidationError as exc:
    diagnostics = [
      {
        "path": ("endpoints." if operation == "setup" else "values.")
        + ".".join(str(part) for part in item["loc"]),
        "code": str(item["type"]),
        "message": str(item["msg"]),
      }
      for item in exc.errors()
    ]
    response = (
      {
        "schema_version": SETUP_SCHEMA_VERSION,
        "ok": False,
        "diagnostics": diagnostics,
        "files": [],
      }
      if operation == "setup"
      else _reply(diagnostics=diagnostics)
    )
  except (TypeError, ValueError) as exc:
    diagnostics = [{"path": "$", "code": "invalid_input", "message": str(exc)}]
    response = (
      {
        "schema_version": SETUP_SCHEMA_VERSION,
        "ok": False,
        "diagnostics": diagnostics,
        "files": [],
      }
      if operation == "setup"
      else _reply(diagnostics=diagnostics)
    )
  json.dump(response, sys.stdout, ensure_ascii=False, separators=(",", ":"))
  sys.stdout.write("\n")
  return 0 if response["ok"] else 2


if __name__ == "__main__":
  raise SystemExit(main())
