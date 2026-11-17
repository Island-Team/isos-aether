"""Direct, run-owned EI cognition nodes for the isolated node library.

These nodes have no graph routes. A caller supplies one exact typed value and
receives bounded typed outputs; model providers are explicitly injected only
after their package has been verified by the owner.
"""

from __future__ import annotations

import hashlib
import json
import time
from collections.abc import Callable
from datetime import UTC, datetime, timedelta
from math import isfinite
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

MAX_BYTES = 65_536
MAX_AGE_MS = 5_000


def _field(type_name: str, **extra: Any) -> dict[str, Any]:
  return {"type": type_name, **extra}


_IDENTITY = {
  "source_id": _field("string", minLength=1, maxLength=128),
  "source_time": _field("string", format="date-time"),
  "sequence": _field("integer", minimum=0),
  "frame_id": _field("string", minLength=1, maxLength=128),
}


def _object(properties: dict[str, Any], required: list[str]) -> dict[str, Any]:
  return {
    "type": "object",
    "properties": properties,
    "required": required,
    "additionalProperties": False,
  }


_NAME = _field("string", minLength=1, maxLength=128)
_TIME = _field("string", format="date-time")
_DIGEST = {"type": "string", "pattern": "^[0-9a-f]{64}$"}
_TEXT = _field("string", minLength=1, maxLength=4096)


def _descriptor(
  identifier: str, purpose: str, properties: dict[str, Any], required: list[str]
) -> dict[str, Any]:
  return {
    "id": identifier,
    "version": "0.2",
    "codec": "json_v1",
    "media_type": "application/json",
    "max_payload_bytes": MAX_BYTES,
    "purpose": purpose,
    "schema": {
      "type": "object",
      "properties": {**_IDENTITY, **properties},
      "required": [*_IDENTITY, *required],
      "additionalProperties": False,
    },
  }


DATA_TYPES = [
  _descriptor(
    "language_instruction@v0.2",
    "Attributed operator instruction.",
    {
      "text": _field("string", minLength=1, maxLength=4096),
      "language": _field("string", minLength=2, maxLength=16),
    },
    ["text", "language"],
  ),
  _descriptor(
    "vla_input@v0.2",
    "Attributed image reference, language instruction and robot joint state for VLA grounding.",
    {
      "image": _object(
        {
          "resource_id": _NAME,
          "digest": _DIGEST,
          "source_time": _TIME,
          "frame_id": _NAME,
          "width_px": _field("integer", minimum=1, maximum=8192),
          "height_px": _field("integer", minimum=1, maximum=8192),
          "encoding": {"enum": ["jpeg", "rgb8"]},
          "calibration_id": _NAME,
        },
        [
          "resource_id",
          "digest",
          "source_time",
          "frame_id",
          "width_px",
          "height_px",
          "encoding",
          "calibration_id",
        ],
      ),
      "language": _object(
        {"text": _TEXT, "language": _field("string", minLength=2, maxLength=16)},
        ["text", "language"],
      ),
      "robot_state": _object(
        {
          "source_time": _TIME,
          "frame_id": _NAME,
          "joint_names": {
            "type": "array",
            "minItems": 1,
            "maxItems": 64,
            "uniqueItems": True,
            "items": _NAME,
          },
          "position_rad": {
            "type": "array",
            "minItems": 1,
            "maxItems": 64,
            "items": _field("number", minimum=-12.57, maximum=12.57),
          },
          "model_version": _NAME,
        },
        ["source_time", "frame_id", "joint_names", "position_rad", "model_version"],
      ),
    },
    ["image", "language", "robot_state"],
  ),
  _descriptor(
    "grounded_goal@v0.2",
    "Model-grounded goal, never an actuator command.",
    {
      "goal_id": _field("string", minLength=1, maxLength=128),
      "target_id": _field("string", minLength=1, maxLength=128),
      "confidence": _field("number", minimum=0, maximum=1),
      "expires_at": _field("string", format="date-time"),
      "model_version": _field("string", minLength=1, maxLength=128),
    },
    ["goal_id", "target_id", "confidence", "expires_at", "model_version"],
  ),
  _descriptor(
    "motion_proposal@v0.2",
    "Bounded short-horizon motion proposal.",
    {
      "proposal_id": _field("string", minLength=1, maxLength=128),
      "target_id": _field("string", minLength=1, maxLength=128),
      "resource_id": _field("string", minLength=1, maxLength=128),
      "horizon_ms": _field("integer", minimum=1, maximum=1000),
      "delta_rad": {
        "type": "array",
        "minItems": 1,
        "maxItems": 32,
        "items": _field("number", minimum=-0.25, maximum=0.25),
      },
      "expires_at": _field("string", format="date-time"),
      "model_version": _field("string", minLength=1, maxLength=128),
    },
    [
      "proposal_id",
      "target_id",
      "resource_id",
      "horizon_ms",
      "delta_rad",
      "expires_at",
      "model_version",
    ],
  ),
  _descriptor(
    "prediction@v0.2",
    "Shadow prediction with explicit uncertainty.",
    {
      "target_id": _field("string", minLength=1, maxLength=128),
      "predicted_value": _field("number"),
      "unit": {"const": "rad"},
      "uncertainty_rad": _field("number", minimum=0),
      "shadow_mode": {"const": True},
    },
    ["target_id", "predicted_value", "unit", "uncertainty_rad", "shadow_mode"],
  ),
  _descriptor(
    "memory_reference@v0.2",
    "Bounded cognitive memory reference.",
    {
      "record_id": _field("string", minLength=1, maxLength=128),
      "policy_id": _field("string", minLength=1, maxLength=128),
      "digest": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
    },
    ["record_id", "policy_id", "digest"],
  ),
]


