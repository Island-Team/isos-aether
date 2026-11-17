"""Typed EOS link contracts consumed by EI."""

from __future__ import annotations

from collections.abc import Mapping
from enum import StrEnum
from math import isfinite
from typing import Any, ClassVar, Literal, Self

from pydantic import (
  AliasChoices,
  BaseModel,
  ConfigDict,
  Field,
  StrictBool,
  field_validator,
  model_validator,
)

from ei.contracts.system.source import load_json


class DeviceKind(StrEnum):
  """Canonical class names admitted from EOS device discovery."""

  BASE = "base"
  GIMBAL = "gimbal"
  MANIPULATOR = "manipulator"
  GRIPPER = "gripper"
  CAMERA = "camera"
  MICROPHONE = "microphone"
  SPEAKER = "speaker"


BODY_DEVICE_KINDS: tuple[DeviceKind, ...] = tuple(DeviceKind)


class ResourceKind(StrEnum):
  """Canonical readable EOS resource classes admitted by EI."""

  WORLD = "world"
  DETECTION = "detection"
  ASR = "asr"


WORLD_RESOURCE_KINDS: tuple[ResourceKind, ...] = (
  ResourceKind.WORLD,
  ResourceKind.DETECTION,
  ResourceKind.ASR,
)


class PrimitiveKind(StrEnum):
  """Canonical EOS primitive names admitted by EI."""

  BASE_VELOCITY = "base.velocity"
  BASE_NAVIGATE = "base.navigate"
  BASE_POSTURE = "base.posture"
  GIMBAL_MOTION = "gimbal.motion"
  ARM_INIT = "arm.init"
  ARM_MOVEJ = "arm.movej"
  ARM_MOVEL = "arm.movel"
  ARM_PICK = "arm.pick"
  GRIPPER_MOTION = "gripper.motion"
  SPEAKER_PLAY = "speaker.play"
  SPEAKER_SPEAK = "speaker.speak"
  FACE_REGISTER = "face.register"
  MOTION_STOP = "motion.stop"
  ROBOT_STOP = "robot.stop"
  SAFETY_ESTOP = "safety.estop"


STANDARD_PRIMITIVE_KINDS: tuple[PrimitiveKind, ...] = (
  PrimitiveKind.BASE_VELOCITY,
  PrimitiveKind.BASE_NAVIGATE,
  PrimitiveKind.BASE_POSTURE,
  PrimitiveKind.GIMBAL_MOTION,
  PrimitiveKind.GRIPPER_MOTION,
  PrimitiveKind.SPEAKER_PLAY,
  PrimitiveKind.ARM_PICK,
  PrimitiveKind.SPEAKER_SPEAK,
  PrimitiveKind.FACE_REGISTER,
)
OPERATOR_PRIMITIVE_KINDS: tuple[PrimitiveKind, ...] = (
  PrimitiveKind.ARM_INIT,
  PrimitiveKind.ARM_MOVEJ,
  PrimitiveKind.ARM_MOVEL,
)
PRIVILEGED_PRIMITIVE_KINDS: tuple[PrimitiveKind, ...] = (
  PrimitiveKind.MOTION_STOP,
  PrimitiveKind.ROBOT_STOP,
  PrimitiveKind.SAFETY_ESTOP,
)
PRIMITIVE_KINDS = (
  *STANDARD_PRIMITIVE_KINDS,
  *OPERATOR_PRIMITIVE_KINDS,
  *PRIVILEGED_PRIMITIVE_KINDS,
)


def resource_kind(resource_class: str | None, uri: str) -> ResourceKind | None:
  """Resolve one EOS resource listing to a supported typed resource."""
  try:
    kind = ResourceKind(resource_class or "")
  except ValueError:
    return None
  expected_suffix = {
    ResourceKind.WORLD: "/world/state",
    ResourceKind.DETECTION: "/detection/state",
    ResourceKind.ASR: "/world/asr/state",
  }[kind]
  if uri.endswith(expected_suffix):
    return kind
  return None


