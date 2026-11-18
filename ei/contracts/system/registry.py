"""Typed registry for EOS discovery, resource, and primitive contracts."""

from __future__ import annotations

from dataclasses import dataclass

from pydantic import BaseModel

from ei.contracts.evidence.perception import (
  AsrSnapshot,
  BaseDevice,
  CameraDevice,
  DetectionSnapshot,
  GimbalDevice,
  GripperDevice,
  ManipulatorDevice,
  MicrophoneDevice,
  SpeakerDevice,
  WorldStateSnapshot,
)
from ei.contracts.io.link import (
  ArmInitCommand,
  ArmMoveJCommand,
  ArmMoveLCommand,
  ArmPickCommand,
  BaseNavigateCommand,
  BasePostureCommand,
  BaseVelocityCommand,
  DeviceKind,
  EstopEngageCommand,
  FaceRegisterCommand,
  GimbalMotionCommand,
  GripperMotionCommand,
  MotionStopCommand,
  PrimitiveKind,
  ResourceKind,
  RobotStopCommand,
  SpeakerPlayCommand,
  SpeakerSpeakCommand,
)


@dataclass(frozen=True)
class DeviceContract:
  """Typed class-specific content and canonical schema for one device kind."""

  kind: DeviceKind
  model: type[BaseModel]
  schema_path: str


@dataclass(frozen=True)
class ResourceContract:
  """Typed model and streaming behavior for one readable resource kind."""

  kind: ResourceKind
  model: type[BaseModel]
  schema_path: str
  version: str
  stream: bool = True


@dataclass(frozen=True)
class ActionContract:
  """Typed argument model for one callable primitive kind."""

  kind: PrimitiveKind
  model: type[BaseModel]
  schema_path: str
  version: str

  @property
  def route(self) -> str:
    """Return the primitive's exact wire name."""
    return self.kind.value


DEVICE_CONTRACTS = {
  DeviceKind.BASE: DeviceContract(
    DeviceKind.BASE, BaseDevice, "schemas/body/base.v1.json"
  ),
  DeviceKind.GIMBAL: DeviceContract(
    DeviceKind.GIMBAL, GimbalDevice, "schemas/body/gimbal.v1.json"
  ),
  DeviceKind.MANIPULATOR: DeviceContract(
    DeviceKind.MANIPULATOR,
    ManipulatorDevice,
    "schemas/body/manipulator.v1.json",
  ),
  DeviceKind.GRIPPER: DeviceContract(
    DeviceKind.GRIPPER, GripperDevice, "schemas/body/gripper.v1.json"
  ),
  DeviceKind.CAMERA: DeviceContract(
    DeviceKind.CAMERA, CameraDevice, "schemas/body/camera.v1.json"
  ),
  DeviceKind.MICROPHONE: DeviceContract(
    DeviceKind.MICROPHONE,
    MicrophoneDevice,
    "schemas/body/mic.v1.json",
  ),
  DeviceKind.SPEAKER: DeviceContract(
    DeviceKind.SPEAKER, SpeakerDevice, "schemas/body/speaker.v1.json"
  ),
}

RESOURCE_CONTRACTS = {
  ResourceKind.WORLD: ResourceContract(
    ResourceKind.WORLD,
    WorldStateSnapshot,
    "schemas/world/world.v1.json",
    "1.0",
  ),
  ResourceKind.DETECTION: ResourceContract(
    ResourceKind.DETECTION,
    DetectionSnapshot,
    "schemas/world/detection.v1.json",
    "1.0",
  ),
  ResourceKind.ASR: ResourceContract(
    ResourceKind.ASR,
    AsrSnapshot,
    "schemas/world/asr.v1.json",
    "1.0",
  ),
}

