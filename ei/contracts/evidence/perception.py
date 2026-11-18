"""Typed perception resource snapshots consumed by EI."""

from __future__ import annotations

from datetime import datetime
from typing import Annotated, Literal, Self

from pydantic import BaseModel, ConfigDict, Field, JsonValue, model_validator

from ei.contracts.system.source import load_json


class PlanarPose(BaseModel):
  """Pose in the map frame using metres and counter-clockwise radians."""

  x_m: float
  y_m: float
  yaw_rad: float


class PlanarVelocity(BaseModel):
  """Planar velocity in the map frame (m/s and rad/s)."""

  x_m_s: float
  y_m_s: float
  yaw_rad_s: float


SCHEMA_VERSION = "1.0"


class SnapshotEnvelope(BaseModel):
  """Wire frame shared by every polled state resource."""

  schema_version: str = SCHEMA_VERSION
  sequence: int = Field(ge=0)
  timestamp: datetime
  frame_id: str = "map"
  part_id: str = Field(
    default="default",
    description="instance address for multi-part systems; 'default' when unique",
  )
  description: str | None = Field(
    default=None,
    description=(
      "human-facing hardware description so an agent can recognize the "
      "instance, e.g. 'left arm' or 'front camera'; null when unnamed"
    ),
  )
  tags: list[str] = Field(
    default_factory=list,
    description=(
      "grouping labels for selection, e.g. ['station1']; tags never target "
      "primitives — callers resolve them to concrete device ids first"
    ),
  )


class PowerState(BaseModel):
  """Semantic battery / power state after adapter-level semanticalization."""

  charge_pct: float = Field(ge=0, le=100)
  health_pct: float = Field(ge=0, le=100)
  voltage_v: float = Field(gt=0)
  current_a: float = Field(description="pack current; negative when charging")
  is_charging: bool = False
  power_source: Literal["battery", "docked", "external"] = "battery"
  remaining_s: float | None = Field(
    default=None, description="estimated seconds of charge left at the current draw"
  )


class BaseConfiguration(BaseModel):
  """Installed mobile-base geometry and supported drive modes."""

  model_config = ConfigDict(extra="forbid")
  footprint_length_m: float = Field(gt=0)
  footprint_width_m: float = Field(gt=0)
  supported_modes: list[Literal["differential", "omnidirectional", "legged"]] = Field(
    min_length=1
  )


class BaseOperationalState(BaseModel):
  """Low-rate base controller state reported during discovery."""

  model_config = ConfigDict(extra="forbid")
  controller_state: Literal["disabled", "idle", "active", "fault"]
  motors_enabled: bool


class BaseDevice(BaseModel):
  """Class-specific content of a discovered base descriptor."""

  model_config = ConfigDict(extra="forbid")
  device_class: Literal["base"] = "base"
  configuration: BaseConfiguration
  operational_state: BaseOperationalState


class CameraConfiguration(BaseModel):
  """Installed camera format and optical configuration."""

  model_config = ConfigDict(extra="forbid")
  width_px: int = Field(ge=1)
  height_px: int = Field(ge=1)
  frame_rate_hz: float = Field(gt=0)
  pixel_format: str = Field(min_length=1)
  fov_rad: float | None = Field(gt=0, le=6.283185307179586)


class CameraOperationalState(BaseModel):
  """Low-rate camera availability reported during discovery."""

  model_config = ConfigDict(extra="forbid")
  present: bool
  capturing: bool


class CameraDevice(BaseModel):
  """Class-specific content of a discovered camera descriptor."""

  model_config = ConfigDict(extra="forbid")
  device_class: Literal["camera"] = "camera"
  configuration: CameraConfiguration
  operational_state: CameraOperationalState


class GimbalConfiguration(BaseModel):
  """Installed gimbal travel and optical limits."""

  model_config = ConfigDict(extra="forbid")
  pan_min_rad: float
  pan_max_rad: float
  tilt_min_rad: float
  tilt_max_rad: float
  fov_rad: float = Field(gt=0, le=6.283185307179586)
  range_m: float | None = Field(gt=0)

  @model_validator(mode="after")
  def _ordered_ranges(self) -> Self:
    if self.pan_min_rad > self.pan_max_rad or self.tilt_min_rad > self.tilt_max_rad:
      raise ValueError("gimbal minimum angle must not exceed maximum angle")
    return self