class NumericRange(BaseModel):
  """One non-empty finite numeric interval from an inline primitive binding."""

  model_config = ConfigDict(extra="forbid", frozen=True, allow_inf_nan=False)

  minimum: float | None = None
  maximum: float | None = None
  minimum_exclusive: StrictBool = Field(
    default=False,
    validation_alias=AliasChoices("minimum_exclusive", "minimumExclusive"),
  )
  maximum_exclusive: StrictBool = Field(
    default=False,
    validation_alias=AliasChoices("maximum_exclusive", "maximumExclusive"),
  )

  @field_validator("minimum", "maximum", mode="before")
  @classmethod
  def _numeric_bound(cls, value: Any) -> Any:
    if value is not None and (
      isinstance(value, bool) or not isinstance(value, (int, float))
    ):
      raise TypeError("numeric bounds must be numbers")
    return value

  @model_validator(mode="after")
  def _valid_interval(self) -> NumericRange:
    if self.minimum is None and self.maximum is None:
      raise ValueError("a numeric range requires minimum or maximum")
    if self.minimum_exclusive and self.minimum is None:
      raise ValueError("minimum_exclusive requires minimum")
    if self.maximum_exclusive and self.maximum is None:
      raise ValueError("maximum_exclusive requires maximum")
    if self.minimum is not None and not isfinite(self.minimum):
      raise ValueError("minimum must be finite")
    if self.maximum is not None and not isfinite(self.maximum):
      raise ValueError("maximum must be finite")
    if (
      self.minimum is not None
      and self.maximum is not None
      and (
        self.minimum > self.maximum
        or (
          self.minimum == self.maximum
          and (self.minimum_exclusive or self.maximum_exclusive)
        )
      )
    ):
      raise ValueError("bounds do not form a non-empty range")
    return self

  def contains(self, value: float) -> bool:
    """Return whether ``value`` lies inside this interval."""
    if not isfinite(value):
      return False
    if self.minimum is not None and (
      value < self.minimum or (self.minimum_exclusive and value == self.minimum)
    ):
      return False
    return not (
      self.maximum is not None
      and (value > self.maximum or (self.maximum_exclusive and value == self.maximum))
    )


PRIMITIVE_LIMIT_FIELDS: Mapping[PrimitiveKind, frozenset[str]] = {
  PrimitiveKind.BASE_VELOCITY: frozenset(
    {"linear_x_m_s", "linear_y_m_s", "angular_z_rad_s", "valid_for_ms"}
  ),
  PrimitiveKind.BASE_NAVIGATE: frozenset({"x_m", "y_m", "yaw_rad"}),
  PrimitiveKind.GIMBAL_MOTION: frozenset({"pan_rad", "tilt_rad", "speed_rad_s"}),
  PrimitiveKind.ARM_MOVEJ: frozenset({"joint_positions_rad", "speed_rad_s"}),
  PrimitiveKind.ARM_MOVEL: frozenset({"speed_m_s"}),
  PrimitiveKind.GRIPPER_MOTION: frozenset({"force_n"}),
}

# EOS requires these fields to admit a primitive. Remaining limit fields are
# optional capabilities: their command values are valid only when advertised.
PRIMITIVE_REQUIRED_LIMIT_FIELDS: Mapping[PrimitiveKind, frozenset[str]] = {
  PrimitiveKind.BASE_VELOCITY: PRIMITIVE_LIMIT_FIELDS[PrimitiveKind.BASE_VELOCITY],
  PrimitiveKind.BASE_NAVIGATE: PRIMITIVE_LIMIT_FIELDS[PrimitiveKind.BASE_NAVIGATE],
  PrimitiveKind.GIMBAL_MOTION: frozenset({"pan_rad", "tilt_rad"}),
  PrimitiveKind.ARM_MOVEJ: frozenset({"joint_positions_rad"}),
}


def parse_numeric_limits(
  kind: PrimitiveKind, raw: Mapping[str, Any]
) -> dict[str, NumericRange]:
  """Validate one descriptor's required and optional numeric limits map."""
  allowed = PRIMITIVE_LIMIT_FIELDS.get(kind, frozenset())
  required = PRIMITIVE_REQUIRED_LIMIT_FIELDS.get(kind, frozenset())
  if not required.issubset(raw) or not set(raw).issubset(allowed):
    missing = sorted(required - set(raw))
    unknown = sorted(set(raw) - allowed)
    details = []
    if missing:
      details.append(f"missing {missing}")
    if unknown:
      details.append(f"unknown {unknown}")
    raise ValueError("invalid numeric limits: " + ", ".join(details))
  return {name: NumericRange.model_validate(value) for name, value in raw.items()}


