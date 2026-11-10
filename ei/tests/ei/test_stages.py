"""Graph-owned stage identity and governed local delivery tests."""

from __future__ import annotations

import asyncio
from datetime import UTC, datetime, timedelta
from typing import Any

import pytest

from ei.config_cli import SetupDiagnostic, _setup_reply, _validate_ei_graph
from ei.stages import (
  MAX_STAGE_EVENT_AGE,
  STAGE_TYPES,
  StageGraphError,
  StagePayload,
  StageRuntime,
  StageValue,
  compose_stages,
)

_FIELDS = {
  "discovery": {"backend": "robot-a", "generation": 1},
  "observations": {"items": []},
  "snapshot": {"text": ""},
  "selected": {"mission_id": "m1", "revision": 1},
  "proposal": {
    "backend": "robot-a",
    "device": "arm",
    "primitive": "move",
    "version": "1",
    "arguments": {},
  },
  "admission": {
    "backend": "robot-a",
    "device": "arm",
    "primitive": "move",
    "version": "1",
    "arguments": {},
  },
  "status": {"state": "unavailable", "episode": 0},
}


def _value(
  kind: str, event_id: str, run_id: str, source: str, when: datetime
) -> StageValue:
  return StageValue(
    kind,
    event_id,
    run_id,
    source,
    when,
    StagePayload(kind, _FIELDS[kind]),
    received_at=when,
  )


def _graph() -> dict:
  nodes: dict[str, dict] = {}
  for spec in STAGE_TYPES.values():
    path = f"isos/ei_{spec.layer}/{spec.group}/{spec.id.split('.')[2].split('@')[0]}"
    node_id = path.rsplit("/", 1)[-1]
    settings = (
      {
        "backend_alias": "robot-a",
        "gateway": {"transport": "tcp", "host": "127.0.0.1", "port": 16022},
      }
      if spec.group == "link"
      else {}
    )
    nodes[path] = {
      "id": node_id,
      "description": spec.description,
      "node_type": spec.id,
      "settings": settings,
      "inputs": {},
    }

  def wire(target: str, input_name: str, source: str, output: str) -> None:
    nodes[target]["inputs"][input_name] = {"node": source, "output": output}

  paths = {
    spec.group: path
    for path, node in nodes.items()
    for spec in STAGE_TYPES.values()
    if node["node_type"] == spec.id
  }
  wire(paths["perception"], "resource", paths["link"], "resource")
  wire(paths["perception"], "result", paths["link"], "result")
  wire(paths["context"], "observations", paths["perception"], "observations")
  wire(paths["decision"], "selected", paths["missions"], "selected")
  wire(paths["decision"], "snapshot", paths["context"], "snapshot")
  wire(paths["safety"], "proposal", paths["decision"], "proposal")
  wire(paths["monitor"], "health", paths["link"], "health")
  layers: dict = {}
  for path, node in nodes.items():
    _, layer, group, node_id = path.split("/")
    layers.setdefault(layer, {"groups": {}})["groups"].setdefault(group, {"nodes": {}})[
      "nodes"
    ][node_id] = node
  return {"version": "0.2.0", "robot": {}, "cores": {"isos": {"layers": layers}}}


def _owner_graph() -> dict:
  return {
    "version": "0.2.0",
    "robot": {},
    "cores": {
      "ei": {
        "layers": {
          "io": {
            "groups": {
              "input": {
                "nodes": {
                  "gateway": {
                    "id": "gateway",
                    "node_type": "ei.io.input.trigger@v0.2",
                    "settings": {},
                    "inputs": {
                      "activation": {
                        "node": "isos/ui/button/trigger",
                        "output": "activation",
                      }
                    },
                  }
                }
              }
            }
          },
          "cognition": {
            "groups": {
              "intent": {
                "nodes": {
                  "builder": {
                    "id": "builder",
                    "node_type": "ei.cognition.intent.forwarder@v0.2",
                    "settings": {},
                    "inputs": {
                      "activation": {
                        "node": "ei/io/input/gateway",
                        "output": "activation",
                      }
                    },
                  }
                }
              }
            }
          },
          "governance": {
            "groups": {
              "safety": {
                "nodes": {
                  "gate": {
                    "id": "gate",
                    "node_type": "ei.governance.safety.gate@v0.2",
                    "settings": {},
                    "inputs": {
                      "activation": {
                        "node": "ei/cognition/intent/builder",
                        "output": "activation",
                      }
                    },
                  }
                }
              }
            }
          },
        }
      }
    },
  }


