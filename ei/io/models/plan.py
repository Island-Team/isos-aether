"""Northbound projections of EI plans."""

from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel, Field

from ei.core.system.models.plan import DecisionGraph, PlanStepStatus, StepOutcome


class ActionProjection(BaseModel):
  """One motion of the operator plan view — a projection, never a copy."""

  index: int
  robot_id: str = "default"
  device_id: str | None = None
  intent: str = ""
  action: str | None = None
  action_kind: Literal["primitive", "skill"] = "primitive"
  version: str | None = None
  request_id: str | None = None
  execution_id: str | None = None
  execution_status: str | None = None
  command: str = ""
  arguments: dict[str, Any] = Field(
    default_factory=dict,
    description="primitive parameters sent with this action",
  )
  outcome: StepOutcome
  state: str
  result: Any | None = None
  reason: str | None = None


class PlanStepProjection(BaseModel):
  """One executed concurrent step in the operator plan view.

  Carries what the operator needs to follow execution: the step's position,
  its aggregate outcome, its motions with their outcomes, and the concluding
  answer when the step ended the mission. Cost accounting (tokens, retries,
  duration) stays on the record itself.
  """

  index: int
  outcome: StepOutcome
  state: str
  result: Any | None = None
  reason: str | None = None
  final: str | None = None
  actions: list[ActionProjection] = Field(default_factory=list)


class PlannedStepProjection(BaseModel):
  """One semantic plan step as the operator sees it."""

  id: str
  sentence: str
  status: PlanStepStatus
  commands: list[str] = Field(default_factory=list)
  reason: str | None = None
  required: bool = False
  after: list[str] = Field(default_factory=list)
  terminal: bool = False
  evidence: str | None = None


class PlanProjection(BaseModel):
  """The working view of a live mission plan (§3.3).

  A projection of the in-flight :class:`PlanRecord` — the steps executed so
  far, the mission's current semantic steps, and the count of side-loop
  annotations — never a separate list that dies with the run. Emitted on
  northbound frames while a mission runs.
  """

  plan_id: str
  mission_id: str
  priority: float = 0.0
  revision: int = Field(default=1, ge=1)
  completeness: float = Field(default=0.0, ge=0.0, le=1.0)
  steps: list[PlanStepProjection] = Field(default_factory=list)
  plan: list[PlannedStepProjection] = Field(default_factory=list)
  decision_mode: Literal["plan", "execute"] = "plan"
  decision_graph: DecisionGraph | None = None
  annotations: int = Field(ge=0, description="side-loop verdicts stamped so far")