def validate_numeric_arguments(
  kind: PrimitiveKind,
  limits: Mapping[str, NumericRange],
  arguments: Mapping[str, Any],
) -> None:
  """Enforce ranges and reject optional values absent from device capabilities."""
  unavailable = sorted(
    name
    for name in arguments
    if name in PRIMITIVE_LIMIT_FIELDS.get(kind, frozenset()) and name not in limits
  )
  if unavailable:
    raise ValueError(f"{unavailable[0]} is not admitted by this device")
  for name, interval in limits.items():
    if name not in arguments:
      continue
    value = arguments[name]
    values = value if isinstance(value, list) else [value]
    for item in values:
      if isinstance(item, bool) or not isinstance(item, (int, float)):
        raise TypeError(f"{name} must be numeric")
      if not interval.contains(float(item)):
        raise ValueError(f"{name}={item!r} is outside the admitted device range")


class PrimitiveCommand(BaseModel):
  """Base for caller-supplied primitive arguments.

  Command identity belongs to ``CapabilityService.SubmitOperation.operation_id``.
  Supplying it inside ``arguments`` is therefore rejected by the common
  extra-field policy.
  """

  model_config = ConfigDict(extra="forbid")
  schema_path: ClassVar[str]

  @classmethod
  def model_json_schema(cls, *args, **kwargs) -> dict:
    """Return the canonical EOS schema selected explicitly by this model."""
    del args, kwargs
    return load_json(cls.schema_path)


class ArmInitCommand(PrimitiveCommand):
  """Request the deployment-configured initial joint pose for one arm."""

  schema_path = "schemas/actions/arm_init.v1.json"
  type: Literal["arm.init"] = "arm.init"


class BaseVelocityCommand(PrimitiveCommand):
  """Command body-frame base velocity for a bounded lease."""

  schema_path = "schemas/actions/base_velocity.v1.json"
  type: Literal["base.velocity"] = "base.velocity"
  linear_x_m_s: float
  linear_y_m_s: float
  angular_z_rad_s: float
  valid_for_ms: int


class BaseNavigateCommand(PrimitiveCommand):
  """Navigate to a relative or absolute planar goal."""

  schema_path = "schemas/actions/base_navigate.v1.json"
  type: Literal["base.navigate"] = "base.navigate"
  reference: Literal["relative", "absolute"]
  frame_id: str | None = Field(default=None, min_length=1)
  x_m: float
  y_m: float
  yaw_rad: float

  @model_validator(mode="after")
  def _frame_matches_reference(self) -> Self:
    if (self.reference == "absolute") != (self.frame_id is not None):
      raise ValueError("frame_id is required only for an absolute target")
    return self


class BasePostureCommand(PrimitiveCommand):
  """Request one supported high-level base posture."""

  schema_path = "schemas/actions/base_posture.v1.json"
  type: Literal["base.posture"] = "base.posture"
  posture: Literal["stand", "walk", "lie"]


class GimbalMotionCommand(PrimitiveCommand):
  """Move a gimbal using relative or absolute pan and tilt targets."""

  schema_path = "schemas/actions/gimbal_motion.v1.json"
  type: Literal["gimbal.motion"] = "gimbal.motion"
  reference: Literal["relative", "absolute"]
  pan_rad: float
  tilt_rad: float
  speed_rad_s: float | None = None


class ArmMoveJCommand(PrimitiveCommand):
  """Move every configured arm joint by or to an ordered angular target."""

  schema_path = "schemas/actions/arm_movej.v1.json"
  type: Literal["arm.movej"] = "arm.movej"
  reference: Literal["relative", "absolute"]
  joint_positions_rad: list[float] = Field(min_length=1)
  speed_rad_s: float | None = None


class ArmMoveLCommand(PrimitiveCommand):
  """Move an arm tool using a relative transform or absolute pose."""

  schema_path = "schemas/actions/arm_movel.v1.json"
  type: Literal["arm.movel"] = "arm.movel"
  reference: Literal["relative", "absolute"]
  frame_id: str | None = Field(default=None, min_length=1)
  position_m: tuple[float, float, float]
  orientation_quat_xyzw: tuple[float, float, float, float]
  speed_m_s: float | None = None

  @model_validator(mode="after")
  def _frame_matches_reference(self) -> Self:
    if (self.reference == "absolute") != (self.frame_id is not None):
      raise ValueError("frame_id is required only for an absolute target")
    return self


class ArmPickCommand(PrimitiveCommand):
  """Pick one typed non-human object from the current world state."""

  schema_path = "schemas/actions/arm_pick.v1.json"
  type: Literal["arm.pick"] = "arm.pick"
  object_id: str = Field(pattern=r"^obj:[^\s:]+$")


class GripperMotionCommand(PrimitiveCommand):
  """Open or close a gripper with an optional admitted force."""

  schema_path = "schemas/actions/gripper_motion.v1.json"
  type: Literal["gripper.motion"] = "gripper.motion"
  action: Literal["open", "close"]
  force_n: float | None = None


