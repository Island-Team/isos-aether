//! Rust skill contract for a confined EOS skill process.
//!
//! A skill executable calls [`serve`] with its registered skills. EOS owns the
//! process, validates its catalog, and brokers every declared primitive call.

use std::collections::BTreeSet;
use std::io;
use std::os::fd::RawFd;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const PROTOCOL_VERSION: u32 = 2;
const SOCKET_FD: RawFd = 3;
mod packet;
static CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Primitive {
  pub name: String,
  pub version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
  pub name: String,
  pub version: String,
  pub description: String,
  pub parameters_schema: Value,
  pub returns_schema: Value,
  pub primitives: Vec<Primitive>,
  pub reentrant: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
  Inspect,
  Invoke {
    descriptor_index: usize,
    command_id: String,
    arguments_json: String,
  },
  PrimitiveReply {
    status: u32,
    acknowledgement_json: String,
  },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
  Catalog {
    descriptors: Vec<Descriptor>,
  },
  PrimitiveCall {
    name: String,
    version: String,
    device_id: String,
    arguments_json: String,
  },
  Completion {
    accepted: bool,
    reason_code: String,
    result_json: String,
    message: String,
    status: u32,
  },
  Failure {
    code: String,
  },
}

#[derive(Debug, Error)]
pub enum SkillError {
  #[error("skill protocol: {0}")]
  Protocol(&'static str),
  #[error("skill IO: {0}")]
  Io(#[from] io::Error),
  #[error("skill serialization: {0}")]
  Json(#[from] serde_json::Error),
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Packet<T> {
  version: u32,
  body: T,
}

/// The only route from a skill to an actuator is this brokered primitive call.
pub trait Host {
  fn call_primitive(
    &mut self,
    name: &str,
    version: &str,
    device_id: &str,
    arguments: &Value,
  ) -> Result<(u32, Value), SkillError>;

  fn is_cancelled(&self) -> bool;
}

pub struct Invocation<'a> {
  pub command_id: &'a str,
  pub arguments: &'a Value,
  pub host: &'a mut dyn Host,
}

#[derive(Clone, Debug)]
pub struct Completion {
  pub accepted: bool,
  pub reason_code: String,
  pub result: Value,
  pub message: String,
}

pub trait Skill: Send + Sync {
  fn descriptor(&self) -> Descriptor;
  fn execute(&self, invocation: Invocation<'_>) -> Result<Completion, SkillError>;
}

pub fn serve(skills: &[Box<dyn Skill>]) -> Result<(), SkillError> {
  install_cancel_handler();
  let result = serve_one(skills);
  if let Err(error) = &result {
    let code = match error {
      SkillError::Protocol(code) => *code,
      SkillError::Io(_) => "skill_io_failed",
      SkillError::Json(_) => "skill_json_invalid",
    };
    let _ = send(&Event::Failure { code: code.into() });
  }
  result
}

fn serve_one(skills: &[Box<dyn Skill>]) -> Result<(), SkillError> {
  if skills.is_empty() || skills.len() > 256 {
    return Err(SkillError::Protocol("catalog_size_invalid"));
  }
  let mut identities = BTreeSet::new();
  let descriptors: Vec<_> = skills.iter().map(|skill| skill.descriptor()).collect();
  for descriptor in &descriptors {
    if !identities.insert((&descriptor.name, &descriptor.version)) {
      return Err(SkillError::Protocol("catalog_duplicate_identity"));
    }
  }
  let request: Request = receive()?;
  match request {
    Request::Inspect => send(&Event::Catalog { descriptors }),
    Request::Invoke {
      descriptor_index,
      command_id,
      arguments_json,
    } => {
      let skill = skills
        .get(descriptor_index)
        .ok_or(SkillError::Protocol("skill_not_admitted"))?;
      let arguments: Value = serde_json::from_str(&arguments_json)?;
      if !arguments.is_object() || command_id.is_empty() || command_id.len() > 96 {
        return Err(SkillError::Protocol("invocation_invalid"));
      }
      let mut host = Broker;
      let result = skill.execute(Invocation {
        command_id: &command_id,
        arguments: &arguments,
        host: &mut host,
      })?;
      if !result.result.is_object() {
        return Err(SkillError::Protocol("result_invalid"));
      }
      send(&Event::Completion {
        accepted: result.accepted,
        reason_code: result.reason_code,
        result_json: serde_json::to_string(&result.result)?,
        message: result.message,
        status: 0,
      })
    }
    Request::PrimitiveReply { .. } => Err(SkillError::Protocol("request_invalid")),
  }
}

struct Broker;

impl Host for Broker {
  fn call_primitive(
    &mut self,
    name: &str,
    version: &str,
    device_id: &str,
    arguments: &Value,
  ) -> Result<(u32, Value), SkillError> {
    if !arguments.is_object() {
      return Err(SkillError::Protocol("primitive_arguments_invalid"));
    }
    send(&Event::PrimitiveCall {
      name: name.into(),
      version: version.into(),
      device_id: device_id.into(),
      arguments_json: serde_json::to_string(arguments)?,
    })?;
    let Request::PrimitiveReply {
      status,
      acknowledgement_json,
    } = receive()?
    else {
      return Err(SkillError::Protocol("primitive_reply_invalid"));
    };
    if status > 4 {
      return Err(SkillError::Protocol("primitive_reply_invalid"));
    }
    let acknowledgement: Value = serde_json::from_str(&acknowledgement_json)?;
    if !acknowledgement.is_object() {
      return Err(SkillError::Protocol("primitive_reply_invalid"));
    }
    Ok((status, acknowledgement))
  }

  fn is_cancelled(&self) -> bool {
    CANCELLED.load(Ordering::SeqCst)
  }
}

pub fn install_cancel_handler() {
  unsafe extern "C" fn cancel(_: libc::c_int) {
    CANCELLED.store(true, Ordering::SeqCst);
  }
  unsafe { libc::signal(libc::SIGUSR1, cancel as *const () as usize) };
}

fn send(event: &Event) -> Result<(), SkillError> {
  let bytes = serde_json::to_vec(&Packet {
    version: PROTOCOL_VERSION,
    body: event,
  })?;
  packet::send_packet(SOCKET_FD, &bytes)
}

fn receive() -> Result<Request, SkillError> {
  let bytes = packet::recv_packet(SOCKET_FD, Duration::from_secs(5))?;
  let packet: Packet<Request> = serde_json::from_slice(&bytes)?;
  if packet.version != PROTOCOL_VERSION {
    return Err(SkillError::Protocol("protocol_version_invalid"));
  }
  Ok(packet.body)
}