BY_ID = {item["id"]: item for item in DATA_TYPES}

PORTS = {
  "ei.cognition.policy.system1_python@v0.2": (
    "policy",
    "input",
    "grounded_goal@v0.2",
    "motion_proposal@v0.2",
  ),
  "ei.cognition.policy.system1_native@v0.2": (
    "policy",
    "input",
    "grounded_goal@v0.2",
    "motion_proposal@v0.2",
  ),
  "ei.cognition.policy.vla@v0.2": (
    "policy",
    "input",
    "vla_input@v0.2",
    "grounded_goal@v0.2",
  ),
  "ei.cognition.world.predictor@v0.2": (
    "world",
    "candidate",
    "motion_proposal@v0.2",
    "prediction@v0.2",
  ),
  "ei.cognition.memory.recall@v0.2": (
    "memory",
    "query",
    "memory_reference@v0.2",
    "memory_reference@v0.2",
  ),
  "ei.cognition.memory.remember@v0.2": (
    "memory",
    "record",
    "memory_reference@v0.2",
    "memory_reference@v0.2",
  ),
}


def _time(value: str) -> datetime:
  parsed = datetime.fromisoformat(value)
  if parsed.tzinfo is None:
    raise ValueError("source time must include UTC offset")
  return parsed.astimezone(UTC)


def validate_value(
  identifier: str,
  value: Any,
  *,
  now: datetime | None = None,
  max_age_ms: int = MAX_AGE_MS,
) -> dict[str, Any]:
  """Validate exact JSON type, finite numbers, byte bound, frame and age."""
  descriptor = BY_ID.get(identifier)
  if descriptor is None:
    raise ValueError("unknown EI data type")
  if not isinstance(value, dict):
    raise TypeError("EI input must be an object")
  raw = json.dumps(value, allow_nan=False, separators=(",", ":")).encode()
  if len(raw) > descriptor["max_payload_bytes"]:
    raise ValueError("EI input exceeds data type byte bound")
  errors = list(Draft202012Validator(descriptor["schema"]).iter_errors(value))
  if errors:
    raise ValueError(f"invalid {identifier}: {errors[0].message}")
  current = now or datetime.now(UTC)
  source_time = _time(value["source_time"])
  age = current - source_time
  if age < timedelta(seconds=-1) or age > timedelta(milliseconds=max_age_ms):
    raise ValueError("EI input source time is stale or in the future")
  for item in value.values():
    if type(item) is float and not isfinite(item):
      raise ValueError("EI input contains a nonfinite number")
  if "expires_at" in value and _time(value["expires_at"]) <= current:
    raise ValueError("EI proposal or goal has expired")
  if identifier == "vla_input@v0.2":
    for modality in ("image", "robot_state"):
      stamp = _time(value[modality]["source_time"])
      age = current - stamp
      if age < timedelta(seconds=-1) or age > timedelta(milliseconds=max_age_ms):
        raise ValueError(f"EI {modality} source time is stale or in the future")
      if value[modality]["frame_id"] != value["frame_id"]:
        raise ValueError(f"EI {modality} coordinate frame mismatch")
    robot = value["robot_state"]
    if len(robot["joint_names"]) != len(robot["position_rad"]):
      raise ValueError("EI robot joint names and positions differ in length")
  return value