ACTION_CONTRACTS = {
  PrimitiveKind.ARM_INIT: ActionContract(
    PrimitiveKind.ARM_INIT,
    ArmInitCommand,
    "schemas/actions/arm_init.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.BASE_VELOCITY: ActionContract(
    PrimitiveKind.BASE_VELOCITY,
    BaseVelocityCommand,
    "schemas/actions/base_velocity.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.BASE_NAVIGATE: ActionContract(
    PrimitiveKind.BASE_NAVIGATE,
    BaseNavigateCommand,
    "schemas/actions/base_navigate.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.BASE_POSTURE: ActionContract(
    PrimitiveKind.BASE_POSTURE,
    BasePostureCommand,
    "schemas/actions/base_posture.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.GIMBAL_MOTION: ActionContract(
    PrimitiveKind.GIMBAL_MOTION,
    GimbalMotionCommand,
    "schemas/actions/gimbal_motion.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.ARM_MOVEJ: ActionContract(
    PrimitiveKind.ARM_MOVEJ,
    ArmMoveJCommand,
    "schemas/actions/arm_movej.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.ARM_MOVEL: ActionContract(
    PrimitiveKind.ARM_MOVEL,
    ArmMoveLCommand,
    "schemas/actions/arm_movel.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.ARM_PICK: ActionContract(
    PrimitiveKind.ARM_PICK,
    ArmPickCommand,
    "schemas/actions/arm_pick.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.GRIPPER_MOTION: ActionContract(
    PrimitiveKind.GRIPPER_MOTION,
    GripperMotionCommand,
    "schemas/actions/gripper_motion.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.SPEAKER_PLAY: ActionContract(
    PrimitiveKind.SPEAKER_PLAY,
    SpeakerPlayCommand,
    "schemas/actions/speaker_play.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.SPEAKER_SPEAK: ActionContract(
    PrimitiveKind.SPEAKER_SPEAK,
    SpeakerSpeakCommand,
    "schemas/actions/speaker_speak.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.FACE_REGISTER: ActionContract(
    PrimitiveKind.FACE_REGISTER,
    FaceRegisterCommand,
    "schemas/actions/face_register.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.MOTION_STOP: ActionContract(
    PrimitiveKind.MOTION_STOP,
    MotionStopCommand,
    "schemas/actions/motion_stop.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.ROBOT_STOP: ActionContract(
    PrimitiveKind.ROBOT_STOP,
    RobotStopCommand,
    "schemas/actions/robot_stop.v1.json",
    "1.0.0",
  ),
  PrimitiveKind.SAFETY_ESTOP: ActionContract(
    PrimitiveKind.SAFETY_ESTOP,
    EstopEngageCommand,
    "schemas/actions/safety_estop.v1.json",
    "1.0.0",
  ),
}


def device_contract(kind: DeviceKind) -> DeviceContract:
  """Return the contract registered for a discovered device kind."""
  try:
    return DEVICE_CONTRACTS[kind]
  except KeyError as exc:
    raise ValueError(f"unknown device contract: {kind!r}") from exc


def resource_contract(kind: ResourceKind) -> ResourceContract:
  """Return the contract registered for a readable resource kind."""
  try:
    return RESOURCE_CONTRACTS[kind]
  except KeyError as exc:
    raise ValueError(f"unknown resource contract: {kind!r}") from exc


def action_contract(kind: PrimitiveKind) -> ActionContract:
  """Return the contract registered for a primitive kind."""
  try:
    return ACTION_CONTRACTS[kind]
  except KeyError as exc:
    raise ValueError(f"unknown primitive contract: {kind!r}") from exc


def action_contract_for_uri(name: str) -> ActionContract:
  """Resolve a primitive wire name, tolerating one leading slash."""
  try:
    return action_contract(PrimitiveKind(name.lstrip("/")))
  except ValueError as exc:
    raise ValueError(f"unknown primitive: {name!r}") from exc


__all__ = [
  "ACTION_CONTRACTS",
  "DEVICE_CONTRACTS",
  "RESOURCE_CONTRACTS",
  "ActionContract",
  "DeviceContract",
  "ResourceContract",
  "action_contract",
  "action_contract_for_uri",
  "device_contract",
  "resource_contract",
]
