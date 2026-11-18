"""Direct lifecycle tests for existing EI catalog stages."""

from __future__ import annotations

import hashlib
from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest
from ei.core.system.node_runtime import GatewayStage, PlannerStage, QueueStage


def test_gateway_admits_only_fresh_same_run_eos_resource() -> None:
  outputs: list[dict] = []
  gateway = GatewayStage(outputs.append, "run-1")
  record = {
    "owner": "eos",
    "run_id": "run-1",
    "source_time": datetime.now(UTC).isoformat(),
    "reference": "arm-1",
    "metadata": {"frame_id": "base_link"},
  }
  gateway.input({"port": "resource", "payload": record})
  assert outputs == [{"port": "resource", "payload": record}]
  for bad in (
    {**record, "owner": "ei"},
    {**record, "run_id": "old-run"},
    {**record, "source_time": (datetime.now(UTC) - timedelta(seconds=301)).isoformat()},
    {**record, "metadata": {"blob": "x" * 70_000}},
  ):
    with pytest.raises(ValueError):
      gateway.input({"port": "resource", "payload": bad})
  gateway.stop()
  with pytest.raises(ValueError, match="stopped"):
    gateway.input({"port": "resource", "payload": record})


def test_queue_selects_priority_rejects_duplicate_and_stops() -> None:
  outputs: list[dict] = []
  queue = QueueStage(outputs.append, capacity=2)
  queue.input(
    {
      "port": "intent",
      "payload": {"mission_id": "m1", "text": "inspect", "priority": 1},
    }
  )
  queue.input(
    {"port": "intent", "payload": {"mission_id": "m2", "text": "stop", "priority": 10}}
  )
  assert [item["payload"]["mission_id"] for item in outputs] == ["m1", "m2"]
  with pytest.raises(ValueError, match="Duplicate"):
    queue.input({"port": "intent", "payload": {"mission_id": "m1", "text": "repeat"}})
  with pytest.raises(ValueError, match="capacity"):
    queue.input({"port": "intent", "payload": {"mission_id": "m3", "text": "overflow"}})
  queue.input({"port": "result", "payload": {"mission_id": "m2", "state": "completed"}})
  assert outputs[-1]["payload"]["mission_id"] == "m1"
  queue.stop()
  with pytest.raises(ValueError, match="stopped"):
    queue.input({"port": "intent", "payload": {"mission_id": "m4", "text": "later"}})


def test_planner_abstains_without_provider_and_bounds_provider_result(
  tmp_path: Path,
) -> None:
  outputs: list[dict] = []
  selected = {"mission_id": "m1"}
  snapshot = {"text": "one observed cup"}
  planner = PlannerStage(outputs.append)
  planner.input({"port": "selected", "payload": selected})
  planner.input({"port": "snapshot", "payload": snapshot})
  assert outputs == [
    {
      "port": "plan_state",
      "payload": {
        "state": "abstained",
        "reason": "missing_model",
      },
    }
  ]
  expiry = (datetime.now(UTC) + timedelta(seconds=10)).isoformat()
  result = {
    "plan_id": "p1",
    "steps": ["inspect cup"],
    "proposal": {
      "backend": "mock",
      "device": "arm",
      "primitive": "inspect",
      "version": "1",
      "arguments": {},
      "target_id": "arm-1",
      "expires_at": expiry,
    },
  }
  model = tmp_path / "planner-model.bin"
  model.write_bytes(b"fixture-model")
  verified = (str(model), hashlib.sha256(model.read_bytes()).hexdigest())
  with pytest.raises(ValueError, match="verified model"):
    PlannerStage(outputs.append, provider=lambda mission, context: result)
  ready = PlannerStage(
    outputs.append, provider=lambda mission, context: result, verified_model=verified
  )
  ready.input({"port": "selected", "payload": selected})
  ready.input({"port": "snapshot", "payload": snapshot})
  assert [item["port"] for item in outputs[-2:]] == ["proposal", "plan_state"]
  over = PlannerStage(
    outputs.append,
    provider=lambda mission, context: {
      **result,
      "steps": ["step"] * 9,
    },
    verified_model=verified,
  )
  over.input({"port": "selected", "payload": selected})
  with pytest.raises(ValueError, match="step bound"):
    over.input({"port": "snapshot", "payload": snapshot})
  ready.stop()
  with pytest.raises(ValueError, match="stopped"):
    ready.input({"port": "snapshot", "payload": snapshot})
  model.write_bytes(b"changed")
  with pytest.raises(ValueError, match="changed"):
    over.input({"port": "snapshot", "payload": snapshot})
