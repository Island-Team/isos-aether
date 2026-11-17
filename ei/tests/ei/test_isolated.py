"""Direct fixture tests for EI isolated library nodes."""

from __future__ import annotations

import hashlib
from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest
from ei.core.system.isolated import DATA_TYPES, IsolatedNode, validate_value
from ei.core.system.node_runtime import AdmissionStage, PlannerStage

NOW = datetime(2026, 1, 1, tzinfo=UTC)


def _verified(
  tmp_path: Path, resource_id: str = "fixture-model"
) -> tuple[str, str, str]:
  path = tmp_path / "fixture-model.bin"
  path.write_bytes(b"verified-model-fixture")
  return resource_id, str(path), hashlib.sha256(path.read_bytes()).hexdigest()


def _base() -> dict:
  return {
    "source_id": "fixture-camera",
    "source_time": NOW.isoformat(),
    "sequence": 4,
    "frame_id": "base_link",
  }


def _goal() -> dict:
  return {
    **_base(),
    "goal_id": "goal-1",
    "target_id": "arm-1",
    "confidence": 0.8,
    "expires_at": (NOW + timedelta(seconds=2)).isoformat(),
    "model_version": "fixture-v1",
  }


def _motion() -> dict:
  return {
    **_base(),
    "proposal_id": "proposal-1",
    "target_id": "arm-1",
    "resource_id": "joint-1",
    "horizon_ms": 100,
    "delta_rad": [0.1],
    "expires_at": (NOW + timedelta(seconds=2)).isoformat(),
    "model_version": "fixture-v1",
  }


def _vla_input() -> dict:
  return {
    **_base(),
    "image": {
      "resource_id": "camera-frame-4",
      "digest": "a" * 64,
      "source_time": NOW.isoformat(),
      "frame_id": "base_link",
      "width_px": 640,
      "height_px": 480,
      "encoding": "jpeg",
      "calibration_id": "camera-cal-1",
    },
    "language": {"text": "point to cup", "language": "en"},
    "robot_state": {
      "source_time": NOW.isoformat(),
      "frame_id": "base_link",
      "joint_names": ["joint-1"],
      "position_rad": [0.1],
      "model_version": "robot-v1",
    },
  }


def test_each_descriptor_has_a_bounded_schema_and_valid_example() -> None:
  examples = {
    "language_instruction@v0.2": {**_base(), "text": "point to cup", "language": "en"},
    "vla_input@v0.2": _vla_input(),
    "grounded_goal@v0.2": _goal(),
    "motion_proposal@v0.2": _motion(),
    "prediction@v0.2": {
      **_base(),
      "target_id": "arm-1",
      "predicted_value": 0.1,
      "unit": "rad",
      "uncertainty_rad": 0.25,
      "shadow_mode": True,
    },
    "memory_reference@v0.2": {
      **_base(),
      "record_id": "m1",
      "policy_id": "public",
      "digest": "a" * 64,
    },
  }
  assert {item["id"] for item in DATA_TYPES} == set(examples)
  for item in DATA_TYPES:
    assert item["codec"] == "json_v1"
    assert item["max_payload_bytes"] == 65_536
    validate_value(item["id"], examples[item["id"]], now=NOW)


def test_value_rejects_missing_identity_stale_oversize_and_wrong_unit() -> None:
  value = _motion()
  for bad in (
    {key: item for key, item in value.items() if key != "source_id"},
    {**value, "source_time": (NOW - timedelta(seconds=6)).isoformat()},
    {**value, "proposal_id": "x" * 70_000},
    {**value, "delta_rad": [0.5]},
  ):
    with pytest.raises(ValueError):
      validate_value("motion_proposal@v0.2", bad, now=NOW)
  with pytest.raises(ValueError):
    validate_value("prediction@v0.2", {**_base(), "unit": "degree"}, now=NOW)


def test_vla_requires_all_fresh_matching_modalities() -> None:
  value = _vla_input()
  for bad in (
    {key: item for key, item in value.items() if key != "image"},
    {**value, "language": {"text": "point to cup"}},
    {**value, "robot_state": {**value["robot_state"], "position_rad": []}},
    {**value, "image": {**value["image"], "frame_id": "camera"}},
    {
      **value,
      "image": {
        **value["image"],
        "source_time": (NOW - timedelta(seconds=6)).isoformat(),
      },
    },
    {
      **value,
      "robot_state": {
        **value["robot_state"],
        "source_time": (NOW - timedelta(seconds=6)).isoformat(),
      },
    },
    {**value, "image": {**value["image"], "width_px": 0}},
    {**value, "image": {**value["image"], "encoding": "png"}},
    {
      **value,
      "robot_state": {**value["robot_state"], "joint_names": ["joint-1", "joint-2"]},
    },
    {**value, "robot_state": {**value["robot_state"], "position_rad": [float("nan")]}},
    {**value, "language": {"text": "x" * 70_000, "language": "en"}},
  ):
    with pytest.raises((ValueError, TypeError)):
      validate_value("vla_input@v0.2", bad, now=NOW)