ModelProvider = Callable[[str, dict[str, Any]], dict[str, Any]]


class IsolatedNode:
  """One directly invoked EI node; a missing model is unavailable."""

  MODEL_TYPES = frozenset(
    {
      "ei.cognition.policy.system1_python@v0.2",
      "ei.cognition.policy.system1_native@v0.2",
      "ei.cognition.policy.vla@v0.2",
    }
  )
  TYPES = MODEL_TYPES | {
    "ei.cognition.world.predictor@v0.2",
    "ei.cognition.memory.recall@v0.2",
    "ei.cognition.memory.remember@v0.2",
  }

  def __init__(
    self,
    node_type: str,
    settings: dict[str, Any],
    run_id: str,
    *,
    provider: ModelProvider | None = None,
    verified_model: tuple[str, str, str] | None = None,
    memory_store: dict[str, dict[str, Any]] | None = None,
    now: Callable[[], datetime] | None = None,
  ) -> None:
    if node_type not in self.TYPES or not run_id:
      raise ValueError("unknown EI node or missing run identity")
    allowed = {
      "max_input_age_ms",
      "shadow_mode",
      "model_resource_id",
      "max_records",
      "frame_id",
      "deadline_ms",
      "max_output_bytes",
      "max_rate_hz",
    }
    if not isinstance(settings, dict) or set(settings) - allowed:
      raise ValueError("unknown EI node setting")
    age = settings.get("max_input_age_ms", MAX_AGE_MS)
    if type(age) is not int or not 1 <= age <= 60_000:
      raise ValueError("invalid max_input_age_ms")
    records = settings.get("max_records", 32)
    if type(records) is not int or not 1 <= records <= 256:
      raise ValueError("invalid max_records")
    shadow = settings.get("shadow_mode", True)
    if type(shadow) is not bool or (
      node_type.endswith("predictor@v0.2") and not shadow
    ):
      raise ValueError("predictor requires shadow_mode:true")
    resource = settings.get("model_resource_id")
    frame = settings.get("frame_id")
    if frame is not None and (not isinstance(frame, str) or not 1 <= len(frame) <= 128):
      raise ValueError("invalid frame_id")
    if node_type in self.MODEL_TYPES and (
      not isinstance(resource, str) or not resource or len(resource) > 128
    ):
      raise ValueError("model_resource_id is required")
    if node_type not in self.MODEL_TYPES and resource is not None:
      raise ValueError("model_resource_id is unsupported")
    deadline = settings.get("deadline_ms", 1000)
    output_bytes = settings.get("max_output_bytes", MAX_BYTES)
    rate = settings.get("max_rate_hz", 20)
    if type(deadline) is not int or not 1 <= deadline <= 30_000:
      raise ValueError("invalid deadline_ms")
    if type(output_bytes) is not int or not 1 <= output_bytes <= MAX_BYTES:
      raise ValueError("invalid max_output_bytes")
    if type(rate) not in {int, float} or not isfinite(rate) or not 0 < rate <= 100:
      raise ValueError("invalid max_rate_hz")
    if provider is not None and node_type in self.MODEL_TYPES:
      if verified_model is None or len(verified_model) != 3:
        raise ValueError("model provider requires exact verified resource")
      verified_id, model_path, digest = verified_model
      if verified_id != resource:
        raise ValueError("verified model resource ID mismatch")
      if (
        not isinstance(model_path, str)
        or not isinstance(digest, str)
        or len(digest) != 64
      ):
        raise ValueError("invalid verified model reference")
      path = Path(model_path)
      if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise ValueError("verified model bytes changed or missing")
    self.node_type = node_type
    self.settings = settings.copy()
    self.run_id = run_id
    self._provider = provider
    self._verified_model = verified_model
    self._now = now or (lambda: datetime.now(UTC))
    self._stopped = False
    self._latest: dict[str, Any] | None = None
    self._generation = 0
    self._last_input_at = 0.0
    self._records: dict[str, dict[str, Any]] = (
      memory_store if memory_store is not None else {}
    )

  def health(self) -> dict[str, Any]:
    state = (
      "stopped"
      if self._stopped
      else (
        "unavailable"
        if self.node_type in self.MODEL_TYPES and self._provider is None
        else "ready"
      )
    )
    return {
      "state": state,
      "reason": "missing_model" if state == "unavailable" else None,
      "run_id": self.run_id,
      "source_time": self._now().isoformat(),
      "generation": self._generation,
    }

  def read(self) -> dict[str, Any] | None:
    return self._latest

  def input(
    self, port: str, data_type: str, value: dict[str, Any]
  ) -> list[dict[str, Any]]:
    if self._stopped:
      raise ValueError("EI node is stopped")
    if self.health()["state"] != "ready":
      raise ValueError("EI model provider is unavailable")
    if self._verified_model is not None:
      path = Path(self._verified_model[1])
      if (
        not path.is_file()
        or hashlib.sha256(path.read_bytes()).hexdigest() != self._verified_model[2]
      ):
        raise ValueError("verified model bytes changed or missing")
    started = time.monotonic()
    now = self._now()
    value = validate_value(
      data_type,
      value,
      now=now,
      max_age_ms=self.settings.get("max_input_age_ms", MAX_AGE_MS),
    )
    if "frame_id" in self.settings and value["frame_id"] != self.settings["frame_id"]:
      raise ValueError("EI input coordinate frame mismatch")
    if self.node_type in self.MODEL_TYPES:
      if self._last_input_at and started - self._last_input_at < 1 / self.settings.get(
        "max_rate_hz", 20
      ):
        raise ValueError("EI node rate limit exceeded")
      expected = (
        "vla_input@v0.2"
        if self.node_type.endswith("vla@v0.2")
        else "grounded_goal@v0.2"
      )
      output_type = (
        "grounded_goal@v0.2"
        if self.node_type.endswith("vla@v0.2")
        else "motion_proposal@v0.2"
      )
      if port != "input" or data_type != expected:
        raise ValueError("EI node input port or type mismatch")
      assert self._provider is not None
      output = self._provider(self.settings["model_resource_id"], value)
      if (time.monotonic() - started) * 1000 > self.settings.get("deadline_ms", 1000):
        raise ValueError("EI model deadline exceeded")
      if len(
        json.dumps(output, allow_nan=False, separators=(",", ":")).encode()
      ) > self.settings.get("max_output_bytes", MAX_BYTES):
        raise ValueError("EI model output exceeds byte bound")
      validate_value(output_type, output, now=now)
    elif self.node_type.endswith("predictor@v0.2"):
      if port != "candidate" or data_type != "motion_proposal@v0.2":
        raise ValueError("EI predictor input port or type mismatch")
      output_type = "prediction@v0.2"
      output = {
        **{key: value[key] for key in _IDENTITY},
        "target_id": value["target_id"],
        "predicted_value": value["delta_rad"][0],
        "unit": "rad",
        "uncertainty_rad": 0.25,
        "shadow_mode": True,
      }
      validate_value(output_type, output, now=now)
    elif self.node_type.endswith("remember@v0.2"):
      if port != "record" or data_type != "memory_reference@v0.2":
        raise ValueError("EI memory input port or type mismatch")
      if value["record_id"] in self._records or len(self._records) >= self.settings.get(
        "max_records", 32
      ):
        raise ValueError("duplicate memory ID or exhausted budget")
      self._records[value["record_id"]] = value.copy()
      output_type, output = data_type, value.copy()
    else:
      if port != "query" or data_type != "memory_reference@v0.2":
        raise ValueError("EI memory input port or type mismatch")
      output_type = data_type
      record = self._records.get(value["record_id"])
      if record is None or record["policy_id"] != value["policy_id"]:
        raise ValueError("memory reference unavailable or policy denied")
      output = record.copy()
    result = {"port": "output", "data_type": output_type, "payload": output}
    self._latest = result
    self._generation += 1
    self._last_input_at = started
    return [result]

  def stop(self) -> bool:
    self._stopped = True
    return True
