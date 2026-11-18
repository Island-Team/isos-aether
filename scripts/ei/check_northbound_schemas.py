#!/usr/bin/env python3
"""Check EI wire models against IsOS-owned northbound JSON Schemas.

The schema files are maintained under share/isos; this check never writes them.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from ei.io.models.invoke import (
  PrimitiveInvokeAck,
  PrimitiveInvokeError,
  PrimitiveInvokeRequest,
)
from ei.io.models.mission import (
  MissionCommandAck,
  MissionCommandError,
  MissionCommandRequest,
)
from ei.io.models.primitives import PrimitivesSnapshot
from ei.io.models.skills import (
  SkillCommandAck,
  SkillCommandError,
  SkillCommandRequest,
  SkillExecutionSnapshot,
  SkillsSnapshot,
  SkillWatchRequest,
)

ROOT = Path(__file__).resolve().parents[2]
DRAFT = "https://json-schema.org/draft/2020-12/schema"
CASES = {
  "mission_command_request.v1.json": (
    "ei.contracts.northbound.mission_command_request.v1",
    MissionCommandRequest,
  ),
  "mission_command_ack.v1.json": (
    "ei.contracts.northbound.mission_command_ack.v1",
    MissionCommandAck,
  ),
  "mission_command_error.v1.json": (
    "ei.contracts.northbound.mission_command_error.v1",
    MissionCommandError,
  ),
  "primitive_invoke_request.v1.json": (
    "ei.contracts.northbound.primitive_invoke_request.v1",
    PrimitiveInvokeRequest,
  ),
  "primitive_invoke_ack.v1.json": (
    "ei.contracts.northbound.primitive_invoke_ack.v1",
    PrimitiveInvokeAck,
  ),
  "primitive_invoke_error.v1.json": (
    "ei.contracts.northbound.primitive_invoke_error.v1",
    PrimitiveInvokeError,
  ),
  "primitives_snapshot.v1.json": (
    "ei.contracts.northbound.primitives_snapshot.v1",
    PrimitivesSnapshot,
  ),
  "skills_snapshot.json": (
    "ei.contracts.northbound.skills_snapshot.v2",
    SkillsSnapshot,
  ),
  "skill_command_request.v1.json": (
    "ei.contracts.northbound.skill_command_request.v1",
    SkillCommandRequest,
  ),
  "skill_command_ack.v1.json": (
    "ei.contracts.northbound.skill_command_ack.v1",
    SkillCommandAck,
  ),
  "skill_command_error.v1.json": (
    "ei.contracts.northbound.skill_command_error.v1",
    SkillCommandError,
  ),
  "skill_execution_snapshot.v1.json": (
    "ei.contracts.northbound.skill_execution_snapshot.v1",
    SkillExecutionSnapshot,
  ),
  "skill_watch_request.v1.json": (
    "ei.contracts.northbound.skill_watch_request.v1",
    SkillWatchRequest,
  ),
}


def main() -> None:
  target = ROOT / "contract/schemas/northbound"
  for name, (schema_id, model) in CASES.items():
    expected = {"$schema": DRAFT, "$id": schema_id, **model.model_json_schema()}
    actual = json.loads((target / name).read_text(encoding="utf-8"))
    if actual != expected:
      raise SystemExit(f"EI wire model differs from IsOS contract: {target / name}")


if __name__ == "__main__":
  main()