class GimbalOperationalState(BaseModel):
  """Low-rate gimbal controller state reported during discovery."""

  model_config = ConfigDict(extra="forbid")
  enabled: bool
  controller_state: Literal["disabled", "idle", "active", "fault"]


class GimbalDevice(BaseModel):
  """Class-specific content of a discovered gimbal descriptor."""

  model_config = ConfigDict(extra="forbid")
  device_class: Literal["gimbal"] = "gimbal"
  configuration: GimbalConfiguration
  operational_state: GimbalOperationalState


class GripperConfiguration(BaseModel):
  """Installed gripper controls and supported actions."""

  model_config = ConfigDict(extra="forbid")
  supported_actions: list[Literal["open", "close"]] = Field(min_length=1)
  supports_force_control: bool
  supports_position_control: bool


class GripperOperationalState(BaseModel):
  """Low-rate jaw state reported during discovery."""

  model_config = ConfigDict(extra="forbid")
  jaw_state: Literal["unknown", "open", "closed", "moving", "fault"]


class GripperDevice(BaseModel):
  """Class-specific content of a discovered gripper descriptor."""

  model_config = ConfigDict(extra="forbid")
  device_class: Literal["gripper"] = "gripper"
  configuration: GripperConfiguration
  operational_state: GripperOperationalState


class ManipulatorConfiguration(BaseModel):
  """Installed manipulator joints, tool, and mounting frame."""

  model_config = ConfigDict(extra="forbid")
  joint_names: list[str] = Field(min_length=1)
  joint_count: int = Field(ge=1)
  end_effector_type: str | None
  mount_frame: str = Field(min_length=1)

  @model_validator(mode="after")
  def _joint_count_matches_names(self) -> Self:
    if self.joint_count != len(self.joint_names):
      raise ValueError("joint_count must equal the number of joint_names")
    if len(set(self.joint_names)) != len(self.joint_names):
      raise ValueError("joint_names must be unique")
    return self


class ManipulatorOperationalState(BaseModel):
  """Low-rate manipulator controller state reported during discovery."""

  model_config = ConfigDict(extra="forbid")
  controller_state: Literal["disabled", "idle", "active", "fault"]
  calibrated: bool


class ManipulatorDevice(BaseModel):
  """Class-specific content of a discovered manipulator descriptor."""

  model_config = ConfigDict(extra="forbid")
  device_class: Literal["manipulator"] = "manipulator"
  configuration: ManipulatorConfiguration
  operational_state: ManipulatorOperationalState


class MicrophoneConfiguration(BaseModel):
  """Installed microphone sampling configuration."""

  model_config = ConfigDict(extra="forbid")
  sample_rate_hz: int = Field(ge=1)
  channels: int = Field(ge=1)
  sample_format: str = Field(min_length=1)
  window_duration_ms: int = Field(ge=1)


class MicrophoneOperationalState(BaseModel):
  """Low-rate microphone availability reported during discovery."""

  model_config = ConfigDict(extra="forbid")
  present: bool
  capturing: bool


class MicrophoneDevice(BaseModel):
  """Class-specific content of a discovered microphone descriptor."""

  model_config = ConfigDict(extra="forbid")
  device_class: Literal["microphone"] = "microphone"
  configuration: MicrophoneConfiguration
  operational_state: MicrophoneOperationalState


class SpeakerConfiguration(BaseModel):
  """Installed speaker formats and channel configuration."""

  model_config = ConfigDict(extra="forbid")
  supported_media_types: list[str] = Field(min_length=1)
  sample_rates_hz: list[int] = Field(min_length=1)
  channels: int = Field(ge=1)


class SpeakerOperationalState(BaseModel):
  """Low-rate speaker playback state reported during discovery."""

  model_config = ConfigDict(extra="forbid")
  present: bool
  playback_state: Literal["idle", "buffering", "playing", "draining", "fault"]
  buffered_duration_ms: int = Field(ge=0)


class SpeakerDevice(BaseModel):
  """Class-specific content of a discovered speaker descriptor."""

  model_config = ConfigDict(extra="forbid")
  device_class: Literal["speaker"] = "speaker"
  configuration: SpeakerConfiguration
  operational_state: SpeakerOperationalState