def _owner_setup(graph: dict, op: str = "render") -> dict:
  return {
    "schema_version": 2,
    "op": op,
    "core": "ei",
    "revision": "a" * 64,
    "graph": graph,
    "endpoints": {},
    "packages": [],
  }


def test_owner_projection_renders_no_stage_overlay() -> None:
  graph = _owner_graph()
  _validate_ei_graph(graph)
  files = _setup_reply(_owner_setup(graph))["files"]
  assert [item["path"] for item in files] == [
    "ei/ei.toml",
    "ei/modes.toml",
    "ei/profiles/isos.toml",
  ]


def test_owner_projection_rejects_authored_input_without_a_port() -> None:
  graph = _owner_graph()
  gateway = graph["cores"]["ei"]["layers"]["io"]["groups"]["input"]["nodes"]["gateway"]
  gateway["inputs"] = {"in": {"node": "ei/cognition/context/builder", "output": "out"}}
  with pytest.raises(SetupDiagnostic) as error:
    _validate_ei_graph(graph)
  assert error.value.code == "unsupported_node_port"
  assert (
    error.value.path == "/graph/cores/ei/layers/io/groups/input/nodes/gateway/inputs"
  )


def test_owner_projection_rejects_retired_type_at_exact_path() -> None:
  graph = _owner_graph()
  graph["cores"]["ei"]["layers"]["io"]["groups"]["input"]["nodes"]["gateway"][
    "node_type"
  ] = "isos.ei_io.gateway@v0.2"
  with pytest.raises(SetupDiagnostic) as error:
    _validate_ei_graph(graph)
  assert error.value.code == "unsupported_node_type_version"
  assert (
    error.value.path == "/graph/cores/ei/layers/io/groups/input/nodes/gateway/node_type"
  )


def test_owner_catalog_exposes_activation_and_monitor_stages() -> None:
  catalog = _setup_reply(_owner_setup(_owner_graph(), "catalog"))["catalog"]
  assert len(catalog["stage_types"]) == 6
  assert catalog["routed_edges"] == []
  assert catalog["data_types"] == []
  assert {item["id"] for item in catalog["stage_types"]} == {
    "ei.io.input.trigger@v0.2",
    "ei.cognition.intent.forwarder@v0.2",
    "ei.governance.safety.gate@v0.2",
    "ei.io.input.arm_status@v0.2",
    "ei.cognition.state.arm_status@v0.2",
    "ei.governance.monitor.arm_status@v0.2",
  }
  for item in catalog["stage_types"]:
    if item["id"] == "ei.governance.monitor.arm_status@v0.2":
      assert item["settings_fields"][0]["name"] == "forward_rate_hz"
      assert item["settings_fields"][0]["data_type"] == "number"
      assert "type" not in item["settings_fields"][0]
      assert item["settings_fields"][0]["default"] == 20
    else:
      assert item["settings_fields"] == []
    assert item["inputs"][0]["data_type"] == (
      "arm_monitor_state@v0.2"
      if item["id"].endswith(".arm_status@v0.2")
      else "button_activation@v0.2"
    )
    assert item["outputs"][0]["delivery_mode"] == "event"


def test_monitor_forward_rate_setting_is_bounded() -> None:
  graph = _owner_graph()
  graph["cores"]["ei"]["layers"]["governance"]["groups"]["monitor"] = {
    "nodes": {
      "arm_status": {
        "node_type": "ei.governance.monitor.arm_status@v0.2",
        "settings": {"forward_rate_hz": 10},
        "inputs": {},
      }
    }
  }
  _validate_ei_graph(graph)
  settings = graph["cores"]["ei"]["layers"]["governance"]["groups"]["monitor"]["nodes"][
    "arm_status"
  ]["settings"]
  settings["forward_rate_hz"] = 21
  with pytest.raises(SetupDiagnostic) as error:
    _validate_ei_graph(graph)
  assert error.value.code == "invalid_node_settings"


