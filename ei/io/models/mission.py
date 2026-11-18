"""Northbound mission commands and projections."""

from __future__ import annotations

from datetime import datetime
from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, RootModel, StrictInt

from ei.core.system.models.mission import (
  MissionDocumentProjection,
  MissionHistoryEntry,
  MissionPriorityBand,
  MissionState,
)
from ei.io.models.plan import PlanProjection


class _MissionCommandRequest(BaseModel):
  """Shared strict fields for one mission command."""

  model_config = ConfigDict(extra="forbid", str_strip_whitespace=True)

  correlation_id: str | None = Field(
    default=None, description="optional client ID for log cross-reference"
  )


class MissionCreateRequest(_MissionCommandRequest):
  """Admit one deterministic operator mission."""

  operation: Literal["create"]
  order: str = Field(min_length=1)
  priority_band: MissionPriorityBand = MissionPriorityBand.NORMAL


class _MissionMutationRequest(_MissionCommandRequest):
  """Shared compare-and-swap fields for a pending-mission mutation."""

  mission_id: str = Field(
    pattern=(
      r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-"
      r"[0-9a-f]{4}-[0-9a-f]{12}$"
    )
  )
  expected_revision: StrictInt = Field(ge=1)


class MissionUpdateRequest(_MissionMutationRequest):
  """Replace the order of one never-started pending mission."""

  operation: Literal["update"]
  order: str = Field(min_length=1)


class MissionPriorityRequest(_MissionMutationRequest):
  """Change the priority band of one pending mission."""

  operation: Literal["priority"]
  priority_band: MissionPriorityBand


class MissionCancelRequest(_MissionMutationRequest):
  """Cancel one pending mission."""

  operation: Literal["cancel"]


MissionCommand = Annotated[
  MissionCreateRequest
  | MissionUpdateRequest
  | MissionPriorityRequest
  | MissionCancelRequest,
  Field(discriminator="operation"),
]


class MissionCommandRequest(RootModel[MissionCommand]):
  """Strict operation-specific request accepted by the mission command route."""


class MissionCommandMission(BaseModel):
  """Accepted mission state sufficient for optimistic client updates."""

  mission_id: str
  order: str
  state: MissionState
  priority_band: MissionPriorityBand
  revision: int = Field(ge=1)


class MissionCommandAck(BaseModel):
  """Acknowledgement for one accepted mission command."""

  ok: Literal[True] = True
  operation: Literal["create", "update", "priority", "cancel"]
  correlation_id: str | None = None
  mission: MissionCommandMission


MissionCommandErrorCode = Literal[
  "invalid_json",
  "invalid_request",
  "queue_full",
  "mission_not_found",
  "mission_revision_conflict",
  "mission_not_pending",
  "mission_protected",
  "mission_started",
  "mission_mutation_failed",
  "storage_failure",
  "internal_failure",
  "unavailable",
]


class MissionCommandError(BaseModel):
  """Stable rejection envelope for one mission command."""

  ok: Literal[False] = False
  operation: Literal["create", "update", "priority", "cancel"] | None = None
  correlation_id: str | None = None
  error: MissionCommandErrorCode
  reason: str | None = None


class MissionTableEntry(BaseModel):
  """One mission as seen by mission-table observers (the TUI console)."""

  mission_id: str
  order: str
  mission_template_id: str | None = None
  mission_template_version: int | None = Field(default=None, ge=1)
  origin: Literal["operator", "self"]
  priority_band: MissionPriorityBand = MissionPriorityBand.NORMAL
  class_rank: int = Field(description="0 = operator, 1 = self-proposed (§3.9)")
  state: MissionState
  priority: float
  urgency: float
  value: float
  est_cost_s: float | None = None
  preempted_count: int = 0
  submitted_at: datetime
  started_at: datetime | None = None
  finished_at: datetime | None = None
  result: str | None = None
  error: str | None = None
  document: MissionDocumentProjection | None = None


class MissionTableSnapshot(BaseModel):
  """Live view of the mission table — the chain's mission-list stage.

  Ordered: the running mission first, then pending by table rank
  ``(class_rank, −priority_band, −score, submitted_at)``, then the most recent finished
  missions. ``history`` is the bounded, chronological live-session transition
  log. ``plan`` is the running mission's action list — the operator view of
  its in-flight plan (steps with per-action outcomes and semantic work) —
  whenever one is running, else ``None``.
  """

  missions: list[MissionTableEntry]
  history: list[MissionHistoryEntry] = Field(default_factory=list)
  plan: PlanProjection | None = None
