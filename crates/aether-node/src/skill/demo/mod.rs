mod arm_status;
mod joint_delta;

use serde_json::Value;

#[derive(Clone, Copy)]
pub(crate) enum Skill {
  JointDelta,
  ArmStatus,
}

impl Skill {
  pub(crate) fn from_type(node_type: &str) -> Option<Self> {
    match node_type {
      "eos.skill.demo.joint_delta@v0.2" => Some(Self::JointDelta),
      "eos.skill.demo.arm_status@v0.2" => Some(Self::ArmStatus),
      _ => None,
    }
  }

  pub(crate) fn output(self, envelope: &Value) -> Value {
    match self {
      Self::JointDelta => joint_delta::output(),
      Self::ArmStatus => arm_status::output(envelope),
    }
  }
}