def test_owner_projection_rejects_legacy_wrapper_type() -> None:
  graph = _owner_graph()
  graph["cores"]["ei"]["layers"]["io"]["groups"]["input"]["nodes"]["gateway"][
    "node_type"
  ] = "ei.io.input.gateway@v0.2"
  with pytest.raises(SetupDiagnostic) as error:
    _validate_ei_graph(graph)
  assert error.value.code == "unsupported_node_type_version"


def test_composes_exact_seven_stage_instances() -> None:
  composition = compose_stages(_graph())
  assert len(composition.instances) == 7
  assert len(composition.edges) == 7
  assert all(instance.description for instance in composition.instances.values())


def test_only_exact_replaceable_evidence_routes_accept_qos() -> None:
  graph = _graph()
  nodes = {
    node["node_type"].split(".")[2].split("@")[0]: node for _, node in _nodes(graph)
  }
  nodes["admission"]["inputs"]["resource"].update(priority=-20, frequency_hz=2.5)
  nodes["builder"]["inputs"]["observations"].update(priority=19, frequency_hz=1)
  composition = compose_stages(graph)
  assert {
    (edge.output, edge.input, edge.priority, edge.frequency_hz)
    for edge in composition.edges
    if edge.frequency_hz is not None
  } == {
    ("resource", "resource", -20, 2.5),
    ("observations", "observations", 19, 1),
  }
  for target, input_name in (
    ("admission", "result"),
    ("health", "health"),
    ("planner", "snapshot"),
  ):
    rejected = _graph()
    node = next(
      node
      for _, node in _nodes(rejected)
      if node["node_type"].split(".")[2].split("@")[0] == target
    )
    node["inputs"][input_name]["frequency_hz"] = 1
    with pytest.raises(StageGraphError, match="no edge QoS") as exc:
      compose_stages(rejected)
    assert exc.value.code == "unsupported_edge_control"


def test_single_source_array_preserves_cardinality_one() -> None:
  graph = _graph()
  context = next(
    node
    for _, node in _nodes(graph)
    if node["node_type"].split(".")[2].split("@")[0] == "builder"
  )
  source = context["inputs"]["observations"]
  context["inputs"]["observations"] = [source]
  assert len(compose_stages(graph).edges) == 7
  context["inputs"]["observations"] = [source, source]
  with pytest.raises(StageGraphError) as exc:
    compose_stages(graph)
  assert exc.value.code == "invalid_cardinality"


@pytest.mark.parametrize(
  "field,value,code,suffix",
  [
    ("priority", -21, "invalid_edge_priority", "/priority"),
    ("frequency_hz", 0, "invalid_edge_frequency", "/frequency_hz"),
    ("node", "missing", "unknown_source", "/node"),
    ("output", "missing", "unknown_output", "/output"),
  ],
)
def test_single_source_array_reports_exact_nested_pointer(
  field: str, value: object, code: str, suffix: str
) -> None:
  graph = _graph()
  context = next(
    node
    for _, node in _nodes(graph)
    if node["node_type"].split(".")[2].split("@")[0] == "builder"
  )
  source = context["inputs"]["observations"]
  source[field] = value
  context["inputs"]["observations"] = [source]
  with pytest.raises(StageGraphError) as exc:
    compose_stages(graph)
  assert exc.value.code == code
  assert exc.value.path == (
    "/graph/cores/isos/layers/ei_cognition/groups/context/nodes/builder"
    f"/inputs/observations/0{suffix}"
  )


def test_single_source_array_unsupported_qos_reports_edge_pointer() -> None:
  graph = _graph()
  decision = next(
    node
    for _, node in _nodes(graph)
    if node["node_type"].split(".")[2].split("@")[0] == "planner"
  )
  source = decision["inputs"]["snapshot"]
  source["priority"] = 1
  decision["inputs"]["snapshot"] = [source]
  with pytest.raises(StageGraphError) as exc:
    compose_stages(graph)
  assert exc.value.code == "unsupported_edge_control"
  assert exc.value.path == (
    "/graph/cores/isos/layers/ei_cognition/groups/decision/nodes/planner/inputs/snapshot/0"
  )


