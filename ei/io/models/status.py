"""Northbound health and backend frames."""

from __future__ import annotations

from datetime import datetime

from pydantic import BaseModel, Field

from ei.core.system.models.status import (
  BackendStatus,
  HealthState,
  SafetyStatus,
  SubsystemStatus,
)


class BackendStatusFrame(BaseModel):
  """One ``/backend`` stream frame: one status per wired backend endpoint."""

  backends: list[BackendStatus] = Field(
    default_factory=list,
    description="one BackendStatus per configured backend, in config order",
  )


class ServerStatus(BaseModel):
  """Health-check payload served under ``/api/aether/ei/*``.

  Timestamps are RFC3339/ISO-8601 UTC strings on the wire. ``uptime_seconds``
  is measured from pipeline launch, not process start.
  """

  status: HealthState
  version: str = Field(description="EI package version, e.g. from importlib.metadata")
  started_at: datetime = Field(description="pipeline launch time, UTC")
  uptime_seconds: float = Field(ge=0, description="seconds since pipeline launch")
  correlation_id: str | None = Field(
    default=None,
    description="correlation ID of the health request, for log cross-reference",
  )
  subsystems: list[SubsystemStatus] = Field(
    default_factory=list,
    description="per-subsystem health reported by the core supervisor",
  )
  safety: SafetyStatus | None = Field(
    default=None,
    description="gate verdict counters; None when no gate is wired",
  )