ROBOT_STATUS_TYPE = "robot.status"


class RobotStatus(SnapshotEnvelope):
  """One replaceable robot operational-status snapshot (standalone resource).

  The robot's **general status section**: system-level operational state plus
  the power subsystem, in one frame. Distinct from the part snapshots: this
  carries no spatial ``state`` struct because it describes the platform
  rather than a kinematic joint. ``estop`` engaged means motion commands must
  be rejected regardless of ``mode``; ``power`` is the composed
  :class:`PowerState` struct — there is no standalone power resource.
  """

  type: Literal["robot.status"] = ROBOT_STATUS_TYPE
  frame_id: str = "robot"
  mode: Literal["auto", "teleop", "idle", "charging", "error"] = "idle"
  estop: bool = False
  motors_enabled: bool = True
  autonomy_enabled: bool = False
  uptime_s: float = Field(ge=0)
  faults: list[str] = Field(default_factory=list)
  power: PowerState = Field(
    default_factory=lambda: PowerState(
      charge_pct=100.0, health_pct=100.0, voltage_v=14.8, current_a=0.0
    )
  )


class WorldSnapshot(BaseModel):
  """Sequence and timing fields shared by all world snapshots."""

  model_config = ConfigDict(extra="forbid")
  schema_version: str = "1.0"
  sequence: int = Field(ge=0)
  timestamp: datetime
  source_timestamp_ns: int | None = Field(ge=0)


class AsrSnapshot(BaseModel):
  """One complete recognized-speech result without audio or token timing."""

  model_config = ConfigDict(extra="forbid")
  type: Literal["asr.snapshot"] = "asr.snapshot"
  schema_version: Literal["1.0"] = "1.0"
  sequence: int = Field(ge=0)
  timestamp: datetime
  text: str
  language: str | None = Field(default=None, min_length=2, max_length=35)
  confidence: float | None = Field(default=None, ge=0, le=1)


class Detection(BaseModel):
  """One typed human or object observation."""

  model_config = ConfigDict(extra="forbid")
  id: str = Field(pattern=r"^(?:human|obj):[^\s:]+$")
  kind: Literal["human", "object"]
  label: str = Field(min_length=1, max_length=128)
  confidence: float = Field(ge=0, le=1)
  distance_m: float | None = Field(default=None, ge=0)
  bearing_rad: float | None = None
  attributes: dict[str, str | float | bool | None] | None = None

  @model_validator(mode="after")
  def _typed_identity(self) -> Self:
    prefix = "human:" if self.kind == "human" else "obj:"
    if not self.id.startswith(prefix):
      raise ValueError(f"{self.kind} detection id must start with {prefix!r}")
    if self.attributes is not None and len(self.attributes) > 16:
      raise ValueError("detection attributes may contain at most 16 values")
    return self


class DetectionSnapshot(BaseModel):
  """One complete semantic model observation."""

  model_config = ConfigDict(extra="forbid")
  type: Literal["detection.snapshot"] = "detection.snapshot"
  schema_version: Literal["1.0"] = "1.0"
  sequence: int = Field(ge=0)
  timestamp: datetime
  source: str = Field(min_length=1)
  source_sequence: int | None = Field(default=None, ge=1)
  detections: list[Detection]


OBJECTS_SNAPSHOT_TYPE = "objects.snapshot"


class ObjectLocation(BaseModel):
  """A named place and optional slot within that place."""

  place_id: str = Field(min_length=1, max_length=256)
  slot_id: str | None = Field(default=None, min_length=1, max_length=256)


class ObjectRelation(BaseModel):
  """One directed relation from the containing object to another entity."""

  kind: str = Field(min_length=1, max_length=128)
  target_id: str = Field(min_length=1, max_length=256)
  slot_id: str | None = Field(default=None, min_length=1, max_length=256)


class ObjectSlot(BaseModel):
  """A child-facing slot, its accepted kinds, occupants, and metadata."""

  slot_id: str = Field(min_length=1, max_length=256)
  accepts: list[str] = Field(default_factory=list)
  occupants: list[str] = Field(default_factory=list)
  properties: dict[str, JsonValue] = Field(default_factory=dict)

  @model_validator(mode="after")
  def _unique_values(self) -> Self:
    if len(set(self.accepts)) != len(self.accepts):
      raise ValueError("slot accepts must be unique")
    if len(set(self.occupants)) != len(self.occupants):
      raise ValueError("slot occupants must be unique")
    return self