@pytest.mark.parametrize("frequency", [0, -1, float("inf"), float("nan"), True, "2"])
def test_invalid_evidence_frequency_rejected_at_field(frequency: object) -> None:
  graph = _graph()
  node = next(
    node
    for _, node in _nodes(graph)
    if node["node_type"].split(".")[2].split("@")[0] == "builder"
  )
  node["inputs"]["observations"]["frequency_hz"] = frequency
  with pytest.raises(StageGraphError) as exc:
    compose_stages(graph)
  assert exc.value.code == "invalid_edge_frequency"
  assert exc.value.path.endswith("/frequency_hz")


def test_empty_current_graph_has_no_stage_instances() -> None:
  graph = {"version": "0.2.0", "cores": {"isos": {"layers": {}}}}
  assert compose_stages(graph).instances == {}


@pytest.mark.parametrize(
  "group,input_name",
  [("link", "admitted"), ("link", "emergency_stop"), ("safety", "latch")],
)
def test_authored_command_or_latch_route_is_rejected(
  group: str, input_name: str
) -> None:
  graph = _graph()
  _, node = next((path, node) for path, node in _nodes(graph) if f"/{group}/" in path)
  node["inputs"][input_name] = {
    "node": "isos/ei_cognition/decision/planner",
    "output": "proposal",
  }
  with pytest.raises(StageGraphError, match="fixed runtime input") as exc:
    compose_stages(graph)
  assert exc.value.code == "fixed_route"
  assert input_name in exc.value.path


def _nodes(graph: dict):
  for layer_id, layer in graph["cores"]["isos"]["layers"].items():
    for group_id, group in layer["groups"].items():
      for node_id, node in group["nodes"].items():
        yield f"isos/{layer_id}/{group_id}/{node_id}", node


@pytest.mark.parametrize("type_id", list(STAGE_TYPES))
def test_each_stage_rejects_malformed_duplicate_stale_and_stop(type_id: str) -> None:
  composition = compose_stages(_graph())
  origin = next(
    instance
    for instance in composition.instances.values()
    if instance.type.id == type_id
  )
  output, kind = next(iter(origin.type.outputs.items()))
  runtime = StageRuntime(composition, "run-1")
  for path in composition.instances:
    runtime.bind_owner(path, object())
  runtime.start()
  now = datetime.now(UTC)
  value = _value(kind, "event-1", "run-1", origin.path, now - timedelta(seconds=1))
  with pytest.raises(ValueError, match="source output"):
    runtime.publish("wrong", _value(kind, "event-2", "run-1", origin.path, now))
  with pytest.raises(ValueError, match="stale run"):
    runtime.publish(output, _value(kind, "event-2", "run-old", origin.path, now))
  delivered: list[StageValue] = []
  for edge in composition.edges:
    if edge.source == origin.path and edge.output == output:
      runtime.bind(edge.target, edge.input, delivered.append)
  assert runtime.publish(output, value) == len(delivered)
  assert runtime.publish(output, value) == 0
  assert runtime.diagnostics[-1].code == "duplicate_event"
  with pytest.raises(ValueError, match="conflicting duplicate"):
    runtime.publish(
      output,
      StageValue(
        kind,
        "event-1",
        "run-1",
        origin.path,
        now,
        StagePayload(kind, _FIELDS[kind]),
        received_at=now,
      ),
    )
  assert runtime.diagnostics[-1].code == "conflicting_event"
  runtime.stop()
  with pytest.raises(RuntimeError, match="stopping"):
    runtime.publish(output, value)