class SpeakerPlayCommand(PrimitiveCommand):
  """Play one EOS-readable audio asset through a speaker."""

  schema_path = "schemas/actions/speaker_play.v1.json"
  type: Literal["speaker.play"] = "speaker.play"
  uri: str = Field(
    min_length=1,
    max_length=2048,
    pattern=r"^[A-Za-z][A-Za-z0-9+.-]*:[^\s]+$",
  )


class SpeakerSpeakCommand(PrimitiveCommand):
  """Synthesize text and play it through the bound speaker."""

  schema_path = "schemas/actions/speaker_speak.v1.json"
  type: Literal["speaker.speak"] = "speaker.speak"
  text: str = Field(min_length=1, max_length=4096)
  language: str | None = Field(default=None, min_length=2, max_length=35)
  voice: str | None = Field(default=None, min_length=1, max_length=128)


class FaceRegisterCommand(PrimitiveCommand):
  """Register a named face using the bound camera's latest valid image."""

  schema_path = "schemas/actions/face_register.v1.json"
  type: Literal["face.register"] = "face.register"
  name: str = Field(min_length=1, max_length=256)


class EstopEngageCommand(PrimitiveCommand):
  """Engage the robot-wide emergency stop."""

  schema_path = "schemas/actions/safety_estop.v1.json"
  type: Literal["safety.estop"] = "safety.estop"


class MotionStopCommand(PrimitiveCommand):
  """Bring one actuator's current motion to a controlled stop."""

  schema_path = "schemas/actions/motion_stop.v1.json"
  type: Literal["motion.stop"] = "motion.stop"


class RobotStopCommand(PrimitiveCommand):
  """Bring all active robot parts to a controlled stop."""

  schema_path = "schemas/actions/robot_stop.v1.json"
  type: Literal["robot.stop"] = "robot.stop"


class RejectReason(StrEnum):
  """Machine-readable EOS primitive rejection taxonomy."""

  ESTOP = "rejected.estop"
  MODE = "rejected.mode"
  POSTURE = "rejected.posture"
  INVALID_STATE = "rejected.invalid_state"
  BUSY = "rejected.busy"
  UNSUPPORTED = "rejected.unsupported"
  INVALID_ARGUMENTS = "rejected.invalid_arguments"
  INVALID_TARGET = "rejected.invalid_target"
  UNSUPPORTED_REFERENCE = "rejected.unsupported_reference"
  UNAVAILABLE_DEPENDENCY = "rejected.unavailable_dependency"
  UNAUTHORIZED = "rejected.unauthorized"
  COMMAND_CONFLICT = "rejected.command_conflict"
  INTERNAL = "rejected.internal"


class TerminalStatus(StrEnum):
  """Terminal outcome of an EOS primitive command."""

  SUCCEEDED = "succeeded"
  REJECTED = "rejected"
  FAILED = "failed"
  CANCELLED = "cancelled"
  TIMED_OUT = "timed_out"
  OUTCOME_UNKNOWN = "outcome_unknown"


class PrimitiveAck(BaseModel):
  """Result envelope for one versioned primitive invocation."""

  model_config = ConfigDict(extra="forbid")

  type: Literal["primitive.ack"] = "primitive.ack"
  primitive_version: str = Field(
    pattern=r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$"
  )
  primitive: str
  accepted: bool
  reason: str | None = None
  reason_code: RejectReason | None = None
  command_id: str | None
  sequence: int = Field(ge=0)
  result: dict[str, Any]

  @model_validator(mode="after")
  def _reason_matches_acceptance(self) -> Self:
    if self.accepted:
      if self.reason is not None or self.reason_code is not None:
        raise ValueError("accepted acknowledgement requires null rejection reason")
    elif not self.reason or self.reason_code is None:
      raise ValueError("rejected acknowledgement requires reason and reason_code")
    try:
      TerminalStatus(self.result.get("status"))
    except (TypeError, ValueError) as exc:
      raise ValueError("result.status must be a terminal status") from exc
    return self

  @property
  def status(self) -> TerminalStatus:
    """Return the validated terminal status from the semantic result."""
    return TerminalStatus(self.result["status"])

  @property
  def succeeded(self) -> bool:
    """Whether EOS confirmed the primitive's terminal success boundary."""
    return self.status is TerminalStatus.SUCCEEDED

  @classmethod
  def model_json_schema(cls, *args, **kwargs) -> dict:
    del args, kwargs
    return load_json("schemas/actions/ack.v1.json")
