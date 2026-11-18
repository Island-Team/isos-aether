"""Side-effect-free IsOS configuration adapter for EI."""

from __future__ import annotations

import argparse
import json
import sys
from math import isfinite
from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field, ValidationError

from ei.core.system.isolated import DATA_TYPES as ISOLATED_DATA_TYPES
from ei.core.system.isolated import PORTS, IsolatedNode
from ei.core.system.node_registry import (
  BUILTIN_PLACEMENTS,
  NodeRegistryError,
  load_claims,
  load_registry,
)
from ei.lib.config import GatewayConfig, ServerConfig
from ei.reader_features import SETUP_FEATURES, product_version

SCHEMA_VERSION = 1
SETUP_SCHEMA_VERSION = 3
MAX_SETUP_INPUT_BYTES = 1024 * 1024
PROFILE_VERSION = "1.0.0"
SUPPORTED_GRAPH_VERSIONS = ("0.2.1",)
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


def _validate_ei_graph(
  graph: Any,
  *,
  validate_edges: bool = True,
  registry: dict[str, dict[str, Any]] | None = None,
) -> None:
  """Validate the current IsOS EI graph projection and its local routes."""
  if not isinstance(graph, dict) or graph.get("version") != "0.2.1":
    raise SetupDiagnostic(
      "/graph/version", "unsupported_graph_version", "EI requires graph 0.2.1"
    )
  _validate_ei_projection(
    graph, validate_edges=validate_edges, registry=registry or load_registry([])
  )


EI_NODE_TYPES = BUILTIN_PLACEMENTS

def _isolated_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  """Catalog a direct-invocation node without claiming a graph route."""
  _, input_port, input_type, output_type = PORTS[node_type]
  model = node_type in IsolatedNode.MODEL_TYPES
  fields = [
    {
      "name": "max_input_age_ms",
      "data_type": "integer",
      "default": 5000,
      "min": 1,
      "max": 60000,
      "unit": "ms",
      "required": False,
    },
    {"name": "frame_id", "data_type": "string", "required": False},
    {
      "name": "deadline_ms",
      "data_type": "integer",
      "default": 1000,
      "min": 1,
      "max": 30000,
      "unit": "ms",
      "required": False,
    },
    {
      "name": "max_output_bytes",
      "data_type": "integer",
      "default": 65536,
      "min": 1,
      "max": 65536,
      "unit": "byte",
      "required": False,
    },
    {
      "name": "max_rate_hz",
      "data_type": "number",
      "default": 20,
      "min": 0.000001,
      "max": 100,
      "unit": "Hz",
      "required": False,
    },
  ]
  if model:
    fields.append(
      {"name": "model_resource_id", "data_type": "string", "required": True}
    )
    fields.append(
      {
        "name": "shadow_mode",
        "data_type": "boolean",
        "default": True,
        "required": False,
      }
    )
  elif group == "memory":
    fields.append(
      {
        "name": "max_records",
        "data_type": "integer",
        "default": 32,
        "min": 1,
        "max": 256,
        "required": False,
      }
    )
  else:
    fields.append(
      {
        "name": "shadow_mode",
        "data_type": "boolean",
        "default": True,
        "required": False,
      }
    )
  return {
    "id": node_type,
    "core": "ei",
    "layer": layer,
    "group": group,
    "description": "Isolated EI cognition node; direct owner invocation only.",
    "kind": "software",
    "lifecycle": "internal",
    "capabilities": [],
    "implementation": {"kind": "built_in"},
    "availability": "requires_verified_model"
    if model
    else "ready_for_direct_invocation",
    "package_required": model,
    "settings_fields": fields,
    "inputs": [
      _port(
        input_port, "One bounded typed input.", input_type, "evidence", required=False
      )
    ],
    "outputs": [
      _port(
        "output",
        "Bounded typed output.",
        output_type,
        "proposal"
        if output_type in {"grounded_goal@v0.2", "motion_proposal@v0.2"}
        else "evidence",
        required=False,
      )
    ],
  }


