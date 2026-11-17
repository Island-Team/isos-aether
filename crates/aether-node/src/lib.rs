//! Built-in EOS nodes using the same Linux node ABI as package nodes.
pub mod hardware;
use std::{ffi::c_void, mem};
use serde_json::Value;

mod skill;
use skill::demo::Skill;

#[repr(C)]
#[derive(Clone, Copy)]
struct Host {
  abi_version: u32,
  struct_size: u32,
  context: *mut c_void,
  emit: Option<extern "C" fn(*mut c_void, *const u8, usize) -> i32>,
  log: Option<extern "C" fn(*mut c_void, u32, *const u8, usize)>,
}
#[repr(C)]
pub struct Api {
  abi_version: u32,
  struct_size: u32,
  create: Option<unsafe extern "C" fn(*const u8, usize, *const Host, *mut *mut c_void) -> i32>,
  start: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
  input: Option<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32>,
  health: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
  stop: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
  destroy: Option<unsafe extern "C" fn(*mut c_void)>,
}
struct Handle {
  host: Host,
  skill: Skill,
  started: bool,
  stopped: bool,
}
unsafe extern "C" fn create(
  config: *const u8,
  length: usize,
  host: *const Host,
  out: *mut *mut c_void,
) -> i32 {
  if config.is_null() || host.is_null() || out.is_null() || length > 64 * 1024 {
    return -1;
  }
  let host = *host;
  if host.abi_version != 1
    || host.struct_size < mem::size_of::<Host>() as u32
    || host.emit.is_none()
  {
    return -1;
  }
  let Ok(config) = serde_json::from_slice::<Value>(std::slice::from_raw_parts(config, length))
  else {
    return -1;
  };
  let kind = config["instance"]["node"]["node_type"]
    .as_str()
    .or_else(|| config["node_type"].as_str());
  let Some(skill) = kind.and_then(Skill::from_type) else {
    return -1;
  };
  *out = Box::into_raw(Box::new(Handle {
    host,
    skill,
    started: false,
    stopped: false,
  }))
  .cast();
  0
}
unsafe extern "C" fn start(node: *mut c_void) -> i32 {
  let Some(node) = (node as *mut Handle).as_mut() else {
    return -1;
  };
  if node.stopped {
    return -1;
  }
  node.started = true;
  0
}
unsafe extern "C" fn input(node: *mut c_void, bytes: *const u8, length: usize) -> i32 {
  let Some(node) = (node as *mut Handle).as_mut() else {
    return -1;
  };
  if !node.started || node.stopped || bytes.is_null() || length > 64 * 1024 {
    return -1;
  }
  let Ok(envelope) = serde_json::from_slice::<Value>(std::slice::from_raw_parts(bytes, length))
  else {
    return -1;
  };
  let output = node.skill.output(&envelope);
  let Ok(payload) = serde_json::to_vec(&output) else {
    return -1;
  };
  node.host.emit.unwrap()(node.host.context, payload.as_ptr(), payload.len())
}
unsafe extern "C" fn health(node: *mut c_void) -> i32 {
  (node as *mut Handle)
    .as_ref()
    .map_or(-1, |node| if node.started && !node.stopped { 1 } else { 0 })
}
unsafe extern "C" fn stop(node: *mut c_void) -> i32 {
  let Some(node) = (node as *mut Handle).as_mut() else {
    return -1;
  };
  node.stopped = true;
  0
}
unsafe extern "C" fn destroy(node: *mut c_void) {
  if !node.is_null() {
    drop(Box::from_raw(node as *mut Handle));
  }
}
static API: Api = Api {
  abi_version: 1,
  struct_size: mem::size_of::<Api>() as u32,
  create: Some(create),
  start: Some(start),
  input: Some(input),
  health: Some(health),
  stop: Some(stop),
  destroy: Some(destroy),
};
pub fn api() -> *const Api {
  &API
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::ptr;
  extern "C" fn no_op_emit(_: *mut c_void, _: *const u8, _: usize) -> i32 {
    0
  }
  #[test]
  fn demo_skill_abi_rejects_unknown_type() {
    let host = Host {
      abi_version: 1,
      struct_size: mem::size_of::<Host>() as u32,
      context: ptr::null_mut(),
      emit: Some(no_op_emit),
      log: None,
    };
    let mut out = ptr::null_mut();
    let config = br#"{"node_type":"eos.skill.demo.unknown@v0.2"}"#;
    assert_ne!(
      unsafe { create(config.as_ptr(), config.len(), &host, &mut out) },
      0
    );
    assert!(out.is_null());
  }

  extern "C" fn capture_emit(context: *mut c_void, bytes: *const u8, length: usize) -> i32 {
    let output = unsafe { &mut *(context as *mut Vec<u8>) };
    output.extend_from_slice(unsafe { std::slice::from_raw_parts(bytes, length) });
    0
  }

  #[test]
  fn demo_skill_emits_and_stops_through_abi() {
    let mut output = Vec::new();
    let host = Host {
      abi_version: 1,
      struct_size: mem::size_of::<Host>() as u32,
      context: (&mut output as *mut Vec<u8>).cast(),
      emit: Some(capture_emit),
      log: None,
    };
    let mut node = ptr::null_mut();
    let config = br#"{"node_type":"eos.skill.demo.arm_status@v0.2"}"#;
    assert_eq!(
      unsafe { create(config.as_ptr(), config.len(), &host, &mut node) },
      0
    );
    assert_eq!(unsafe { start(node) }, 0);
    let input_bytes = br#"{"payload":{"ready":true}}"#;
    assert_eq!(
      unsafe { input(node, input_bytes.as_ptr(), input_bytes.len()) },
      0
    );
    assert_eq!(
      serde_json::from_slice::<Value>(&output).unwrap(),
      serde_json::json!({"port":"observation","payload":{"ready":true}})
    );
    assert_eq!(unsafe { health(node) }, 1);
    assert_eq!(unsafe { stop(node) }, 0);
    assert_eq!(unsafe { health(node) }, 0);
    assert_ne!(
      unsafe { input(node, input_bytes.as_ptr(), input_bytes.len()) },
      0
    );
    unsafe { destroy(node) };
  }
}