@pytest.mark.parametrize(
  "node_type",
  [
    "ei.cognition.policy.system1_python@v0.2",
    "ei.cognition.policy.system1_native@v0.2",
  ],
)
def test_system1_requires_provider_and_conforms_to_one_proposal_schema(
  node_type: str, tmp_path: Path
) -> None:
  unavailable = IsolatedNode(
    node_type, {"model_resource_id": "fixture-model"}, "run-1", now=lambda: NOW
  )
  assert unavailable.health()["state"] == "unavailable"
  with pytest.raises(ValueError, match="unavailable"):
    unavailable.input("input", "grounded_goal@v0.2", _goal())
  with pytest.raises(ValueError, match="verified resource"):
    IsolatedNode(
      node_type,
      {"model_resource_id": "fixture-model"},
      "run-1",
      provider=lambda resource, value: _motion(),
      now=lambda: NOW,
    )
  verified = _verified(tmp_path)
  node = IsolatedNode(
    node_type,
    {"model_resource_id": "fixture-model", "frame_id": "base_link"},
    "run-1",
    provider=lambda resource, value: _motion(),
    verified_model=verified,
    now=lambda: NOW,
  )
  assert node.health()["state"] == "ready"
  assert node.health()["generation"] == 0
  assert (
    node.input("input", "grounded_goal@v0.2", _goal())[0]["data_type"]
    == "motion_proposal@v0.2"
  )
  assert node.health()["generation"] == 1
  assert node.read() is not None
  with pytest.raises(ValueError, match="frame"):
    node.input("input", "grounded_goal@v0.2", {**_goal(), "frame_id": "camera"})
  assert node.stop()
  with pytest.raises(ValueError, match="stopped"):
    node.input("input", "grounded_goal@v0.2", _goal())
  Path(verified[1]).write_bytes(b"changed")
  with pytest.raises(ValueError, match="changed"):
    IsolatedNode(
      node_type,
      {"model_resource_id": "fixture-model"},
      "run-2",
      provider=lambda resource, value: _motion(),
      verified_model=verified,
      now=lambda: NOW,
    )


def test_model_rechecks_bytes_and_output_limit(tmp_path: Path) -> None:
  verified = _verified(tmp_path)
  node_type = "ei.cognition.policy.system1_python@v0.2"
  node = IsolatedNode(
    node_type,
    {"model_resource_id": "fixture-model", "max_output_bytes": 30},
    "run-1",
    provider=lambda resource, value: _motion(),
    verified_model=verified,
    now=lambda: NOW,
  )
  with pytest.raises(ValueError, match="output exceeds"):
    node.input("input", "grounded_goal@v0.2", _goal())
  Path(verified[1]).write_bytes(b"changed")
  with pytest.raises(ValueError, match="changed"):
    node.input("input", "grounded_goal@v0.2", _goal())


def test_vla_rejects_invalid_model_output_and_accepts_grounded_goal(
  tmp_path: Path,
) -> None:
  node_type = "ei.cognition.policy.vla@v0.2"
  verified = _verified(tmp_path, "vla-fixture")
  instruction = _vla_input()
  node = IsolatedNode(
    node_type,
    {"model_resource_id": "vla-fixture"},
    "run-1",
    provider=lambda resource, value: {**_goal(), "confidence": 2},
    verified_model=verified,
    now=lambda: NOW,
  )
  with pytest.raises(ValueError):
    node.input("input", "vla_input@v0.2", instruction)
  valid = IsolatedNode(
    node_type,
    {"model_resource_id": "vla-fixture"},
    "run-1",
    provider=lambda resource, value: _goal(),
    verified_model=verified,
    now=lambda: NOW,
  )
  assert (
    valid.input("input", "vla_input@v0.2", instruction)[0]["payload"]["goal_id"]
    == "goal-1"
  )


def test_predictor_remains_shadow_and_memory_enforces_policy() -> None:
  with pytest.raises(ValueError, match="shadow_mode"):
    IsolatedNode("ei.cognition.world.predictor@v0.2", {"shadow_mode": False}, "run-1")
  predictor = IsolatedNode(
    "ei.cognition.world.predictor@v0.2", {}, "run-1", now=lambda: NOW
  )
  prediction = predictor.input("candidate", "motion_proposal@v0.2", _motion())[0]
  assert prediction["payload"]["shadow_mode"] is True
  assert prediction["payload"]["uncertainty_rad"] > 0
  shared: dict = {}
  record = {**_base(), "record_id": "m1", "policy_id": "public", "digest": "a" * 64}
  remember = IsolatedNode(
    "ei.cognition.memory.remember@v0.2",
    {"max_records": 1},
    "run-1",
    memory_store=shared,
    now=lambda: NOW,
  )
  recall = IsolatedNode(
    "ei.cognition.memory.recall@v0.2", {}, "run-1", memory_store=shared, now=lambda: NOW
  )
  remember.input("record", "memory_reference@v0.2", record)
  with pytest.raises(ValueError, match="duplicate"):
    remember.input("record", "memory_reference@v0.2", record)
  assert recall.input("query", "memory_reference@v0.2", record)[0]["payload"] == record
  with pytest.raises(ValueError, match="policy"):
    recall.input("query", "memory_reference@v0.2", {**record, "policy_id": "private"})
  remember.stop()
  assert recall.read() is not None


def test_existing_planner_abstains_and_admission_requires_provenance() -> None:
  outputs: list[dict] = []
  planner = PlannerStage(outputs.append)
  planner.input({"port": "selected", "payload": {"mission_id": "m1"}})
  planner.input({"port": "snapshot", "payload": {"text": "fixture"}})
  assert outputs == [
    {
      "port": "plan_state",
      "payload": {
        "state": "abstained",
        "reason": "missing_model",
      },
    }
  ]
  assert not any(item["port"] == "proposal" for item in outputs)
  admission = AdmissionStage(outputs.append)
  with pytest.raises(ValueError, match="attributed"):
    admission.input({"port": "resource", "payload": "unattributed"})
  with pytest.raises(ValueError, match="kind, source and text"):
    admission.input({"port": "resource", "payload": {"text": "unknown"}})
  admission.stop()
  with pytest.raises(ValueError, match="stopped"):
    admission.input(
      {
        "port": "resource",
        "payload": {
          "kind": "hardware",
          "source": "eos",
          "text": "fixture",
        },
      }
    )