# Input ports each full-cognition stage may receive over graph edges. Fixed
# owner-injected inputs (gateway/admitted/emergency_stop, intent, body, memory,
# progress, bounds, registry, latch) are intentionally excluded.
PIPELINE_STAGE_PORTS = {
  "ei.io.link.gateway@v0.2": set(),
  "ei.evidence.perception.admission@v0.2": {"resource", "hearing", "result"},
  "ei.cognition.context.builder@v0.2": {"observations"},
  "ei.cognition.missions.queue@v0.2": {"result"},
  "ei.cognition.decision.planner@v0.2": {"selected", "snapshot"},
  "ei.governance.monitor.health@v0.2": {"health"},
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
    "implementation": (
      {
        "kind": "external",
        "interface": "ei.owner.stage.v1",
        "version": "1",
        "lifecycle_adapter": "ei-owner-stage",
      }
      if first or last
      else {
        "kind": "native_plugin",
        "abi": "isos_node_v1",
        "library": "native/libei_demo_stage.so",
      }
    ),
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


def _context_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  """Stage that organizes agent input context before cognition.

  It receives the button activation and the latest arm observation, then
  emits the activation onward so the cognition forwarder can reason over a
  model-ready context derived from admitted perception.
  """
  stage = {
    "id": node_type,
    "core": "ei",
    "layer": layer,
    "group": group,
    "description": (
      "Organize agent input context from observations and forward the "
      "activation toward cognition."
    ),
    "kind": "software",
    "lifecycle": "internal",
    "implementation": {
      "kind": "native_plugin",
      "abi": "isos_node_v1",
      "library": "native/libei_demo_stage.so",
    },
    "capabilities": [],
    "settings_fields": [],
    "service_inputs": {},
    "inputs": [
      _activation_port(
        "in_process",
        "ei",
        "activation-local",
        required=True,
      ),
      {
        "name": "observation",
        "description": "Latest admitted arm observation folded into context.",
        "data_type": "arm_monitor_state@v0.2",
        "data_class": "observation",
        "transport": "in_process",
        "delivery_mode": "event",
        "cardinality": "one",
        "required": False,
        "contract": {"owner": "ei", "id": "arm-monitor-local", "version": "1"},
      },
    ],
    "outputs": [
      _activation_port(
        "in_process",
        "ei",
        "activation-local",
        required=False,
      ),
      {
        "name": "snapshot",
        "description": "Model-ready context snapshot built from admitted observations.",
        "data_type": "context_snapshot@v0.2",
        "data_class": "evidence",
        "transport": "in_process",
        "delivery_mode": "event",
        "cardinality": "one",
        "required": False,
        "contract": {"owner": "ei", "id": "context-local", "version": "1"},
      },
    ],
  }
  return stage


def _snapshot_port(
  transport: str, owner: str, contract_id: str, *, required: bool
) -> dict[str, Any]:
  return {
    "name": "snapshot",
    "description": "Model-ready context snapshot carried between cognition stages.",
    "data_type": "context_snapshot@v0.2",
    "data_class": "evidence",
    "transport": transport,
    "delivery_mode": "event",
    "cardinality": "one",
    "required": required,
    "contract": {"owner": owner, "id": contract_id, "version": "1"},
  }


def _forwarder_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  """Cognition intent forwarder that receives both activation and context snapshot."""
  stage = {
    "id": node_type,
    "core": "ei",
    "layer": layer,
    "group": group,
    "description": (
      "Consume the context snapshot while forwarding the activation event "
      "toward the safety gate."
    ),
    "kind": "software",
    "lifecycle": "internal",
    "implementation": {
      "kind": "external",
      "interface": "ei.owner.stage.v1",
      "version": "1",
      "lifecycle_adapter": "ei-owner-stage",
    },
    "capabilities": [],
    "settings_fields": [],
    "service_inputs": {},
    "inputs": [
      _activation_port("in_process", "ei", "activation-local", required=True),
      _snapshot_port("in_process", "ei", "context-local", required=False),
    ],
    "outputs": [
      _activation_port("in_process", "ei", "activation-local", required=False),
    ],
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
    "implementation": (
      {
        "kind": "external",
        "interface": "ei.owner.stage.v1",
        "version": "1",
        "lifecycle_adapter": "ei-owner-stage",
      }
      if first or last
      else {
        "kind": "native_plugin",
        "abi": "isos_node_v1",
        "library": "native/libei_demo_stage.so",
      }
    ),
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


def _valid_custom_settings(settings: Any, fields: Any) -> bool:
  if not isinstance(settings, dict) or not isinstance(fields, list):
    return False
  declarations = {
    field.get("name"): field for field in fields if isinstance(field, dict)
  }
  if len(declarations) != len(fields) or set(settings) - set(declarations):
    return False
  for name, declaration in declarations.items():
    if not isinstance(name, str):
      return False
    if name not in settings:
      if declaration.get("required") is True and "default" not in declaration:
        return False
      continue
    value = settings[name]
    kind = declaration.get("data_type")
    if kind in {"string", "enum", "absolute_path"}:
      if not isinstance(value, str):
        return False
      if kind == "enum" and value not in declaration.get("options", []):
        return False
    elif kind in {"array", "string_array", "object_array"}:
      if (
        not isinstance(value, list)
        or (kind == "string_array" and not all(isinstance(item, str) for item in value))
        or (
          kind == "object_array" and not all(isinstance(item, dict) for item in value)
        )
      ):
        return False
    elif kind in {"object", "string_table", "endpoint"}:
      if not isinstance(value, dict) or (
        kind == "string_table"
        and not all(isinstance(item, str) for item in value.values())
      ):
        return False
    elif kind == "boolean":
      if type(value) is not bool:
        return False
    elif kind in {"integer", "positive_integer", "nonnegative_integer"}:
      if (
        type(value) is not int
        or (kind == "positive_integer" and value <= 0)
        or (kind == "nonnegative_integer" and value < 0)
      ):
        return False
    elif kind in {"number", "positive_number"}:
      if type(value) not in {int, float} or not isfinite(value) or (kind == "positive_number" and value <= 0):
        return False
    else:
      return False
    if type(value) in {int, float} and (
      ("min" in declaration and value < declaration["min"])
      or ("max" in declaration and value > declaration["max"])
    ):
      return False
  return True


def _generic_stage(
  node_type: str,
  layer: str,
  group: str,
  description: str,
  inputs: list[dict[str, Any]],
  outputs: list[dict[str, Any]],
) -> dict[str, Any]:
  """Generic stage builder for in-process EI stages."""
  return {
    "id": node_type,
    "core": "ei",
    "layer": layer,
    "group": group,
    "description": description,
    "kind": "software",
    "lifecycle": "internal",
    "implementation": {
      "kind": "external",
      "interface": "ei.owner.stage.v1",
      "version": "1",
      "lifecycle_adapter": "ei-owner-stage",
    },
    "capabilities": [],
    "settings_fields": [],
    "service_inputs": {},
    "inputs": inputs,
    "outputs": outputs,
  }


def _data_type(identifier: str, purpose: str) -> dict[str, Any]:
  return {
    "id": identifier,
    "version": "0.2",
    "codec": "json_v1",
    "media_type": "application/json",
    "max_payload_bytes": 65536,
    "purpose": purpose,
    "schema": {"type": "object"},
  }


# Data types carried on the full-cognition pipeline edges. IsOS merges these
# into its catalog so edge admission can resolve their unit schemas.
EI_DATA_TYPES = [
  _data_type("resource_bundle@v0.2", "Resource descriptors emitted by the EI gateway."),
  _data_type("observation_set@v0.2", "Admitted observation set rendered for cognition."),
  _data_type("context_snapshot@v0.2", "Bounded model-ready context snapshot."),
  _data_type("selected_mission@v0.2", "Mission selected for the planner."),
  _data_type("primitive_proposal@v0.2", "Typed primitive proposal from the planner."),
  _data_type("health_update@v0.2", "Backend health update from the gateway."),
] + ISOLATED_DATA_TYPES


def _safety_gate_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  stage = _activation_stage(node_type, layer, group)
  stage["description"] = (
    "Gate button activation and planner proposals before EOS admission."
  )
  stage["inputs"].append(
    _port(
      "proposal",
      "Typed primitive proposal from the planner.",
      "primitive_proposal@v0.2",
      "proposal",
      required=False,
    )
  )
  return stage


def _port(
  name: str,
  description: str,
  data_type: str,
  data_class: str,
  *,
  contract_owner: str = "ei",
  contract_id: str | None = None,
  required: bool = True,
) -> dict[str, Any]:
  if contract_id is None:
    contract_id = name + "-local"
  return {
    "name": name,
    "description": description,
    "data_type": data_type,
    "data_class": data_class,
    "transport": "in_process",
    "delivery_mode": "event",
    "cardinality": "one",
    "required": required,
    "contract": {"owner": contract_owner, "id": contract_id, "version": "1"},
  }


def _gateway_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  return _generic_stage(
    node_type, layer, group,
    "Connects a stable backend alias to verified EOS discovery and operation results.",
    inputs=[
      _port("gateway", "Gateway endpoint configuration.", "gateway_endpoint@v0.2", "configuration", contract_id="gateway-cfg", required=False),
      _port("admitted", "Previously admitted command state.", "admission_record@v0.2", "command", contract_id="gateway-admitted", required=False),
      _port("emergency_stop", "Emergency stop signal.", "emergency_stop@v0.2", "command", contract_id="gateway-emergency", required=False),
    ],
    outputs=[
      _port("discovery", "Discovered backend resources.", "discovery_result@v0.2", "evidence", contract_id="gateway-discovery", required=False),
      _port("resource", "Resource descriptors for admission.", "resource_bundle@v0.2", "evidence", contract_id="gateway-resource", required=False),
      _port("result", "Operation result feedback.", "operation_result@v0.2", "feedback", contract_id="gateway-result", required=False),
      _port("health", "Backend health update.", "health_update@v0.2", "feedback", contract_id="gateway-health", required=False),
    ],
  )


def _admission_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  return _generic_stage(
    node_type, layer, group,
    "Validates and bounds attributed EOS and operator evidence.",
    inputs=[
      _port("resource", "Resource descriptors from gateway.", "resource_bundle@v0.2", "evidence", contract_id="gateway-resource", required=False),
      _port("hearing", "Direct operator/operator input.", "hearing_event@v0.2", "evidence", required=False),
      _port("result", "Past operation result for admission context.", "operation_result@v0.2", "feedback", required=False),
    ],
    outputs=[
      _port("observations", "Admitted observations rendered for cognition.", "observation_set@v0.2", "evidence", required=False),
    ],
  )


def _builder_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  return _generic_stage(
    node_type, layer, group,
    "Renders a deterministic bounded context snapshot from evidence.",
    inputs=[
      _port("observations", "Admitted observations from admission.", "observation_set@v0.2", "evidence", required=False),
      _port("body", "Embodied state information.", "body_state@v0.2", "evidence", required=False),
      _port("memory", "Recalled memory items.", "memory_set@v0.2", "evidence", required=False),
    ],
    outputs=[
      _port("snapshot", "Model-ready context snapshot.", "context_snapshot@v0.2", "evidence", required=False),
    ],
  )


def _queue_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  return _generic_stage(
    node_type, layer, group,
    "Owns mission admission, selection, revisions, and durable lifecycle.",
    inputs=[
      _port("intent", "Mission intent (owner-internal, not wired in graph).", "mission_intent@v0.2", "mission", required=False),
      _port("result", "Previous operation result.", "operation_result@v0.2", "feedback", required=False),
    ],
    outputs=[
      _port("selected", "Selected mission for planner.", "selected_mission@v0.2", "mission", required=False),
      _port("lifecycle", "Mission lifecycle event.", "mission_lifecycle@v0.2", "mission", required=False),
    ],
  )


def _planner_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  stage = _generic_stage(
    node_type, layer, group,
    "Produces typed primitive proposals and plan state from a mission and context.",
    inputs=[
      _port("selected", "Selected mission from queue.", "selected_mission@v0.2", "mission", required=False),
      _port("snapshot", "Context snapshot from builder.", "context_snapshot@v0.2", "evidence", required=False),
    ],
    outputs=[
      _port("proposal", "Typed primitive proposal.", "primitive_proposal@v0.2", "proposal", required=False),
      _port("plan_state", "Plan state update.", "plan_state@v0.2", "mission", required=False),
    ],
  )
  stage["availability"] = "requires_verified_model"
  stage["package_required"] = True
  return stage


def _health_monitor_stage(node_type: str, layer: str, group: str) -> dict[str, Any]:
  return _generic_stage(
    node_type, layer, group,
    "Tracks backend health and progress and owns the emergency latch.",
    inputs=[
      _port("health", "Health update from gateway.", "health_update@v0.2", "feedback", contract_id="gateway-health", required=False),
      _port("progress", "Long-running operation progress (owner-internal).", "progress_update@v0.2", "feedback", required=False),
      _port("bounds", "Safety bounds configuration.", "safety_bounds@v0.2", "evidence", required=False),
    ],
    outputs=[
      _port("status", "Health status report.", "health_status@v0.2", "feedback", required=False),
      _port("latch", "Emergency latch activation.", "emergency_latch@v0.2", "evidence", required=False),
    ],
  )


def _validate_ei_projection(
  graph: dict[str, Any], *, validate_edges: bool, registry: dict[str, dict[str, Any]]
) -> None:
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
        spec = registry.get(node_type) if isinstance(node_type, str) else None
        placement = (spec["layer"], spec["group"]) if spec is not None else None
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
        assert spec is not None
        if node_type in PORTS:
          raise SetupDiagnostic(
            f"{node_path}/node_type", "isolated_node_only",
            "EI isolated library nodes have no graph execution route",
          )
        if node_type == "ei.cognition.decision.planner@v0.2":
          raise SetupDiagnostic(
            f"{node_path}/node_type", "missing_model_package",
            "EI planner requires a verified model package before graph execution",
          )
        custom = node_type not in EI_NODE_TYPES
        if custom:
          pin = node.get("package")
          if not isinstance(pin, dict) or pin != {
            "id": spec["_package_id"],
            "version": spec["_package_version"],
          }:
            raise SetupDiagnostic(
              f"{node_path}/package",
              "package_pin_mismatch",
              "EI node package pin does not match its registered release",
            )
        settings = node.get("settings", {})
        monitor_egress = node_type == "ei.governance.monitor.arm_status@v0.2"
        valid_settings = (
          _valid_custom_settings(settings, spec.get("settings", []))
          if custom
          else isinstance(settings, dict)
          and (
            (
              set(settings) <= {"forward_rate_hz"}
              and (
                "forward_rate_hz" not in settings
                or type(settings["forward_rate_hz"]) in (int, float)
                and 1 <= settings["forward_rate_hz"] <= 20
              )
            )
            if monitor_egress
            else not settings
          )
        )
        if (
          set(node)
          - {"id", "label", "description", "node_type", "settings", "inputs", "package"}
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
        # Full-cognition pipeline stages admit only their edge-wired ports.
        if isinstance(node_type, str) and node_type in PIPELINE_STAGE_PORTS:
          allowed_ports = PIPELINE_STAGE_PORTS[node_type]
        elif node_type == "ei.cognition.intent.context@v0.2":
          allowed_ports = {"activation", "observation"}
        elif node_type == "ei.cognition.intent.forwarder@v0.2":
          allowed_ports = {"activation", "snapshot"}
        elif node_type == "ei.governance.safety.gate@v0.2":
          allowed_ports = {"activation", "proposal"}
        elif custom:
          allowed_ports = {item["name"] for item in spec.get("inputs", [])}
        else:
          allowed_ports = {port_name}
        if not isinstance(inputs, dict) or set(inputs) - allowed_ports:
          raise SetupDiagnostic(
            f"{node_path}/inputs",
            "unsupported_node_port",
            "EI stage has an unsupported input port",
          )
        for input_name, link in inputs.items():
          if (
            not isinstance(link, dict)
            or not isinstance(link.get("node"), str)
            or not isinstance(link.get("output"), str)
            or (not custom and link.get("output") != input_name)
            or set(link) - {"node", "output", "priority", "frequency_hz"}
          ):
            raise SetupDiagnostic(
              f"{node_path}/inputs/{input_name}",
              "invalid_node_input",
              f"{input_name} requires a source node and {input_name} output",
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
  try:
    registry = load_registry(request["packages"])
  except (NodeRegistryError, OSError, ValueError, TypeError) as exc:
    raise SetupDiagnostic("/packages", "invalid_node_registry", str(exc)) from exc
  _validate_ei_graph(
    graph, validate_edges=request["op"] not in {"catalog", "options"}, registry=registry
  )
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


def _route(
  source: str,
  target: str,
  port: str,
  transport: str,
  data_type: str,
  owner: str,
  contract: str,
) -> dict[str, Any]:
  return {
    "source_type": source,
    "output": port,
    "target_type": target,
    "input": port,
    "transport": transport,
    "delivery_mode": "event",
    "data_type": data_type,
    "contract": {"owner": owner, "id": contract, "version": "1"},
  }


EI_ROUTES = [
  _route(
    "isos.ui.button.trigger@v0.2",
    "ei.io.input.trigger@v0.2",
    "activation",
    "unix_socket",
    "button_activation@v0.2",
    "isos",
    "button-activation",
  ),
  _route(
    "ei.io.input.trigger@v0.2",
    "ei.cognition.intent.context@v0.2",
    "activation",
    "in_process",
    "button_activation@v0.2",
    "ei",
    "activation-local",
  ),
  _route(
    "ei.cognition.state.arm_status@v0.2",
    "ei.cognition.intent.context@v0.2",
    "observation",
    "in_process",
    "arm_monitor_state@v0.2",
    "ei",
    "arm-monitor-local",
  ),
  _route(
    "ei.cognition.intent.context@v0.2",
    "ei.cognition.intent.forwarder@v0.2",
    "activation",
    "in_process",
    "button_activation@v0.2",
    "ei",
    "activation-local",
  ),
  _route(
    "ei.cognition.intent.forwarder@v0.2",
    "ei.governance.safety.gate@v0.2",
    "activation",
    "in_process",
    "button_activation@v0.2",
    "ei",
    "activation-local",
  ),
  _route(
    "ei.governance.safety.gate@v0.2",
    "eos.interface.ingress.trigger@v0.2",
    "activation",
    "unix_socket",
    "button_activation@v0.2",
    "ei",
    "activation-to-eos",
  ),
  _route(
    "eos.interface.egress.arm_status@v0.2",
    "ei.io.input.arm_status@v0.2",
    "observation",
    "unix_socket",
    "arm_monitor_state@v0.2",
    "eos",
    "arm-monitor-to-ei",
  ),
  _route(
    "ei.io.input.arm_status@v0.2",
    "ei.cognition.state.arm_status@v0.2",
    "observation",
    "in_process",
    "arm_monitor_state@v0.2",
    "ei",
    "arm-monitor-local",
  ),
  _route(
    "ei.cognition.state.arm_status@v0.2",
    "ei.governance.monitor.arm_status@v0.2",
    "observation",
    "in_process",
    "arm_monitor_state@v0.2",
    "ei",
    "arm-monitor-local",
  ),
  _route(
    "ei.governance.monitor.arm_status@v0.2",
    "isos.ui.monitor.arm@v0.2",
    "observation",
    "unix_socket",
    "arm_monitor_state@v0.2",
    "ei",
    "arm-monitor-to-isos",
  ),
  _route(
    "ei.io.link.gateway@v0.2",
    "ei.evidence.perception.admission@v0.2",
    "resource",
    "in_process",
    "resource_bundle@v0.2",
    "ei",
    "gateway-resource",
  ),
  _route(
    "ei.evidence.perception.admission@v0.2",
    "ei.cognition.context.builder@v0.2",
    "observations",
    "in_process",
    "observation_set@v0.2",
    "ei",
    "observations-local",
  ),
  _route(
    "ei.cognition.context.builder@v0.2",
    "ei.cognition.decision.planner@v0.2",
    "snapshot",
    "in_process",
    "context_snapshot@v0.2",
    "ei",
    "snapshot-local",
  ),
  _route(
    "ei.cognition.missions.queue@v0.2",
    "ei.cognition.decision.planner@v0.2",
    "selected",
    "in_process",
    "selected_mission@v0.2",
    "ei",
    "selected-local",
  ),
  _route(
    "ei.cognition.decision.planner@v0.2",
    "ei.governance.safety.gate@v0.2",
    "proposal",
    "in_process",
    "primitive_proposal@v0.2",
    "ei",
    "proposal-local",
  ),
  _route(
    "ei.io.link.gateway@v0.2",
    "ei.governance.monitor.health@v0.2",
    "health",
    "in_process",
    "health_update@v0.2",
    "ei",
    "gateway-health",
  ),
  _route(
    "ei.cognition.intent.context@v0.2",
    "ei.cognition.intent.forwarder@v0.2",
    "snapshot",
    "in_process",
    "context_snapshot@v0.2",
    "ei",
    "context-local",
  ),
]


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
        "graph_version": "0.2.1",
        "node_type_version": "0.2",
        "supported_graph_versions": list(SUPPORTED_GRAPH_VERSIONS),
        "supported_node_type_versions": list(SUPPORTED_NODE_TYPE_VERSIONS),
        "supported_node_types": sorted(node_id for node_id in EI_NODE_TYPES if node_id not in PORTS),
        "features": list(SETUP_FEATURES),
        "setup_schema_version": SETUP_SCHEMA_VERSION,
      },
    }
  if op == "catalog":
    registry = load_registry(request["packages"])
    package_routes, package_bridges = load_claims(request["packages"], registry)
    return {
      "schema_version": SETUP_SCHEMA_VERSION,
      "ok": True,
      "diagnostics": [],
      "files": [],
      "catalog": {
        "profiles": [_profile()],
        "stage_types": [
          (
            _gateway_stage(node_type, layer, group)
            if node_type == "ei.io.link.gateway@v0.2"
            else _isolated_stage(node_type, layer, group)
            if node_type in PORTS
            else _admission_stage(node_type, layer, group)
            if node_type == "ei.evidence.perception.admission@v0.2"
            else _builder_stage(node_type, layer, group)
            if node_type == "ei.cognition.context.builder@v0.2"
            else _queue_stage(node_type, layer, group)
            if node_type == "ei.cognition.missions.queue@v0.2"
            else _planner_stage(node_type, layer, group)
            if node_type == "ei.cognition.decision.planner@v0.2"
            else _health_monitor_stage(node_type, layer, group)
            if node_type == "ei.governance.monitor.health@v0.2"
            else _monitor_stage(node_type, layer, group)
            if node_type.endswith(".arm_status@v0.2")
            else _safety_gate_stage(node_type, layer, group)
            if node_type == "ei.governance.safety.gate@v0.2"
            else _context_stage(node_type, layer, group)
            if node_type == "ei.cognition.intent.context@v0.2"
            else _forwarder_stage(node_type, layer, group)
            if node_type == "ei.cognition.intent.forwarder@v0.2"
            else _activation_stage(node_type, layer, group)
          )
          for node_type, (layer, group) in EI_NODE_TYPES.items()
        ]
        + [
          {key: value for key, value in spec.items() if not key.startswith("_")}
          for node_id, spec in registry.items()
          if node_id not in EI_NODE_TYPES
        ],
        "routed_edges": [],
        "port_routes": EI_ROUTES + package_routes,
        "bridge_adapters": [
          {"core": "ei", "adapter": "unix_socket", "node_type": node_type}
          for node_type in (
            "ei.io.input.trigger@v0.2",
            "ei.governance.safety.gate@v0.2",
            "ei.io.input.arm_status@v0.2",
            "ei.governance.monitor.arm_status@v0.2",
          )
        ]
        + package_bridges,
        "data_types": EI_DATA_TYPES,
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
    from ei.core.system.owner_service import serve

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
