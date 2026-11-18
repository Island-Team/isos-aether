"""Mission records and live history retained by EI cognition."""

from __future__ import annotations

from datetime import datetime
from enum import StrEnum
from typing import Any, Literal

from pydantic import BaseModel, Field


class MissionState(StrEnum):
  """Lifecycle of one admitted order."""

  PENDING = "pending"
  RUNNING = "running"
  SUCCEEDED = "succeeded"
  FAILED = "failed"
  CANCELLED = "cancelled"


class MissionPriorityBand(StrEnum):
  """Operator-selected priority band, independent of the dynamic score."""

  LOW = "low"
  NORMAL = "normal"
  HIGH = "high"
  URGENT = "urgent"


class MissionRecord(BaseModel):
  """One admitted mission and its terminal outcome.

  The record is the audit trail of the main chain: what was ordered, when it
  ran, and how it ended. ``result`` carries the planner's final answer on
  success; ``error`` the failure reason otherwise.
  """

  id: str
  order: str
  source: str
  state: MissionState = MissionState.PENDING
  submitted_at: datetime
  started_at: datetime | None = None
  finished_at: datetime | None = None
  result: str | None = None
  error: str | None = None
  correlation_id: str | None = None
  origin: Literal["operator", "self"] = "operator"
  priority_band: MissionPriorityBand = MissionPriorityBand.NORMAL
  priority: float = Field(default=0.0, description="mission-table score (§3.9)")
  urgency: float = Field(default=0.5, ge=0, le=1, description="proposer urgency hint")
  value: float = Field(default=0.5, ge=0, le=1, description="proposer value hint")
  preempted_count: int = Field(
    default=0, ge=0, description="step-boundary preemptions so far"
  )
  est_cost_s: float | None = Field(
    default=None,
    description="estimated duration from recalled plans; None = default (§3.9)",
  )
  mission_template_id: str | None = None
  mission_template_version: int | None = Field(default=None, ge=1)


class MissionHistoryEntry(BaseModel):
  """One live-session mission summary for operators.

  A mission owns exactly one entry.  The queue refreshes its state and
  timestamp as the mission progresses instead of appending lifecycle events.
  ``steps`` is the mission document's current executable work, so completed
  missions retain their action outcomes in the northbound history view.
  """

  mission_id: str
  order: str
  state: MissionState
  priority_band: MissionPriorityBand = MissionPriorityBand.NORMAL
  timestamp: datetime
  detail: str | None = None
  revision: int = Field(default=1, ge=1)
  completeness: float = Field(default=0.0, ge=0.0, le=1.0)
  steps: list[MissionHistoryStep] = Field(default_factory=list)
  document: MissionDocumentProjection | None = None


class MissionHistoryAction(BaseModel):
  """One action in a northbound mission-history record."""

  index: int = Field(ge=1)
  robot_id: str
  device_id: str | None = None
  intent: str = ""
  action: str
  command: str = ""
  arguments: dict[str, Any] = Field(default_factory=dict)
  state: str
  result: Any | None = None
  reason: str | None = None


class MissionHistoryStep(BaseModel):
  """One ordered step in a northbound mission-history record."""

  index: int = Field(ge=1)
  id: str | None = None
  sentence: str = ""
  state: str
  result: Any | None = None
  reason: str | None = None
  required: bool = False
  after: list[str] = Field(default_factory=list)
  terminal: bool = False
  evidence: str | None = None
  actions: list[MissionHistoryAction] = Field(default_factory=list)


class MissionPlanProjection(BaseModel):
  """One plan node; nested subplans retain their own steps and status."""

  plan_id: str
  title: str
  revision: int
  state: str
  result: Any | None = None
  reason: str | None = None
  completeness: float = Field(ge=0.0, le=1.0)
  steps: list[MissionHistoryStep] = Field(default_factory=list)
  subplans: list[MissionPlanProjection] = Field(default_factory=list)


class MissionDocumentProjection(BaseModel):
  """Authoritative model-owned mission and its complete plan tree."""

  revision: int
  completeness: float = Field(ge=0.0, le=1.0)
  plan: MissionPlanProjection | None = None