class WorldObjectState(BaseModel):
  """Current operational state for one backend-owned world object."""

  object_id: str = Field(min_length=1, max_length=256)
  kind: str = Field(min_length=1, max_length=128)
  location: ObjectLocation | None = None
  states: list[str] = Field(default_factory=list)
  relations: list[ObjectRelation] = Field(default_factory=list)
  slots: list[ObjectSlot] = Field(default_factory=list)
  properties: dict[str, JsonValue] = Field(default_factory=dict)

  @model_validator(mode="after")
  def _unique_members(self) -> Self:
    if len(set(self.states)) != len(self.states):
      raise ValueError("object states must be unique")
    slot_ids = [slot.slot_id for slot in self.slots]
    if len(set(slot_ids)) != len(slot_ids):
      raise ValueError("object slot ids must be unique")
    relations = [
      (relation.kind, relation.target_id, relation.slot_id)
      for relation in self.relations
    ]
    if len(set(relations)) != len(relations):
      raise ValueError("object relations must be unique")
    return self


class StockLevel(BaseModel):
  """Aggregate uninstantiated stock available from one inventory."""

  inventory_id: str = Field(min_length=1, max_length=256)
  kind: str = Field(min_length=1, max_length=128)
  variant: str | None = Field(default=None, min_length=1, max_length=256)
  available: int = Field(ge=0)
  properties: dict[str, JsonValue] = Field(default_factory=dict)


class ObjectStateSnapshot(SnapshotEnvelope):
  """One complete, replaceable operational object-state snapshot."""

  type: Literal["objects.snapshot"] = OBJECTS_SNAPSHOT_TYPE
  frame_id: str = "world"
  objects: list[WorldObjectState] = Field(default_factory=list)
  stock: list[StockLevel] = Field(default_factory=list)

  @model_validator(mode="after")
  def _unique_records(self) -> Self:
    object_ids = [item.object_id for item in self.objects]
    if len(set(object_ids)) != len(object_ids):
      raise ValueError("snapshot object ids must be unique")
    stock_keys = [(item.inventory_id, item.kind, item.variant) for item in self.stock]
    if len(set(stock_keys)) != len(stock_keys):
      raise ValueError("snapshot stock records must be unique")
    return self


Keypoint = tuple[float, float]
Keypoints = Annotated[list[Keypoint], Field(min_length=17, max_length=17)]
KeypointScores = Annotated[
  list[Annotated[float, Field(ge=0, le=1)]], Field(min_length=17, max_length=17)
]


class PosePerson(BaseModel):
  """One tracked or untracked person with COCO-17 keypoints."""

  model_config = ConfigDict(extra="forbid")
  track_id: int | None = Field(ge=1)
  bbox: tuple[float, float, float, float]
  confidence: float = Field(ge=0, le=1)
  distance_m: float | None = Field(ge=0)
  keypoints: Keypoints
  keypoint_scores: KeypointScores


class PoseSnapshot(WorldSnapshot):
  """One complete normalized human-pose result."""

  type: Literal["pose.snapshot"] = "pose.snapshot"
  frame_id: str = Field(min_length=1)
  model_id: str = Field(min_length=1)
  persons: list[PosePerson]


class WorldEntity(BaseModel):
  """One typed physical entity in the fused world state."""

  model_config = ConfigDict(extra="forbid")
  id: str = Field(pattern=r"^(?:human|obj):[^\s:]+$")
  label: str = Field(min_length=1, max_length=128)
  position_m: tuple[float, float, float]
  velocity_m_s: tuple[float, float, float] | None = None
  state: str | None = Field(default=None, min_length=1, max_length=64)


class WorldRelation(BaseModel):
  """One directed semantic relation between two world entities."""

  model_config = ConfigDict(extra="forbid")
  subject: str = Field(pattern=r"^(?:human|obj):[^\s:]+$")
  relation: str = Field(min_length=1, max_length=64)
  object: str = Field(pattern=r"^(?:human|obj):[^\s:]+$")