def test_link_disconnect_cannot_invent_successful_eos_result() -> None:
  composition = compose_stages(_graph())
  runtime = StageRuntime(composition, "run-1")
  for path in composition.instances:
    runtime.bind_owner(path, object())
  runtime.start()
  link = next(
    instance
    for instance in composition.instances.values()
    if instance.type.group == "link"
  )
  result_edge = next(
    edge
    for edge in composition.edges
    if edge.source == link.path and edge.output == "result"
  )
  seen: list[StageValue] = []
  runtime.bind(result_edge.target, result_edge.input, seen.append)
  now = datetime.now(UTC)
  with pytest.raises(ValueError, match="source output"):
    runtime.publish(
      "result",
      _value("admission", "e1", "run-1", link.path, now),
    )
  assert seen == []


def test_typed_payload_deep_freezes_admitted_arguments() -> None:
  fields: dict[str, Any] = {
    "backend": "robot-a",
    "device": "arm",
    "primitive": "move",
    "version": "1",
    "arguments": {"joint": [1, 2]},
  }
  payload = StagePayload("admission", fields)
  fields["arguments"]["joint"].append(3)
  assert payload.fields["arguments"]["joint"] == (1, 2)
  with pytest.raises(ValueError, match="proposal.device"):
    StagePayload("proposal", {"backend": "robot-a"})


def test_same_run_observation_age_is_bounded_before_deduplication() -> None:
  composition = compose_stages(_graph())
  now = datetime(2026, 11, 2, 12, 0, tzinfo=UTC)
  runtime = StageRuntime(composition, "run-1", clock=lambda: now)
  for path in composition.instances:
    runtime.bind_owner(path, object())
  runtime.start()
  link = next(
    path for path, item in composition.instances.items() if item.type.group == "link"
  )
  edge = next(
    edge
    for edge in composition.edges
    if edge.source == link and edge.output == "result"
  )
  seen: list[StageValue] = []
  runtime.bind(edge.target, edge.input, seen.append)
  old = now - MAX_STAGE_EVENT_AGE - timedelta(microseconds=1)
  stale = StageValue(
    "result",
    "old-1",
    "run-1",
    link,
    old,
    StagePayload("result", {"operation_id": "op-1", "terminal_state": "failed"}),
    received_at=old,
  )
  with pytest.raises(ValueError, match="stale stage observation"):
    runtime.publish("result", stale)
  assert runtime.diagnostics[-1].code == "stale_event"
  assert runtime.diagnostics[-1].node == link
  assert runtime.diagnostics[-1].event_id == "old-1"
  assert seen == []
  fresh = StageValue(
    "result",
    "old-1",
    "run-1",
    link,
    now,
    stale.payload,
    received_at=now,
  )
  assert runtime.publish("result", fresh) == 1
  assert seen == [fresh]


@pytest.mark.parametrize(
  "sample", [{}, {"text": ""}, {"text": "ready"}, {"text": "ready", "eos_robot_id": ""}]
)
def test_malformed_resource_is_rejected_before_dedupe(sample: dict) -> None:
  composition = compose_stages(_graph())
  now = datetime(2026, 11, 2, 12, 0, tzinfo=UTC)
  runtime = StageRuntime(composition, "run-1", clock=lambda: now)
  for path in composition.instances:
    runtime.bind_owner(path, object())
  runtime.start()
  link = next(
    path for path, item in composition.instances.items() if item.type.group == "link"
  )
  edge = next(
    edge
    for edge in composition.edges
    if edge.source == link and edge.output == "resource"
  )
  seen: list[StageValue] = []
  runtime.bind(edge.target, edge.input, seen.append)

  def value(resource_sample: dict) -> StageValue:
    return StageValue(
      "resource",
      "resource-1",
      "run-1",
      link,
      now,
      StagePayload(
        "resource",
        {"backend": "robot-a", "reference": "robot", "sample": resource_sample},
      ),
      received_at=now,
    )

  with pytest.raises(ValueError, match="resource sample requires"):
    runtime.publish("resource", value(sample))
  assert runtime.diagnostics[-1].code == "malformed_resource"
  assert runtime.diagnostics[-1].node == link
  assert runtime.diagnostics[-1].event_id == "resource-1"
  assert seen == []
  assert (
    runtime.publish("resource", value({"text": "ready", "eos_robot_id": "robot-a"}))
    == 1
  )
  assert len(seen) == 1


def test_capped_evidence_keeps_latest_and_reports_gaps_until_stop() -> None:
  graph = _graph()
  context = next(
    node
    for _, node in _nodes(graph)
    if node["node_type"].split(".")[2].split("@")[0] == "builder"
  )
  context["inputs"]["observations"].update(priority=-2, frequency_hz=2)
  now = datetime(2026, 11, 2, 12, 0, tzinfo=UTC)
  monotonic = [10_000_000_000]
  runtime = StageRuntime(
    compose_stages(graph),
    "run-1",
    clock=lambda: now,
    monotonic_clock=lambda: monotonic[0],
  )
  for path in runtime.composition.instances:
    runtime.bind_owner(path, object())
  edge = next(
    edge for edge in runtime.composition.edges if edge.output == "observations"
  )
  seen: list[StageValue] = []
  runtime.bind(edge.target, edge.input, seen.append)
  warnings: list[int] = []
  runtime.on_overflow = lambda _edge, counters: warnings.append(counters.overflow)
  runtime.start()
  for index in range(4):
    runtime.publish(
      "observations", _value("observations", f"o-{index}", "run-1", edge.source, now)
    )
  assert [value.event_id for value in seen] == ["o-0"]
  assert runtime.edge_counters[edge].gaps == 2
  assert runtime.edge_counters[edge].overflow == 2
  assert warnings == [1]
  assert runtime.diagnostics[-1].code == "evidence_overflow"
  monotonic[0] += 500_000_000
  assert runtime.drain_ready() == 1
  assert [value.event_id for value in seen] == ["o-0", "o-3"]
  runtime.publish(
    "observations", _value("observations", "o-4", "run-1", edge.source, now)
  )
  runtime.stop()
  monotonic[0] += 1_000_000_000
  assert runtime.drain_ready() == 0
  assert [value.event_id for value in seen] == ["o-0", "o-3"]


@pytest.mark.parametrize("frequency", [5e-324, 1e300])
def test_extreme_positive_caps_use_elapsed_nanoseconds(frequency: float) -> None:
  graph = _graph()
  context = next(
    node
    for _, node in _nodes(graph)
    if node["node_type"].split(".")[2].split("@")[0] == "builder"
  )
  context["inputs"]["observations"]["frequency_hz"] = frequency
  now = datetime(2026, 11, 2, 12, 0, tzinfo=UTC)
  tick = [100]
  runtime = StageRuntime(
    compose_stages(graph), "run-1", clock=lambda: now, monotonic_clock=lambda: tick[0]
  )
  for path in runtime.composition.instances:
    runtime.bind_owner(path, object())
  edge = next(
    edge for edge in runtime.composition.edges if edge.output == "observations"
  )
  seen: list[str] = []
  runtime.bind(edge.target, edge.input, lambda value: seen.append(value.event_id))
  runtime.start()
  runtime.publish(
    "observations", _value("observations", "first", "run-1", edge.source, now)
  )
  runtime.publish(
    "observations", _value("observations", "second", "run-1", edge.source, now)
  )
  assert seen == ["first"]
  tick[0] = 99
  assert runtime.drain_ready() == 0
  tick[0] = 101
  assert runtime.drain_ready() == (1 if frequency > 1 else 0)


def test_weighted_evidence_delivery_is_fair_under_overload() -> None:
  graph = _graph()
  for _, node in _nodes(graph):
    group = node["node_type"].split(".")[2].split("@")[0]
    if group == "admission":
      node["inputs"]["resource"]["priority"] = -20
    if group == "builder":
      node["inputs"]["observations"]["priority"] = 19
  now = datetime(2026, 11, 2, 12, 0, tzinfo=UTC)
  runtime = StageRuntime(
    compose_stages(graph), "run-1", clock=lambda: now, auto_drain=False
  )
  for path in runtime.composition.instances:
    runtime.bind_owner(path, object())
  edges = {
    edge.output: edge
    for edge in runtime.composition.edges
    if edge.output in {"resource", "observations"}
  }
  seen: list[str] = []
  for edge in edges.values():
    runtime.bind(edge.target, edge.input, lambda value: seen.append(value.kind))
  runtime.start()
  for index in range(200):
    resource = StageValue(
      "resource",
      f"r-{index}",
      "run-1",
      edges["resource"].source,
      now,
      StagePayload(
        "resource",
        {
          "backend": "robot-a",
          "reference": "robot",
          "sample": {"text": "ready", "eos_robot_id": "robot-a"},
        },
      ),
      received_at=now,
    )
    runtime.publish("resource", resource)
    runtime.publish(
      "observations",
      _value("observations", f"o-{index}", "run-1", edges["observations"].source, now),
    )
    assert runtime.drain_ready() == 1
  assert seen.count("resource") > seen.count("observations") > 0
  assert len(seen) == 200