class WorldStateSnapshot(BaseModel):
  """One complete fused external-world state."""

  model_config = ConfigDict(extra="forbid")
  type: Literal["world.snapshot"] = "world.snapshot"
  schema_version: Literal["1.0"] = "1.0"
  sequence: int = Field(ge=0)
  timestamp: datetime
  frame_id: str = Field(min_length=1)
  entities: list[WorldEntity]
  relations: list[WorldRelation]


class VadSegment(BaseModel):
  """One detected speech interval in seconds."""

  model_config = ConfigDict(extra="forbid")
  start_s: float = Field(ge=0)
  end_s: float = Field(gt=0)
  confidence: float | None = Field(ge=0, le=1)

  @model_validator(mode="after")
  def _positive_duration(self) -> Self:
    if self.end_s <= self.start_s:
      raise ValueError("VAD segment end_s must be greater than start_s")
    return self


class VadSnapshot(WorldSnapshot):
  """One complete normalized voice-activity result."""

  type: Literal["vad.snapshot"] = "vad.snapshot"
  model_id: str = Field(min_length=1)
  speech: bool
  probability: float | None = Field(ge=0, le=1)
  segments: list[VadSegment]

  @model_validator(mode="after")
  def _segments_match_speech(self) -> Self:
    if self.speech != bool(self.segments):
      raise ValueError("speech must be true exactly when VAD segments are present")
    if any(
      current.start_s < previous.end_s
      for previous, current in zip(self.segments, self.segments[1:])
    ):
      raise ValueError("VAD segments must be ordered and non-overlapping")
    return self


class VisionDetection(BaseModel):
  """One normalized object detection in source-image coordinates."""

  model_config = ConfigDict(extra="forbid")
  object_id: str | None
  label: str = Field(min_length=1)
  bbox: tuple[float, float, float, float] | None
  distance_m: float | None = Field(ge=0)
  bearing_rad: float | None
  confidence: float = Field(ge=0, le=1)
  pickable: bool | None
  unpickable_reason: str | None


class VisionSnapshot(WorldSnapshot):
  """One complete normalized object-detection result."""

  type: Literal["vision.snapshot"] = "vision.snapshot"
  frame_id: str = Field(min_length=1)
  model_id: str = Field(min_length=1)
  detections: list[VisionDetection]


class RobotKinematics(BaseModel):
  """Any currently observed subset of robot planar kinematics."""

  model_config = ConfigDict(extra="forbid")
  pose: PlanarPose | None = None
  velocity: PlanarVelocity | None = None
  posture: str | None = None


class EndEffectorState(BaseModel):
  """Any currently observed subset of one end effector's state."""

  model_config = ConfigDict(extra="forbid")
  frame_id: str | None = Field(default=None, min_length=1)
  position_m: tuple[float, float, float] | None = None
  orientation_quat_xyzw: tuple[float, float, float, float] | None = None
  jaw_state: str | None = None
  held_object_id: str | None = None


class SceneObject(BaseModel):
  """One spatial object fused into the scene."""

  model_config = ConfigDict(extra="forbid")
  id: str = Field(min_length=1)
  label: str = Field(min_length=1)
  position_m: tuple[float, float, float]
  orientation_quat_xyzw: tuple[float, float, float, float] | None = None
  radius_m: float | None = Field(default=None, gt=0)
  confidence: float | None = Field(default=None, ge=0, le=1)


class SceneSnapshot(WorldSnapshot):
  """One complete, replaceable fused world-scene snapshot."""

  type: Literal["scene.snapshot"] = "scene.snapshot"
  frame_id: str = Field(min_length=1)
  robot: RobotKinematics | None = None
  end_effectors: list[EndEffectorState] | None = None
  objects: list[SceneObject] | None = None

  @model_validator(mode="after")
  def _contains_observation(self) -> Self:
    if self.robot is None and self.end_effectors is None and self.objects is None:
      raise ValueError("scene requires robot, end_effectors, or objects")
    if self.objects is not None:
      ids = [item.id for item in self.objects]
      if len(ids) != len(set(ids)):
        raise ValueError("scene object ids must be unique")
    return self

  @classmethod
  def model_json_schema(cls, *args, **kwargs) -> dict:
    """Return EOS's canonical scene schema."""
    del args, kwargs
    return load_json("schemas/world/world.v1.json")


SceneState = SceneSnapshot