@pytest.mark.asyncio
async def test_owner_overload_is_counted_and_next_evidence_still_runs() -> None:
  now = datetime(2026, 11, 2, 12, 0, tzinfo=UTC)
  runtime = StageRuntime(
    compose_stages(_graph()), "run-1", clock=lambda: now, auto_drain=False
  )
  for path in runtime.composition.instances:
    runtime.bind_owner(path, object())
  edge = next(edge for edge in runtime.composition.edges if edge.output == "resource")
  accepted: list[str] = []
  runtime.bind(edge.target, edge.input, lambda value: accepted.append(value.event_id))

  async def admit(value: StageValue) -> None:
    if value.event_id == "full":
      raise RuntimeError("perception queue full")

  runtime.bind_async(edge.target, edge.input, admit)
  runtime.start()

  def resource(event_id: str) -> StageValue:
    return StageValue(
      "resource",
      event_id,
      "run-1",
      edge.source,
      now,
      StagePayload(
        "resource",
        {
          "backend": "robot-a",
          "reference": "robot",
          "sample": {"text": "ready", "eos_robot_id": "robot-a"},
        },
      ),
      received_at=now,
    )

  assert await runtime.publish_async("resource", resource("full")) == 0
  assert await runtime.drain_async() == 0
  assert runtime.diagnostics[-1].code == "owner_admission_failed"
  assert runtime.edge_counters[edge].overflow == 1
  assert runtime.edge_counters[edge].gaps == 1
  assert accepted == []
  assert await runtime.publish_async("resource", resource("next")) == 0
  assert await runtime.drain_async() == 1
  assert accepted == ["next"]


@pytest.mark.asyncio
async def test_async_stop_waits_for_owner_and_discards_pending() -> None:
  now = datetime(2026, 11, 2, 12, 0, tzinfo=UTC)
  runtime = StageRuntime(
    compose_stages(_graph()), "run-1", clock=lambda: now, auto_drain=False
  )
  for path in runtime.composition.instances:
    runtime.bind_owner(path, object())
  edge = next(edge for edge in runtime.composition.edges if edge.output == "resource")
  accepted: list[str] = []
  entered = asyncio.Event()
  release = asyncio.Event()
  runtime.bind(edge.target, edge.input, lambda value: accepted.append(value.event_id))

  async def admit(value: StageValue) -> None:
    entered.set()
    await release.wait()

  runtime.bind_async(edge.target, edge.input, admit)
  runtime.start()

  def resource(event_id: str) -> StageValue:
    return StageValue(
      "resource",
      event_id,
      "run-1",
      edge.source,
      now,
      StagePayload(
        "resource",
        {
          "backend": "robot-a",
          "reference": "robot",
          "sample": {"text": "ready", "eos_robot_id": "robot-a"},
        },
      ),
      received_at=now,
    )

  await runtime.publish_async("resource", resource("in-flight"))
  delivery = asyncio.create_task(runtime.drain_async())
  await entered.wait()
  await runtime.publish_async("resource", resource("queued"))
  stopping = asyncio.create_task(runtime.stop_async())
  await asyncio.sleep(0)
  assert not stopping.done()
  release.set()
  await delivery
  await stopping
  assert accepted == ["in-flight"]
  assert runtime.edge_counters[edge].gaps == 1
  assert runtime.diagnostics[-1].code == "evidence_stopped"
