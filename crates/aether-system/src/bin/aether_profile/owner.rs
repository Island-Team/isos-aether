//! Persistent EOS setup and arm activation runtime owner endpoint.

use std::{
  collections::{BTreeMap, BTreeSet},
  error::Error,
  fs::{self, File},
  io::{BufRead, BufReader, Read, Write},
  os::unix::{
    fs::{FileTypeExt, PermissionsExt},
    net::{UnixListener, UnixStream},
    process::CommandExt,
  },
  path::Path,
  process::{Child, Command},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  time::{Duration, Instant},
};

use aether_hal::{NodeKind, NodeTypeRegistry, Profile};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_REQUEST: usize = 1024 * 1024;
// The registered image payload may occupy 20 MiB before the owner envelope is added.
const MAX_REPLY: usize = 24 * 1024 * 1024;
const MAX_NATIVE_REPLY: u64 = 24 * 1024 * 1024;
const MAX_HEADER: usize = 8192;
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn request_shutdown(_: libc::c_int) {
  SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

#[derive(Default)]
struct State {
  child: Option<Child>,
  demo_activation_enabled: bool,
  demo_monitor_enabled: bool,
  mock_child: Option<Child>,
  mock_socket: std::path::PathBuf,
  mock_observer: String,
  cross_links: Vec<CrossLink>,
  last_mock_sequence: u64,
  skipped_mock_samples: u64,
  cross_error: Option<String>,
  cross_sampler: Option<CrossSampler>,
  service_endpoints: Vec<(Value, aether_hal::NodeProfile)>,
  package_child: Option<super::package_runtime::ProcessChild>,
  package_inputs: super::package_runtime::PackageInputs,
  package_socket: std::path::PathBuf,
  run_id: String,
  revision: String,
  worker_socket: std::path::PathBuf,
  worker_loss_reported: bool,
  native: bool,
  mixed: bool,
  ei_socket: Option<std::path::PathBuf>,
  ei_links: Vec<EiLink>,
  native_inputs: Vec<InputLink>,
  native_outputs: BTreeMap<(String, String), String>,
  native_observers: Vec<String>,
  vision_transfers: BTreeMap<String, String>,
  vision_token: String,
  external_dispatcher: Option<super::external_dispatcher::ExternalDispatcher>,
  external_ports: BTreeMap<(String, String), String>,
  external_types: BTreeMap<String, String>,
  external_frames: BTreeMap<String, String>,
  external_joints: BTreeMap<String, Vec<String>>,
  external_limits: BTreeMap<String, Value>,
  admission_token: String,
}

#[derive(Default)]
struct CrossSnapshot {
  sequence: u64,
  skipped: u64,
  error: Option<String>,
  latest: Option<Value>,
}

struct CrossSampler {
  stop: Arc<AtomicBool>,
  snapshot: Arc<Mutex<CrossSnapshot>>,
  thread: Option<std::thread::JoinHandle<()>>,
}

fn sync_cross_sampler(state: &mut State) {
  let Some(sampler) = state.cross_sampler.as_ref() else {
    return;
  };
  if let Ok(snapshot) = sampler.snapshot.lock() {
    state.last_mock_sequence = snapshot.sequence;
    state.skipped_mock_samples = snapshot.skipped;
    if let Some(error) = snapshot.error.as_ref() {
      state.cross_error = Some(error.clone());
    }
  }
}

fn stop_cross_sampler(state: &mut State) {
  if let Some(mut sampler) = state.cross_sampler.take() {
    sampler.stop.store(true, Ordering::SeqCst);
    if let Some(thread) = sampler.thread.take() {
      let _ = thread.join();
    }
    if let Ok(snapshot) = sampler.snapshot.lock() {
      state.last_mock_sequence = snapshot.sequence;
      state.skipped_mock_samples = snapshot.skipped;
      if let Some(error) = snapshot.error.as_ref() {
        state.cross_error = Some(error.clone());
      }
    }
  }
}

fn start_cross_sampler(state: &mut State, registry: &NodeTypeRegistry) {
  let stop = Arc::new(AtomicBool::new(false));
  let snapshot = Arc::new(Mutex::new(CrossSnapshot::default()));
  let mut route = State {
    run_id: state.run_id.clone(),
    revision: state.revision.clone(),
    mock_socket: state.mock_socket.clone(),
    mock_observer: state.mock_observer.clone(),
    worker_socket: state.worker_socket.clone(),
    cross_links: state.cross_links.clone(),
    native_outputs: state.native_outputs.clone(),
    ..State::default()
  };
  let registry = registry.clone();
  let thread_stop = Arc::clone(&stop);
  let thread_snapshot = Arc::clone(&snapshot);
  let thread = std::thread::spawn(move || {
    while !thread_stop.load(Ordering::SeqCst) {
      match route_mock_observation(&mut route, &registry) {
        Ok(reply) => {
          if let Ok(mut snapshot) = thread_snapshot.lock() {
            snapshot.sequence = route.last_mock_sequence;
            snapshot.skipped = route.skipped_mock_samples;
            snapshot.latest = Some(reply);
          }
        }
        Err(error) => {
          mark_cross_error(&mut route, &error);
          if let Ok(mut snapshot) = thread_snapshot.lock() {
            snapshot.error = Some(error);
          }
          break;
        }
      }
      // The worker's own sampler controls when a new source sequence exists.
      // Yield between reads without imposing an additional observation rate.
      std::thread::sleep(Duration::from_millis(1));
    }
  });
  state.cross_sampler = Some(CrossSampler {
    stop,
    snapshot,
    thread: Some(thread),
  });
}

#[derive(Clone)]
struct EiLink {
  source: String,
  output: String,
  target: String,
  input: String,
}
#[derive(Clone)]
struct InputLink {
  source: String,
  output: String,
  target: String,
  input: String,
  data_type: String,
}
#[derive(Clone)]
struct CrossLink {
  source: String,
  output: String,
  target: String,
  input: String,
  data_type: String,
}

fn forward_ei_input(
  socket: &Path,
  run_id: &str,
  revision: &str,
  link: &EiLink,
  payload: &Value,
) -> Result<(), String> {
  let body = serde_json::to_vec(
    &json!({"schema_version":1,"core":"ei","op":"input","run_id":run_id,"revision":revision,
    "source_node":link.source,"source_port":link.output,"node":link.target,"port":link.input,"payload":payload}),
  )
  .map_err(|error| error.to_string())?;
  if body.len() > 64 * 1024 {
    return Err("EI input exceeds 64 KiB".into());
  }
  let mut stream = UnixStream::connect(socket).map_err(|error| error.to_string())?;
  stream
    .set_read_timeout(Some(Duration::from_secs(2)))
    .map_err(|error| error.to_string())?;
  stream
    .set_write_timeout(Some(Duration::from_secs(2)))
    .map_err(|error| error.to_string())?;
  write!(
    stream,
    "POST /v1/input HTTP/1.1\r\nHost: owner\r\nContent-Length: {}\r\n\r\n",
    body.len()
  )
  .map_err(|error| error.to_string())?;
  stream.write_all(&body).map_err(|error| error.to_string())?;
  let mut reply = Vec::new();
  stream
    .take(64 * 1024)
    .read_to_end(&mut reply)
    .map_err(|error| error.to_string())?;
  let text = std::str::from_utf8(&reply).map_err(|error| error.to_string())?;
  let (head, body) = text.split_once("\r\n\r\n").ok_or("EI reply missing body")?;
  let value: Value = serde_json::from_str(body).map_err(|error| error.to_string())?;
  if !head.starts_with("HTTP/1.1 200")
    || value["ok"] != true
    || value["run_id"] != run_id
    || value["revision"] != revision
  {
    return Err("EI rejected forwarded input".into());
  }
  Ok(())
}

fn native_worker_exchange(socket: &Path, body: &Value) -> Result<Value, String> {
  let bytes = serde_json::to_vec(body).map_err(|e| e.to_string())?;
  if bytes.len() > 64 * 1024 {
    return Err("native command too large".into());
  }
  let mut worker = UnixStream::connect(socket).map_err(|e| e.to_string())?;
  worker
    .set_read_timeout(Some(Duration::from_secs(2)))
    .map_err(|e| e.to_string())?;
  worker
    .set_write_timeout(Some(Duration::from_secs(2)))
    .map_err(|e| e.to_string())?;
  worker.write_all(&bytes).map_err(|e| e.to_string())?;
  worker
    .shutdown(std::net::Shutdown::Write)
    .map_err(|e| e.to_string())?;
  let mut reply = Vec::new();
  worker
    .take(MAX_NATIVE_REPLY + 1)
    .read_to_end(&mut reply)
    .map_err(|e| e.to_string())?;
  if reply.len() as u64 > MAX_NATIVE_REPLY {
    return Err("native reply too large".into());
  }
  serde_json::from_slice(&reply).map_err(|e| e.to_string())
}

fn route_mock_observation(state: &mut State, registry: &NodeTypeRegistry) -> Result<Value, String> {
  let run_id = &state.run_id;
  let revision = &state.revision;
  let source = &state.mock_observer;
  let observation = native_worker_exchange(
    &state.mock_socket,
    &json!({"schema_version":1,"core":"eos","op":"observe_output","run_id":run_id,
      "revision":revision,"node":source,"port":"observation"}),
  )?;
  if observation["schema_version"] != 1
    || observation["core"] != "eos"
    || observation["ok"] != true
    || observation["run_id"] != *run_id
    || observation["revision"] != *revision
    || observation["node"] != *source
    || observation["port"] != "observation"
  {
    return Err("mock_observation_invalid".into());
  }
  let payload = &observation["payload"];
  registry
    .validate_data_payload("arm_monitor_state@v0.2", payload)
    .map_err(|_| "mock_observation_payload_invalid")?;
  let mut outputs = vec![json!({"node":source,"output":{"port":"observation","payload":payload}})];
  let sequence = payload["sequence"]
    .as_u64()
    .ok_or("mock_observation_payload_invalid")?;
  if sequence <= state.last_mock_sequence {
    return Ok(
      json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,
      "revision":revision,"outputs":outputs}),
    );
  }
  if state.cross_links.len() > 256 {
    return Err("cross_worker_delivery_limit".into());
  }
  for link in state
    .cross_links
    .iter()
    .filter(|link| link.source == *source && link.output == "observation")
  {
    registry
      .validate_data_payload(&link.data_type, payload)
      .map_err(|_| "mock_observation_payload_invalid")?;
    let reply = native_worker_exchange(
      &state.worker_socket,
      &json!({"op":"input","run_id":run_id,"revision":revision,
        "source_node":link.source,"source_port":link.output,"node":link.target,
        "port":link.input,"payload":payload}),
    )?;
    if reply["ok"] != true {
      return Err("cross_worker_delivery_failed".into());
    }
    let emitted = reply["outputs"]
      .as_array()
      .ok_or("cross_worker_outputs_invalid")?;
    for output in emitted {
      let node = output["node"]
        .as_str()
        .ok_or("cross_worker_output_invalid")?;
      let port = output["output"]["port"]
        .as_str()
        .ok_or("cross_worker_output_invalid")?;
      let data_type = state
        .native_outputs
        .get(&(node.to_owned(), port.to_owned()))
        .ok_or("cross_worker_output_invalid")?;
      if registry.data_type(data_type).is_some() {
        registry
          .validate_data_payload(data_type, &output["output"]["payload"])
          .map_err(|_| "cross_worker_output_invalid")?;
      }
    }
    outputs.extend(emitted.iter().cloned());
    if outputs.len() > 256 {
      return Err("cross_worker_delivery_limit".into());
    }
  }
  let skipped = sequence.saturating_sub(state.last_mock_sequence.saturating_add(1));
  if skipped > 0 {
    let previous = state.skipped_mock_samples;
    state.skipped_mock_samples = previous.saturating_add(skipped);
    if previous == 0 || state.skipped_mock_samples / 100 > previous / 100 {
      let _ = super::eos_log::append_with_run(
        "warn",
        "eos.aether_profile.owner",
        &format!(
          "EOS observation sampler skipped {skipped} sequences; total {}",
          state.skipped_mock_samples
        ),
        Some(run_id),
      );
    }
  }
  state.last_mock_sequence = sequence;
  Ok(
    json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"outputs":outputs}),
  )
}

fn mark_cross_error(state: &mut State, reason: &str) {
  if state.cross_error.is_none() {
    let _ = super::eos_log::append_with_run(
      "error",
      "eos.aether_profile.owner",
      &format!("EOS cross-worker observation delivery failed: {reason}"),
      Some(&state.run_id),
    );
    state.cross_error = Some(reason.to_owned());
  }
}

fn mark_native_observation_error(state: &mut State, reason: &str) {
  if state.cross_error.is_none() {
    let _ = super::eos_log::append_with_run(
      "error",
      "eos.aether_profile.owner",
      &format!("EOS native observation route failed: {reason}"),
      Some(&state.run_id),
    );
    state.cross_error = Some(reason.to_owned());
  }
}

fn check_selected_services(state: &mut State) {
  if state.cross_error.is_some() || state.run_id.is_empty() {
    return;
  }
  let deadline = Instant::now() + Duration::from_millis(500);
  let mut checked = BTreeSet::new();
  let failed = state.service_endpoints.iter().any(|(handoff, node)| {
    let key = (
      handoff["socket_path"].as_str().unwrap_or("").to_owned(),
      handoff["service_generation"]
        .as_str()
        .unwrap_or("")
        .to_owned(),
      node
        .adapter
        .settings
        .get("device_id")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_owned(),
    );
    if !checked.insert(key) {
      return false;
    }
    probe_service_handoff(handoff, node, &state.run_id, &state.revision, deadline).is_err()
  });
  if failed {
    mark_native_observation_error(state, "service_unavailable");
  }
}

fn translate_external_command(
  node_type: &str,
  port: &str,
  payload: &Value,
) -> Result<Value, &'static str> {
  let get = |key: &str| payload[key].as_f64().ok_or("missing numeric command field");
  let translated = match (node_type, port) {
    ("eos.hardware.base.device@v0.2", "command") => {
      json!({"linear_x_m_s":get("vx_m_s")?,"linear_y_m_s":get("vy_m_s")?,"angular_z_rad_s":get("yaw_rate_rad_s")?,"valid_for_ms":payload["lease_ms"]})
    }
    ("eos.hardware.manipulator.device@v0.2", "command") => {
      json!({"joint_positions_rad":payload["position_rad"]})
    }
    ("eos.hardware.gripper.binary@v0.2", "command") => json!({"close":payload["action"]=="close"}),
    ("eos.hardware.base.normalized_axes@v0.2", "command") => {
      let axes = payload["axes"].as_object().ok_or("axes missing")?;
      if axes.len() != 3
        || axes
          .keys()
          .any(|key| !matches!(key.as_str(), "x" | "y" | "yaw"))
      {
        return Err("unknown base axis");
      }
      payload.clone()
    }
    ("eos.hardware.manipulator.normalized_cartesian@v0.2", "command") => {
      let axes = payload["axes"].as_object().ok_or("axes missing")?;
      if axes.len() != 6
        || axes.keys().any(|key| {
          !matches!(
            key.as_str(),
            "dx" | "dy" | "dz" | "droll" | "dpitch" | "dyaw"
          )
        })
      {
        return Err("unknown arm axis");
      }
      payload.clone()
    }
    ("eos.hardware.manipulator.normalized_cartesian@v0.2", "mode") => {
      let mode = payload["mode"].as_str().ok_or("mode missing")?;
      json!({"mode":match mode {"0"=>0,"1"=>1,_=>return Err("unknown control mode")}})
    }
    ("eos.hardware.torso.normalized_axes@v0.2", "command") => {
      let axes = payload["axes"].as_object().ok_or("axes missing")?;
      if axes.len() != 1 || axes.keys().any(|key| key != "lift") {
        return Err("unknown torso axis");
      }
      payload.clone()
    }
    _ => return Err("undeclared external command port"),
  };
  Ok(translated)
}

fn stop_child(child: &mut Child, socket: &Path) -> Result<(), String> {
  let mut stop_proven = false;
  if let Ok(mut worker) = UnixStream::connect(socket) {
    let _ = worker.set_read_timeout(Some(Duration::from_millis(500)));
    if worker.write_all(b"stop\n").is_ok() {
      let mut reply = [0u8; 512];
      if let Ok(count) = worker.read(&mut reply) {
        stop_proven =
          serde_json::from_slice::<Value>(&reply[..count]).is_ok_and(|value| value["ok"] == true);
      }
    }
  }
  if !stop_proven {
    return Err("worker Stop proof missing".into());
  }
  for _ in 0..40 {
    match child.try_wait() {
      Ok(Some(_)) => return cleanup_worker_socket(socket),
      Ok(None) => std::thread::sleep(Duration::from_millis(25)),
      Err(error) => return Err(format!("worker wait failed: {error}")),
    }
  }
  match child.kill() {
    Ok(()) => {}
    Err(_error) if child.try_wait().ok().flatten().is_some() => {
      return cleanup_worker_socket(socket)
    }
    Err(error) => return Err(format!("worker kill failed: {error}")),
  }
  child
    .wait()
    .map_err(|error| format!("worker reap failed: {error}"))?;
  cleanup_worker_socket(socket)
}

fn cleanup_worker_socket(socket: &Path) -> Result<(), String> {
  match fs::remove_file(socket) {
    Ok(()) => Ok(()),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(error) => Err(format!("worker socket cleanup failed: {error}")),
  }
}

fn abort_package_child(state: &mut State) {
  if let Some(mut process) = state.package_child.take() {
    let _ = process.stop_with_proof(|| Err("native adapter Stop unavailable".into()));
  }
  if !state.package_socket.as_os_str().is_empty() {
    let _ = fs::remove_file(&state.package_socket);
    state.package_socket.clear();
  }
}

fn rollback_native_start(state: &mut State) -> bool {
  let admission_closed = state
    .external_dispatcher
    .as_ref()
    .is_none_or(|dispatcher| dispatcher.close().is_ok());
  if !admission_closed {
    return false;
  }
  if let Some(child) = state.child.as_mut() {
    if stop_child(child, &state.worker_socket).is_err() {
      return false;
    }
    state.child = None;
  }
  if let Some(process) = state.package_child.as_mut() {
    if process.stop_with_proof(|| Ok(())).is_err() {
      return false;
    }
    state.package_child = None;
  }
  if !state.package_socket.as_os_str().is_empty() {
    let _ = fs::remove_file(&state.package_socket);
    state.package_socket.clear();
  }
  state.run_id.clear();
  state.revision.clear();
  state.worker_socket.clear();
  state.worker_loss_reported = false;
  state.native = false;
  state.mixed = false;
  state.demo_activation_enabled = false;
  state.demo_monitor_enabled = false;
  state.mock_child = None;
  state.mock_socket.clear();
  state.mock_observer.clear();
  state.cross_links.clear();
  state.last_mock_sequence = 0;
  state.skipped_mock_samples = 0;
  state.cross_error = None;
  state.service_endpoints.clear();
  state.ei_socket = None;
  state.ei_links.clear();
  state.native_inputs.clear();
  state.native_outputs.clear();
  state.native_observers.clear();
  state.vision_transfers.clear();
  state.vision_token.clear();
  state.external_dispatcher = None;
  state.external_ports.clear();
  state.external_types.clear();
  state.external_frames.clear();
  state.external_joints.clear();
  state.external_limits.clear();
  state.admission_token.clear();
  true
}

fn rollback_mixed_start_failure(state: &mut State, mock: &mut State, failure: Value) -> Value {
  state.mock_child = mock.child.take();
  state.mock_socket = std::mem::take(&mut mock.worker_socket);
  state.mixed = true;
  let uncertain = |error: String| {
    json!({"schema_version":1,"core":"eos","ok":false,
    "state":"stop_uncertain","run_id":state.run_id,"revision":state.revision,"error":error})
  };
  if state
    .external_dispatcher
    .as_ref()
    .is_some_and(|dispatcher| dispatcher.close().is_err())
  {
    return uncertain("dispatcher_admission_close_failed".into());
  }
  if let Some(child) = state.mock_child.as_mut() {
    if let Err(error) = stop_child(child, &state.mock_socket) {
      return uncertain(error);
    }
    state.mock_child = None;
  }
  if let Some(child) = state.child.as_mut() {
    if let Err(error) = stop_child(child, &state.worker_socket) {
      return uncertain(error);
    }
    state.child = None;
  }
  if let Some(process) = state.package_child.as_mut() {
    if let Err(error) = process.stop_with_proof(|| Ok(())) {
      return uncertain(error);
    }
    state.package_child = None;
  }
  if rollback_native_start(state) {
    failure
  } else {
    json!({"schema_version":1,"core":"eos","ok":false,"state":"stop_uncertain",
      "error":"startup_cleanup_unproven"})
  }
}

fn valid_button_source(path: &str) -> bool {
  let parts: Vec<_> = path.split('/').collect();
  matches!(parts.as_slice(), ["isos", "ui", "button", id] if !id.is_empty() && id.len() <= 48 && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')))
}

fn new_admission_token() -> Result<String, String> {
  let mut bytes = [0u8; 32];
  File::open("/dev/urandom")
    .and_then(|mut file| file.read_exact(&mut bytes))
    .map_err(|error| error.to_string())?;
  Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// IsOS supplies this run-scoped endpoint. Graph authors may select a service
/// instance, but cannot supply an endpoint, generation, or configuration hash.
fn service_handoff<'a>(
  request: &'a Value,
  node: &aether_hal::NodeProfile,
) -> Result<Option<&'a Value>, &'static str> {
  let Some(instance) = node.adapter.settings.get("service_instance") else {
    return Ok(None);
  };
  let Some(instance) = instance.as_str() else {
    return Err("invalid_service_instance");
  };
  let pin = node
    .adapter
    .package
    .as_ref()
    .ok_or("service_package_pin_missing")?;
  let handoff = &request["endpoints"]["package_services"][node.path()];
  let socket = handoff["socket_path"]
    .as_str()
    .ok_or("service_socket_missing")?;
  let generation = handoff["service_generation"]
    .as_str()
    .ok_or("service_generation_missing")?;
  let digest = handoff["config_digest"]
    .as_str()
    .ok_or("service_config_digest_missing")?;
  if handoff["package"]["id"] != pin.id
    || handoff["package"]["version"] != pin.version
    || handoff["service_instance"] != instance
    || !std::path::Path::new(socket).is_absolute()
    || socket.len() > 102
    || generation.is_empty()
    || generation.len() > 128
    || digest.len() != 64
    || !digest
      .bytes()
      .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    || node.adapter.settings.contains_key("socket_path")
    || node.adapter.settings.contains_key("service_generation")
    || node.adapter.settings.contains_key("config_digest")
  {
    return Err("service_handoff_mismatch");
  }
  Ok(Some(handoff))
}

fn probe_service_handoff(
  handoff: &Value,
  node: &aether_hal::NodeProfile,
  run_id: &str,
  revision: &str,
  deadline: Instant,
) -> Result<(), &'static str> {
  let pin = node
    .adapter
    .package
    .as_ref()
    .ok_or("service_package_pin_missing")?;
  let instance = node
    .adapter
    .settings
    .get("service_instance")
    .and_then(|value| value.as_str())
    .ok_or("invalid_service_instance")?;
  let device = node
    .adapter
    .settings
    .get("device_id")
    .and_then(|value| value.as_str())
    .filter(|value| !value.is_empty() && value.len() <= 128)
    .ok_or("service_device_missing")?;
  let expected = format!(
    "{:x}",
    Sha256::digest(
      format!(
        "{run_id}:{revision}:{}@{}/{}",
        pin.id, pin.version, instance
      )
      .as_bytes()
    )
  );
  if handoff["service_generation"] != expected {
    return Err("service_generation_mismatch");
  }
  let socket = handoff["socket_path"]
    .as_str()
    .ok_or("service_socket_missing")?;
  if Instant::now() >= deadline {
    return Err("service_not_ready");
  }
  let mut stream = UnixStream::connect(socket).map_err(|_| "service_not_ready")?;
  let body = serde_json::to_vec(&json!({"version":"2.0.0","id":"eos-health","node":device,
    "op":"health","generation_id":expected}))
  .map_err(|_| "service_not_ready")?;
  let mut header = [0u8; 8];
  header[..4].copy_from_slice(&(body.len() as u32).to_le_bytes());
  service_io_timeout(&stream, deadline)?;
  stream.write_all(&header).map_err(|_| "service_not_ready")?;
  service_io_timeout(&stream, deadline)?;
  stream.write_all(&body).map_err(|_| "service_not_ready")?;
  service_io_timeout(&stream, deadline)?;
  stream
    .read_exact(&mut header)
    .map_err(|_| "service_not_ready")?;
  let metadata_size = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
  let payload_size = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
  if !(1..=65536).contains(&metadata_size) || payload_size != 0 {
    return Err("service_not_ready");
  }
  let mut metadata = vec![0u8; metadata_size];
  service_io_timeout(&stream, deadline)?;
  stream
    .read_exact(&mut metadata)
    .map_err(|_| "service_not_ready")?;
  let reply: Value = serde_json::from_slice(&metadata).map_err(|_| "service_not_ready")?;
  if reply["version"] != "2.0.0"
    || reply["id"] != "eos-health"
    || reply["ok"] != true
    || reply["accepted"] != true
    || reply["generation_id"] != expected
    || !reply["features"]
      .as_array()
      .is_some_and(|features| features.iter().any(|feature| feature == "strict-socket-v1"))
  {
    return Err("service_not_ready");
  }
  Ok(())
}

fn service_io_timeout(stream: &UnixStream, deadline: Instant) -> Result<(), &'static str> {
  let remaining = deadline.saturating_duration_since(Instant::now());
  if remaining.is_zero() {
    return Err("service_not_ready");
  }
  stream
    .set_read_timeout(Some(remaining))
    .map_err(|_| "service_not_ready")?;
  stream
    .set_write_timeout(Some(remaining))
    .map_err(|_| "service_not_ready")?;
  Ok(())
}

fn lifecycle_http_status(op: &str, reply: &Value) -> u16 {
  if reply["ok"] == true {
    return 200;
  }
  if ((op == "activate" || op == "result")
    && matches!(
      reply["error"].as_str(),
      Some("outcome_unknown" | "worker_unavailable")
    ))
    || (op == "monitor" && reply["error"] == "worker_unavailable")
  {
    return 503;
  }
  422
}

fn journal_result(state: &State, event_id: &str) -> Value {
  let path = (if state.mixed {
    &state.mock_socket
  } else {
    &state.worker_socket
  })
  .with_file_name(format!("eos-commands-{}.jsonl", state.run_id));
  let Ok(file) = fs::File::open(path) else {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
  };
  let mut result = None;
  for line in BufReader::new(file).lines() {
    let Ok(line) = line else { break };
    let Ok(entry) = serde_json::from_str::<Value>(&line) else {
      break;
    };
    if entry["event_id"] == event_id {
      let candidate = &entry["result"];
      if candidate["run_id"] == state.run_id
        && candidate["revision"] == state.revision
        && candidate["event_id"] == event_id
        && candidate["ok"] == true
      {
        result = Some(candidate.clone());
      } else {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"outcome_unknown"});
      }
    }
  }
  let Some(mut result) = result else {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"event_not_found"});
  };
  if matches!(result["status"].as_str(), Some("accepted" | "running")) {
    result["status"] = json!("outcome_unknown");
  }
  result
}

fn worker_state(socket: &Path, run_id: &str) -> Option<&'static str> {
  let Ok(mut stream) = UnixStream::connect(socket) else {
    return None;
  };
  if stream
    .set_read_timeout(Some(Duration::from_secs(1)))
    .is_err()
    || stream.write_all(b"status\n").is_err()
  {
    return None;
  }
  let mut bytes = Vec::new();
  let Ok(_) = stream.take(512).read_to_end(&mut bytes) else {
    return None;
  };
  let reply = serde_json::from_slice::<Value>(&bytes).ok()?;
  if reply["ok"] != true || reply["run_id"] != run_id {
    return None;
  }
  match reply["state"].as_str()? {
    "ready" => Some("ready"),
    "degraded" => Some("degraded"),
    "stale" => Some("stale"),
    "stopped" => Some("stopped"),
    _ => None,
  }
}

fn aggregate_worker_state(
  native: Option<&'static str>,
  mock: Option<&'static str>,
  mixed: bool,
) -> Option<&'static str> {
  if !mixed {
    return native;
  }
  let (Some(native), Some(mock)) = (native, mock) else {
    return None;
  };
  if native == "stale" || mock == "stale" {
    Some("stale")
  } else if native == "degraded" || mock == "degraded" {
    Some("degraded")
  } else if native == "ready" && mock == "ready" {
    Some("ready")
  } else {
    Some("stopped")
  }
}

pub fn serve_with_inputs(
  socket: &Path,
  registry: &NodeTypeRegistry,
  package_inputs: &super::package_runtime::PackageInputs,
) -> Result<(), Box<dyn Error>> {
  super::package_runtime::validate_inputs(package_inputs, registry)?;
  if !socket.is_absolute() {
    return Err("owner socket must be absolute".into());
  }
  if socket.exists() {
    if !fs::symlink_metadata(socket)?.file_type().is_socket() || UnixStream::connect(socket).is_ok()
    {
      return Err("owner socket already exists".into());
    }
    fs::remove_file(socket)?;
  }
  let parent = socket.parent().ok_or("owner socket has no parent")?;
  if !parent.is_dir() {
    return Err("owner socket parent does not exist".into());
  }
  let listener = UnixListener::bind(socket)?;
  // Signals are a local emergency stop lane. They do not depend on a saved
  // graph revision or run ID, and the owner still stops its child first.
  unsafe {
    libc::signal(
      libc::SIGINT,
      request_shutdown as *const () as libc::sighandler_t,
    );
    libc::signal(
      libc::SIGTERM,
      request_shutdown as *const () as libc::sighandler_t,
    );
  }
  listener.set_nonblocking(true)?;
  fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;
  super::eos_log::append_with_run(
    "info",
    "eos.aether_profile.owner",
    "EOS owner service listening",
    None,
  )?;
  let mut state = State {
    package_inputs: package_inputs.clone(),
    ..State::default()
  };
  while !SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
    match listener.accept() {
      Ok((mut stream, _)) => {
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let _ = respond(&mut stream, registry, &mut state, socket);
      }
      Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
        std::thread::sleep(Duration::from_millis(1));
      }
      Err(error) => return Err(error.into()),
    }
  }
  stop_cross_sampler(&mut state);
  if state
    .external_dispatcher
    .as_ref()
    .is_some_and(|dispatcher| dispatcher.close().is_err())
  {
    return Err("emergency dispatcher admission close failed".into());
  }
  if let Some(child) = state.mock_child.as_mut() {
    stop_child(child, &state.mock_socket)
      .map_err(|error| format!("emergency EOS mock worker stop failed: {error}"))?;
  }
  if let Some(child) = state.child.as_mut() {
    stop_child(child, &state.worker_socket)
      .map_err(|error| format!("emergency EOS worker stop failed: {error}"))?;
  }
  if let Some(process) = state.package_child.as_mut() {
    process.stop_with_proof(|| Ok(()))?;
  }
  fs::remove_file(socket)?;
  Ok(())
}

fn respond(
  stream: &mut UnixStream,
  registry: &NodeTypeRegistry,
  state: &mut State,
  socket: &Path,
) -> Result<(), Box<dyn Error>> {
  let mut bytes = Vec::new();
  let header_end = loop {
    if bytes.len() > MAX_HEADER {
      return write_json(stream, 431, &json!({"error":"headers_too_large"}));
    }
    let mut chunk = [0u8; 1024];
    let count = stream.read(&mut chunk)?;
    if count == 0 {
      return Ok(());
    }
    bytes.extend_from_slice(&chunk[..count]);
    if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
      break end + 4;
    }
  };
  let head = std::str::from_utf8(&bytes[..header_end])?.to_owned();
  let mut lines = head.split("\r\n");
  let request_line = lines.next().ok_or("missing request line")?;
  let mut parts = request_line.split_whitespace();
  let method = parts.next().unwrap_or("");
  let path = parts.next().unwrap_or("");
  if parts.next() != Some("HTTP/1.1") {
    return write_json(stream, 400, &json!({"error":"invalid_http"}));
  }
  let mut content_length = None;
  for line in lines.filter(|line| !line.is_empty()) {
    let Some((name, value)) = line.split_once(':') else {
      return write_json(stream, 400, &json!({"error":"invalid_header"}));
    };
    if name.eq_ignore_ascii_case("content-length") {
      if content_length.is_some() {
        return write_json(stream, 400, &json!({"error":"duplicate_length"}));
      }
      content_length = value.trim().parse::<usize>().ok();
    }
    if name.eq_ignore_ascii_case("transfer-encoding") {
      return write_json(
        stream,
        400,
        &json!({"error":"transfer_encoding_unsupported"}),
      );
    }
  }
  if method == "GET" && path == "/v1/health" {
    sync_cross_sampler(state);
    check_selected_services(state);
    let child_live = state
      .child
      .as_mut()
      .is_some_and(|child| child.try_wait().ok().flatten().is_none());
    let worker = child_live
      .then(|| worker_state(&state.worker_socket, &state.run_id))
      .flatten();
    let mock = if state.mixed {
      state.mock_child.as_mut().and_then(|child| {
        child
          .try_wait()
          .ok()
          .flatten()
          .is_none()
          .then(|| worker_state(&state.mock_socket, &state.run_id))
          .flatten()
      })
    } else {
      None
    };
    let actual = aggregate_worker_state(worker, mock, state.mixed);
    let combined = if state.cross_error.is_some() {
      None
    } else {
      actual
    };
    report_worker_loss(state, actual.is_some());
    let inhibited = state
      .external_dispatcher
      .as_ref()
      .is_some_and(|dispatcher| dispatcher.uncertain());
    return write_json(
      stream,
      200,
      &json!({"schema_version":1,"core":"eos","state":if inhibited && combined==Some("ready") {"degraded"} else {combined.unwrap_or(if state.run_id.is_empty() {"idle"} else {"failed"})},"run_id":if state.run_id.is_empty() {None} else {Some(state.run_id.as_str())},"revision":if state.revision.is_empty() {None} else {Some(state.revision.as_str())},"movement_inhibited":inhibited,"observation_samples_skipped":state.skipped_mock_samples,"setup_schema_version":3}),
    );
  }
  if method == "GET" && path.starts_with("/v1/logs") {
    let Some(after) = path
      .strip_prefix("/v1/logs?after=")
      .and_then(|value| value.parse::<u64>().ok())
    else {
      return write_json(stream, 400, &json!({"error":"invalid_log_cursor"}));
    };
    return match super::owner_log::read_after(after) {
      Ok(page) => write_json(stream, 200, &page),
      Err(_) => write_json(stream, 503, &json!({"error":"owner_logs_unavailable"})),
    };
  }
  if method != "POST" {
    return write_json(stream, 405, &json!({"error":"method_not_allowed"}));
  }
  let Some(length) = content_length else {
    return write_json(stream, 411, &json!({"error":"length_required"}));
  };
  if length > MAX_REQUEST {
    return write_json(stream, 413, &json!({"error":"request_too_large"}));
  }
  let mut body = bytes.split_off(header_end);
  if body.len() > length {
    return write_json(stream, 400, &json!({"error":"invalid_length"}));
  }
  while body.len() < length {
    let mut chunk = [0u8; 8192];
    let read_size = chunk.len().min(length - body.len());
    let count = stream.read(&mut chunk[..read_size])?;
    if count == 0 {
      return write_json(stream, 400, &json!({"error":"truncated_body"}));
    }
    body.extend_from_slice(&chunk[..count]);
  }
  let Some(op) = path.strip_prefix("/v1/") else {
    return write_json(stream, 404, &json!({"error":"not_found"}));
  };
  if !matches!(
    op,
    "identity"
      | "catalog"
      | "options"
      | "validate"
      | "render"
      | "run"
      | "status"
      | "stop"
      | "activate"
      | "result"
      | "monitor"
      | "input"
      | "command"
      | "observe"
      | "vision_frame"
      | "latest_output"
      | "vision_subscribe"
  ) {
    return write_json(stream, 404, &json!({"error":"not_found"}));
  }
  let mut request: Value = match serde_json::from_slice(&body) {
    Ok(value) => value,
    Err(_) => return write_json(stream, 400, &json!({"error":"invalid_json"})),
  };
  if request["schema_version"] != 1 || request["core"] != "eos" || request["op"] != op {
    return write_json(stream, 400, &json!({"error":"invalid_envelope"}));
  }
  let selected_registry = match super::package_registry::selected(&request, registry) {
    Ok(registry) => registry,
    Err(error) => return write_json(stream, 422, &json!({"error":error})),
  };
  let registry = &selected_registry;
  if matches!(
    op,
    "run"
      | "status"
      | "stop"
      | "activate"
      | "result"
      | "monitor"
      | "input"
      | "command"
      | "observe"
      | "vision_frame"
      | "latest_output"
      | "vision_subscribe"
  ) {
    let reply = lifecycle(op, &request, registry, state, socket);
    if matches!(op, "run" | "stop") {
      let source = "eos.aether_profile.owner";
      let (level, message) = match (op, reply["ok"] == true) {
        ("run", true) => ("info", "EOS run ready".to_owned()),
        ("run", false) => (
          "error",
          format!(
            "EOS run failed: {}",
            reply["error"].as_str().unwrap_or("unknown")
          ),
        ),
        ("stop", true) => ("info", "EOS run stopped".to_owned()),
        ("stop", false) => (
          "error",
          format!(
            "EOS stop unconfirmed: {}",
            reply["error"].as_str().unwrap_or("unknown")
          ),
        ),
        _ => unreachable!(),
      };
      let _ = super::eos_log::append_with_run(level, source, &message, request["run_id"].as_str());
    }
    return write_json(stream, lifecycle_http_status(op, &reply), &reply);
  }
  // The persistent transport has its own v1 envelope; owner setup uses v3.
  request["schema_version"] = json!(3);
  let reply = super::setup::handle(&request, registry);
  write_json(stream, if reply["ok"] == true { 200 } else { 422 }, &reply)
}

fn report_worker_loss(state: &mut State, live: bool) {
  if !live && !state.run_id.is_empty() && !state.worker_loss_reported {
    state.worker_loss_reported = true;
    let _ = super::eos_log::append_with_run(
      "error",
      "eos.aether_profile.owner",
      "EOS health worker unavailable",
      Some(&state.run_id),
    );
  }
}

fn run_native(
  request: &Value,
  registry: &NodeTypeRegistry,
  state: &mut State,
  socket: &Path,
  profile: &Profile,
  run_id: &str,
  revision: &str,
) -> Value {
  for transfer in profile
    .all_nodes()
    .filter(|node| node.node_type == "eos.interface.egress.vision_flow@v0.2")
  {
    let Some(source) = transfer
      .inputs
      .get("frame")
      .and_then(|sources| sources.values().first())
    else {
      continue;
    };
    let Some(source_node) = profile.node(&source.node) else {
      continue;
    };
    let Some(source_type) = registry.get(&source_node.node_type) else {
      continue;
    };
    let source_abi = source_type
      .implementation
      .as_ref()
      .and_then(|implementation| implementation.abi.as_deref());
    if matches!(source_abi, Some("isos_node_v1" | "isos_node_v2")) {
      let frequency = source_node
        .adapter
        .settings
        .get("frequency_hz")
        .and_then(|value| {
          value
            .as_float()
            .or_else(|| value.as_integer().map(|number| number as f64))
        });
      if frequency.is_none_or(|hz| !hz.is_finite() || hz <= 0.0 || hz > 1000.0) {
        return json!({"schema_version":1,"core":"eos","ok":false,
          "error":"source_frequency_hz_required",
          "diagnostic":format!("Vision source {} for {} uses an observe callback; set adapter.settings.frequency_hz to a finite value above 0 and at most 1000", source.node, transfer.path())});
      }
    }
  }
  let demo_nodes: BTreeSet<String> = if profile
    .all_nodes()
    .any(|node| node.node_type == "eos.hardware.mock.arm@v0.2")
  {
    match super::setup::demo_projection(profile) {
      Ok(projection) => projection
        .all_nodes()
        .map(|node| node.path().to_owned())
        .collect(),
      Err(reason) => {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph","diagnostic":reason})
      }
    }
  } else {
    BTreeSet::new()
  };
  let process = state.package_inputs.process.clone();
  let process_socket = socket.with_file_name("eos-package.sock");
  if let Some(launch) = &process {
    if !profile.all_nodes().any(|node| {
      node.path() == launch.node_path()
        && node.node_type == launch.node_type()
        && node.package.as_ref().is_some_and(|pin| {
          pin.id == launch.package_id() && pin.version == launch.package_version()
        })
    }) {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"package_process_not_in_graph"});
    }
  }
  if super::package_runtime::validate_inputs(&state.package_inputs, registry).is_err() {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"package_runtime_changed"});
  }
  let mut nodes = Vec::new();
  let mut edges = Vec::new();
  let mut native_inputs = Vec::new();
  let mut native_outputs = BTreeMap::new();
  let mut native_observers = Vec::new();
  let mut vision_transfers = BTreeMap::new();
  let mut external_ports = BTreeMap::new();
  let mut external_types = BTreeMap::new();
  let mut external_frames = BTreeMap::new();
  let mut external_joints = BTreeMap::new();
  let mut external_limits = BTreeMap::new();
  let mut external_bindings = Vec::new();
  let mut service_endpoints = Vec::new();
  for node in profile.all_nodes() {
    if demo_nodes.contains(node.path()) {
      continue;
    }
    let Some(definition) = registry.get(&node.node_type) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"unknown_node_type"});
    };
    if node.node_type == "eos.interface.egress.vision_flow@v0.2" {
      let Some(source) = node
        .inputs
        .get("frame")
        .and_then(|sources| sources.values().first())
      else {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_source_missing"});
      };
      vision_transfers.insert(node.path().to_owned(), source.node.clone());
      continue;
    }
    if process
      .as_ref()
      .is_some_and(|launch| launch.node_path() == node.path())
    {
      continue;
    }
    let service = match service_handoff(request, node) {
      Ok(service) => service,
      Err(error) => return json!({"schema_version":1,"core":"eos","ok":false,"error":error}),
    };
    if let Some(handoff) = service {
      if let Err(error) = probe_service_handoff(
        handoff,
        node,
        run_id,
        revision,
        Instant::now() + Duration::from_secs(2),
      ) {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":error});
      }
      service_endpoints.push((handoff.clone(), node.clone()));
    }
    let library = if service.is_some() && definition.kind != NodeKind::Software {
      let pin = node.adapter.package.as_ref().expect("checked service pin");
      if registry
        .package_for_adapter_family(&node.adapter.adapter_type)
        .is_none_or(|registered| registered.id != pin.id || registered.version != pin.version)
      {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"service_adapter_pin_mismatch"});
      }
      match registry.verified_adapter_library_for_family(&node.adapter.adapter_type) {
        Ok(path) => path,
        Err(_) => {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"adapter_library_unavailable"})
        }
      }
    } else if definition.kind == NodeKind::Camera {
      let family = &node.adapter.adapter_type;
      if process
        .as_ref()
        .is_none_or(|launch| launch.lifecycle_adapter() != family)
      {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"camera_process_binding_missing"});
      }
      let exact_pin = |pin: &aether_hal::PackageRef| {
        process.as_ref().is_some_and(|launch| {
          pin.id == launch.package_id() && pin.version == launch.package_version()
        })
      };
      if node
        .adapter
        .package
        .as_ref()
        .is_none_or(|pin| !exact_pin(pin))
        || node.package.as_ref().is_some_and(|pin| !exact_pin(pin))
        || registry
          .package_for_adapter_family(family)
          .is_none_or(|pin| !exact_pin(pin))
      {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"camera_package_binding_mismatch"});
      }
      match registry.verified_adapter_library_for_family(family) {
        Ok(path) => path,
        Err(_) => {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"adapter_library_unavailable"})
        }
      }
    } else if definition.kind == NodeKind::Software {
      match registry.verified_native_library_for_type(&node.node_type) {
        Ok(path) => path,
        Err(_) => {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"native_library_unavailable"})
        }
      }
    } else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"native_actuator_requires_dispatcher"});
    };
    let mut instance = json!({"path":node.path(),"library":library,"node":node});
    if let Some(service) = service {
      instance["node"]["adapter"]["settings"]["socket_path"] = service["socket_path"].clone();
      instance["node"]["adapter"]["settings"]["service_generation"] =
        service["service_generation"].clone();
      instance["node"]["adapter"]["settings"]["config_digest"] = service["config_digest"].clone();
      if !definition.outputs.is_empty() {
        native_observers.push(node.path().to_owned());
      }
      if definition.kind != NodeKind::Camera {
        external_types.insert(node.path().to_owned(), node.node_type.clone());
        external_frames.insert(node.path().to_owned(), node.frame_id.clone());
        if let Some(joints) = &node.joint_names {
          external_joints.insert(node.path().to_owned(), joints.clone());
        }
        if let Some(movement) = &node.movement {
          external_limits.insert(
            node.path().to_owned(),
            serde_json::to_value(&movement.limits).unwrap_or(Value::Null),
          );
        }
        for port in &definition.inputs {
          external_ports.insert(
            (node.path().to_owned(), port.name.clone()),
            port.data_type.id().to_owned(),
          );
          external_bindings.push(super::external_dispatcher::ExternalBinding {
            node: node.path().to_owned(),
            port: port.name.clone(),
          });
        }
      }
    } else if definition.kind == NodeKind::Camera {
      let declared_socket = process.as_ref().and_then(|launch| {
        profile
          .node(launch.node_path())
          .and_then(|backend| backend.settings.get("socket"))
          .and_then(|value| value.as_str())
      });
      let camera_socket = instance["node"]["adapter"]["settings"]["socket_path"].as_str();
      if camera_socket != Some("${socket}")
        && (declared_socket.is_none() || camera_socket != declared_socket)
      {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"camera_socket_binding_invalid"});
      }
      instance["node"]["adapter"]["settings"]["socket_path"] = json!(process_socket);
      native_observers.push(node.path().to_owned());
    }
    instance["node_type"] = json!(node.node_type);
    nodes.push(instance);
    for output in &definition.outputs {
      native_outputs.insert(
        (node.path().to_owned(), output.name.clone()),
        output.data_type.id().to_owned(),
      );
    }
    for (input, sources) in &node.inputs {
      let Some(data_type) = definition
        .inputs
        .iter()
        .find(|port| port.name == *input)
        .map(|port| port.data_type.id())
      else {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"native_input_port_invalid"});
      };
      for source in sources.values() {
        native_inputs.push(InputLink {
          source: source.node.clone(),
          output: source.output.clone(),
          target: node.path().into(),
          input: input.clone(),
          data_type: data_type.to_owned(),
        });
        if source.node.starts_with("eos/") && !demo_nodes.contains(&source.node) {
          edges.push(
            json!({"source":source.node,"output":source.output,"target":node.path(),"input":input}),
          );
        }
      }
    }
  }
  if nodes.is_empty() {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph"});
  }
  let ei_socket = request["endpoints"]["ei_owner_socket"]["path"]
    .as_str()
    .map(std::path::PathBuf::from);
  let mut ei_links = Vec::new();
  if let Some(links) = request["endpoints"]["eos_to_ei_edges"].as_array() {
    for link in links {
      let Some((source, output, target, input)) = link["source"]
        .as_str()
        .zip(link["output"].as_str())
        .zip(link["target"].as_str().zip(link["input"].as_str()))
        .map(|((source, output), (target, input))| (source, output, target, input))
      else {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_ei_route"});
      };
      if !target.starts_with("ei/")
        || !profile.all_nodes().any(|node| {
          node.path() == source
            && registry
              .get(&node.node_type)
              .is_some_and(|definition| definition.outputs.iter().any(|port| port.name == output))
        })
      {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_ei_route"});
      }
      ei_links.push(EiLink {
        source: source.into(),
        output: output.into(),
        target: target.into(),
        input: input.into(),
      });
    }
  }
  if !ei_links.is_empty()
    && ei_socket
      .as_ref()
      .is_none_or(|socket| !socket.is_absolute())
  {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_ei_socket"});
  }
  let executable = match std::env::current_exe() {
    Ok(path) => path.with_file_name("aether-runtime"),
    Err(_) => {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"runtime_unavailable"})
    }
  };
  let Ok(bytes) = fs::read(&executable) else {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"runtime_unavailable"});
  };
  if request["launch"]["sha256"] != format!("{:x}", Sha256::digest(&bytes))
    || request["launch"]["args"]
      .as_array()
      .is_none_or(|args| !args.is_empty())
    || request["launch"]["env"]
      .as_object()
      .is_none_or(|env| !env.is_empty())
  {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"launch_mismatch"});
  }
  let worker_socket = socket.with_file_name("eos-run.sock");
  if let Some(launch) = &process {
    if process_socket.exists() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"package_socket_conflict"});
    }
    let args = match launch.arguments(&process_socket) {
      Ok(args) => args,
      Err(_) => {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"package_process_arguments_invalid"})
      }
    };
    match super::package_runtime::ProcessChild::spawn(launch, &args) {
      Ok(child) => state.package_child = Some(child),
      Err(_) => {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"package_process_start_failed"})
      }
    }
    state.package_socket = process_socket.clone();
    for _ in 0..100 {
      if process_socket.exists() {
        break;
      }
      std::thread::sleep(Duration::from_millis(20));
    }
    if !process_socket.exists() {
      if let Some(child) = state.package_child.as_mut() {
        let _ = child.stop_with_proof(|| Err("adapter Stop unavailable".into()));
      }
      state.package_child = None;
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"package_process_not_ready"});
    }
  }
  if worker_socket.exists() {
    let _ = fs::remove_file(&worker_socket);
  }
  let admission_token = if external_bindings.is_empty() {
    String::new()
  } else {
    match new_admission_token() {
      Ok(token) => token,
      Err(_) => {
        abort_package_child(state);
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"dispatcher_token_unavailable"});
      }
    }
  };
  let vision_token = if vision_transfers.is_empty() {
    String::new()
  } else {
    match new_admission_token() {
      Ok(token) => token,
      Err(_) => {
        abort_package_child(state);
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_token_unavailable"});
      }
    }
  };
  let external_dispatcher = if external_bindings.is_empty() {
    None
  } else {
    let path = socket.with_file_name(format!("eos-external-commands-{run_id}.jsonl"));
    match super::external_dispatcher::ExternalDispatcher::open(
      &path,
      run_id,
      revision,
      external_bindings,
    ) {
      Ok(dispatcher) => Some(dispatcher),
      Err(_) => {
        abort_package_child(state);
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"dispatcher_unavailable"});
      }
    }
  };
  let builtin = NodeTypeRegistry::builtin();
  let output_types: Vec<_> = native_outputs
    .iter()
    .map(|((node, port), data_type)| json!({"node":node,"port":port,"data_type":data_type}))
    .collect();
  let custom_ids: BTreeSet<_> = native_outputs
    .values()
    .filter(|id| builtin.data_type(id).is_none())
    .collect();
  let mut data_types = Vec::new();
  for id in custom_ids {
    let Some(descriptor) = registry.data_type(id) else {
      abort_package_child(state);
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"native_output_type_unavailable"});
    };
    data_types.push(descriptor);
  }
  let config = json!({"run_id":run_id,"revision":revision,"nodes":nodes,"edges":edges,
    "output_types":output_types,"data_types":data_types,"admission_token":admission_token,"vision_token":vision_token,
    "vision_sources":vision_transfers.values().collect::<Vec<_>>()});
  let Ok(config_bytes) = serde_json::to_vec(&config) else {
    abort_package_child(state);
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"native_graph_invalid"});
  };
  if config_bytes.len() > 1024 * 1024 {
    abort_package_child(state);
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"native_graph_too_large"});
  }
  let mut command = Command::new(executable);
  command
    .arg("--native-graph")
    .arg(&worker_socket)
    .arg(run_id)
    .arg(revision)
    .stdin(std::process::Stdio::piped());
  let parent_pid = unsafe { libc::getpid() };
  unsafe {
    command.pre_exec(move || {
      if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 || libc::getppid() != parent_pid {
        return Err(std::io::Error::other(
          "EOS owner exited during native worker spawn",
        ));
      }
      Ok(())
    });
  }
  let Ok(mut child) = command.spawn() else {
    abort_package_child(state);
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"runtime_start_failed"});
  };
  if child
    .stdin
    .take()
    .is_none_or(|mut input| input.write_all(&config_bytes).is_err())
  {
    let _ = child.kill();
    let _ = child.wait();
    abort_package_child(state);
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"runtime_config_failed"});
  }
  state.child = Some(child);
  state.run_id = run_id.into();
  state.revision = revision.into();
  state.worker_socket = worker_socket;
  state.worker_loss_reported = false;
  state.native = true;
  state.ei_socket = ei_socket;
  state.ei_links = ei_links;
  state.native_inputs = native_inputs;
  state.native_outputs = native_outputs;
  state.service_endpoints = service_endpoints;
  state.native_observers = native_observers;
  state.vision_transfers = vision_transfers;
  state.vision_token = vision_token;
  state.external_ports = external_ports;
  state.external_types = external_types;
  state.external_frames = external_frames;
  state.external_joints = external_joints;
  state.external_limits = external_limits;
  state.external_dispatcher = external_dispatcher;
  state.admission_token = admission_token;
  for _ in 0..40 {
    if state.worker_socket.exists() {
      break;
    }
    std::thread::sleep(Duration::from_millis(25));
  }
  let ready = worker_state(&state.worker_socket, run_id) == Some("ready")
    && state
      .child
      .as_mut()
      .is_some_and(|child| child.try_wait().ok().flatten().is_none());
  report_worker_loss(state, ready);
  if !ready {
    let stopped = rollback_native_start(state);
    return json!({"schema_version":1,"core":"eos","ok":false,"state":if stopped {"failed"} else {"stop_uncertain"},"run_id":run_id,"revision":revision,"error":"runtime_start_failed"});
  }
  json!({"schema_version":1,"core":"eos","ok":true,"state":"ready","run_id":run_id,"revision":revision,"worker_pid":state.child.as_ref().map(Child::id),"error":null})
}

fn run_demo(
  request: &Value,
  state: &mut State,
  socket: &Path,
  profile: &Profile,
  run_id: &str,
  revision: &str,
  worker_name: &str,
) -> Value {
  if profile.validate_demo_chain().is_err()
    || request["graph"]["cores"]
      .as_object()
      .is_none_or(|cores| cores.len() != 1 || !cores.contains_key("eos"))
  {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph"});
  }
  state.demo_activation_enabled = profile.demo_activation_chain_complete();
  state.demo_monitor_enabled = profile.demo_monitor_chain_complete();
  let Some(arm_node) = Some(profile).and_then(|profile| {
    profile
      .all_nodes()
      .find(|node| node.node_type == "eos.hardware.mock.arm@v0.2")
  }) else {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph"});
  };
  let Some(joint_limit_rad) = arm_node
    .movement
    .as_ref()
    .and_then(|movement| movement.limits.max_joint_position_rad)
  else {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph"});
  };
  let Some(joint_names) = arm_node.joint_names.as_ref() else {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph"});
  };
  if joint_names.len() != 6 {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph"});
  }
  let joint_names_csv = joint_names.join(",");
  let arm_path = arm_node.path().to_owned();
  let Some((observer_path, legacy_output)) = Some(profile)
    .and_then(|profile| {
      profile.all_nodes().find(|node| {
        matches!(
          node.node_type.as_str(),
          "eos.hardware.mock.arm_observer@v0.2" | "eos.hardware.mock.arm_output@v0.2"
        )
      })
    })
    .map(|node| {
      (
        node.path().to_owned(),
        node.node_type == "eos.hardware.mock.arm_output@v0.2",
      )
    })
  else {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph"});
  };
  let sample_interval_ms = arm_node
    .settings
    .get("sample_interval_ms")
    .and_then(|value| value.as_integer())
    .unwrap_or(10);
  let freshness_timeout_ms = arm_node
    .settings
    .get("freshness_timeout_ms")
    .and_then(|value| value.as_integer())
    .unwrap_or(50);
  let observation_source_path = if legacy_output {
    arm_path.clone()
  } else {
    observer_path.clone()
  };
  let executable = match std::env::current_exe() {
    Ok(path) => path.with_file_name("aether-runtime"),
    Err(_) => {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"runtime_unavailable"});
    }
  };
  let bytes = match fs::read(&executable) {
    Ok(bytes) => bytes,
    Err(_) => {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"runtime_unavailable"});
    }
  };
  let digest = format!("{:x}", Sha256::digest(&bytes));
  if request["launch"]["sha256"] != digest
    || !request["launch"]["args"]
      .as_array()
      .is_some_and(Vec::is_empty)
    || !request["launch"]["env"]
      .as_object()
      .is_some_and(serde_json::Map::is_empty)
  {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"launch_mismatch"});
  }
  let worker_socket = socket.with_file_name(worker_name);
  if worker_socket.exists() {
    let _ = fs::remove_file(&worker_socket);
  }
  let mut command = Command::new(&executable);
  command
    .arg("--serve")
    .arg(&worker_socket)
    .arg("--run-id")
    .arg(run_id)
    .arg("--revision")
    .arg(revision)
    .arg("--joint-limit")
    .arg(joint_limit_rad.to_string())
    .arg("--source-node")
    .arg(arm_path)
    .arg("--observer-node")
    .arg(observer_path)
    .arg("--observation-source-node")
    .arg(observation_source_path)
    .arg("--joint-names")
    .arg(joint_names_csv)
    .arg("--sample-interval-ms")
    .arg(sample_interval_ms.to_string())
    .arg("--freshness-timeout-ms")
    .arg(freshness_timeout_ms.to_string());
  let parent_pid = unsafe { libc::getpid() };
  unsafe {
    command.pre_exec(move || {
      if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
        return Err(std::io::Error::last_os_error());
      }
      if libc::getppid() != parent_pid {
        return Err(std::io::Error::other(
          "EOS owner exited during worker spawn",
        ));
      }
      Ok(())
    });
  }
  let child = match command.spawn() {
    Ok(child) => child,
    Err(_) => {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"runtime_start_failed"});
    }
  };
  state.child = Some(child);
  state.run_id = run_id.into();
  state.revision = revision.into();
  state.worker_socket = worker_socket;
  state.worker_loss_reported = false;
  state.native = false;
  for _ in 0..40 {
    if state.worker_socket.exists() {
      break;
    }
    std::thread::sleep(Duration::from_millis(25));
  }
  let ready = worker_state(&state.worker_socket, run_id) == Some("ready")
    && state
      .child
      .as_mut()
      .is_some_and(|child| child.try_wait().ok().flatten().is_none());
  json!({"schema_version":1,"core":"eos","ok":ready,"state":if ready {"ready"} else {"failed"},"run_id":run_id,"revision":revision,"worker_pid":state.child.as_ref().map(Child::id),"error":if ready {Value::Null} else {json!("runtime_start_failed")}})
}

fn lifecycle(
  op: &str,
  request: &Value,
  registry: &NodeTypeRegistry,
  state: &mut State,
  socket: &Path,
) -> Value {
  sync_cross_sampler(state);
  let run_id = request["run_id"].as_str().unwrap_or("");
  let revision = request["revision"].as_str().unwrap_or("");
  if run_id.is_empty()
    || run_id.len() > 128
    || !run_id
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    || revision.len() != 64
    || !revision.bytes().all(|byte| byte.is_ascii_hexdigit())
  {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_run_identity"});
  }
  let child_live = state
    .child
    .as_mut()
    .is_some_and(|child| child.try_wait().ok().flatten().is_none());
  // The latest-output request itself probes the worker and reports a failed
  // source. A separate status round trip for every frame serializes capture
  // behind two worker requests and substantially reduces delivered frame rate.
  let worker = if matches!(op, "latest_output" | "vision_subscribe") {
    child_live.then_some("ready")
  } else {
    child_live
      .then(|| worker_state(&state.worker_socket, &state.run_id))
      .flatten()
  };
  let mock_worker = if state.mixed {
    state.mock_child.as_mut().and_then(|child| {
      child
        .try_wait()
        .ok()
        .flatten()
        .is_none()
        .then(|| {
          if matches!(op, "latest_output" | "vision_subscribe") {
            Some("ready")
          } else {
            worker_state(&state.mock_socket, &state.run_id)
          }
        })
        .flatten()
    })
  } else {
    None
  };
  let combined = aggregate_worker_state(worker, mock_worker, state.mixed);
  if op == "status" || op == "run" {
    check_selected_services(state);
  }
  let live = combined.is_some() && state.cross_error.is_none();
  report_worker_loss(state, combined.is_some());
  if op == "run" {
    if !state.run_id.is_empty() {
      let same = state.run_id == run_id && state.revision == revision;
      return json!({"schema_version":1,"core":"eos","ok":same && live,"state":if live {combined.unwrap_or("failed")} else {"failed"},"run_id":state.run_id,"revision":state.revision,"error":if same && live {Value::Null} else {json!("run_conflict")}});
    }
    let Some(graph) = request.get("graph") else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"missing_graph"});
    };
    let parsed = serde_json::to_string(graph)
      .ok()
      .and_then(|source| Profile::from_json_with_registry(&source, registry).ok());
    if let Some(profile) = parsed.as_ref() {
      match super::setup::execution_mode(profile, registry) {
        Ok(super::setup::ExecutionMode::Native) => {
          return run_native(request, registry, state, socket, profile, run_id, revision)
        }
        Ok(super::setup::ExecutionMode::Mixed) => {
          let demo = match super::setup::demo_projection(profile) {
            Ok(projection) => projection,
            Err(reason) => {
              return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph","diagnostic":reason});
            }
          };
          let observer = demo
            .all_nodes()
            .find(|node| {
              matches!(
                node.node_type.as_str(),
                "eos.hardware.mock.arm_observer@v0.2" | "eos.hardware.mock.arm_output@v0.2"
              )
            })
            .map(|node| node.path().to_owned())
            .unwrap_or_default();
          let plan = match super::setup::route_plan(profile, registry) {
            Ok(plan) => plan,
            Err(reason) => {
              return json!({"schema_version":1,"core":"eos","ok":false,
              "error":"unsupported_execution","diagnostic":reason})
            }
          };
          if plan.links.iter().any(|link| {
            plan.workers.get(&link.source) == Some(&super::setup::WorkerKind::Demo)
              && plan.workers.get(&link.target) == Some(&super::setup::WorkerKind::Native)
              && (link.source != observer || link.output != "observation")
          }) {
            return json!({"schema_version":1,"core":"eos","ok":false,
              "error":"unsupported_execution","diagnostic":"unsupported_mock_cross_worker_source"});
          }
          let native = run_native(request, registry, state, socket, profile, run_id, revision);
          if native["ok"] != true {
            return native;
          }
          let mut mock_state = State::default();
          let result = run_demo(
            request,
            &mut mock_state,
            socket,
            &demo,
            run_id,
            revision,
            "eos-mock-run.sock",
          );
          if result["ok"] != true {
            return rollback_mixed_start_failure(state, &mut mock_state, result);
          }
          state.mock_child = mock_state.child.take();
          state.demo_activation_enabled = mock_state.demo_activation_enabled;
          state.demo_monitor_enabled = mock_state.demo_monitor_enabled;
          state.mock_socket = mock_state.worker_socket;
          state.mock_observer = observer;
          state.cross_links = plan
            .links
            .into_iter()
            .filter(|link| {
              plan.workers.get(&link.source) == Some(&super::setup::WorkerKind::Demo)
                && plan.workers.get(&link.target) == Some(&super::setup::WorkerKind::Native)
            })
            .map(|link| CrossLink {
              source: link.source,
              output: link.output,
              target: link.target,
              input: link.input,
              data_type: link.data_type,
            })
            .collect();
          state.mixed = true;
          if !state.cross_links.is_empty() {
            start_cross_sampler(state, registry);
          }
          return json!({"schema_version":1,"core":"eos","ok":true,"state":"ready","run_id":run_id,"revision":revision,
            "worker_pid":state.child.as_ref().map(Child::id),"mock_worker_pid":state.mock_child.as_ref().map(Child::id),"error":null});
        }
        Ok(super::setup::ExecutionMode::Demo) => {}
        Err(reason) => {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"unsupported_execution","diagnostic":reason})
        }
      }
    }
    let Some(profile) = parsed.as_ref() else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph"});
    };
    return run_demo(
      request,
      state,
      socket,
      profile,
      run_id,
      revision,
      "eos-run.sock",
    );
  }
  if (state.child.is_none() && op != "stop") || state.run_id != run_id || state.revision != revision
  {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"stale_or_missing_run"});
  }
  if op == "vision_frame" {
    let transfer = request["transfer_path"].as_str().unwrap_or("");
    let source = request["source_path"].as_str().unwrap_or("");
    if !state.native
      || !live
      || state.vision_transfers.get(transfer).map(String::as_str) != Some(source)
      || !state.native_observers.iter().any(|path| path == source)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_route_unavailable"});
    }
    let reply = match native_worker_exchange(
      &state.worker_socket,
      &json!({"op":"observe","run_id":run_id,"revision":revision,"node":source}),
    ) {
      Ok(reply) if reply["ok"] == true => reply,
      _ => {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_source_failed"})
      }
    };
    let frames: Vec<_> = reply["outputs"]
      .as_array()
      .into_iter()
      .flatten()
      .filter(|entry| entry["node"] == source && entry["output"]["port"] == "frame")
      .collect();
    let Some(frame) = (frames.len() == 1).then(|| &frames[0]["output"]["payload"]) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_frame_missing"});
    };
    if aether_hal::vision_flow::validate_frame(frame, registry).is_err() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_frame_invalid"});
    }
    return json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,
      "transfer_path":transfer,"source_path":source,"frame":frame});
  }
  if op == "vision_subscribe" {
    let transfer = request["transfer_path"].as_str().unwrap_or("");
    let source = request["source_path"].as_str().unwrap_or("");
    if !state.native
      || !live
      || state.vision_transfers.get(transfer).map(String::as_str) != Some(source)
      || !state.native_observers.iter().any(|path| path == source)
      || state.vision_token.len() != 64
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_route_unavailable"});
    }
    return json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,
      "transfer_path":transfer,"source_path":source,"worker_socket":state.worker_socket,
      "vision_token":state.vision_token});
  }
  if op == "latest_output" {
    let transfer = request["transfer_path"].as_str().unwrap_or("");
    let source = request["source_path"].as_str().unwrap_or("");
    if !state.native
      || !live
      || state.vision_transfers.get(transfer).map(String::as_str) != Some(source)
      || !state.native_observers.iter().any(|path| path == source)
      || request["node"] != transfer
      || request["port"] != "frame"
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_route_unavailable"});
    }
    let after = request["after_sequence"].as_u64().unwrap_or(0);
    let reply = match native_worker_exchange(
      &state.worker_socket,
      &json!({"op":"latest_output","run_id":run_id,"revision":revision,
        "node":source,"port":"frame","after_sequence":after}),
    ) {
      Ok(reply)
        if reply["ok"] == true && reply["run_id"] == run_id && reply["revision"] == revision =>
      {
        reply
      }
      _ => {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_source_failed"})
      }
    };
    let sample = &reply["sample"];
    if !sample.is_null()
      && (sample["node"] != source
        || sample["output"]["port"] != "frame"
        || sample["sequence"].as_u64().is_none()
        || aether_hal::vision_flow::validate_frame(&sample["output"]["payload"], registry).is_err())
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"vision_frame_invalid"});
    }
    let transfer_sample = if sample.is_null() {
      Value::Null
    } else {
      json!({"node":transfer,"output":sample["output"],"sequence":sample["sequence"]})
    };
    return json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,
      "transfer_path":transfer,"source_path":source,"sample":transfer_sample});
  }
  if op == "observe" && state.mixed && request["node"] == state.mock_observer {
    if !live {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    if let Some(sampler) = state.cross_sampler.as_ref() {
      if let Ok(snapshot) = sampler.snapshot.lock() {
        if let Some(reply) = snapshot.latest.as_ref() {
          return reply.clone();
        }
      }
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"observation_unavailable"});
    }
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"observation_unavailable"});
  }
  if op == "input" || op == "command" || op == "observe" {
    if !state.native || !live || (op != "observe" && request["payload"].is_null()) {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"input_unavailable"});
    }
    if op == "observe"
      && !state
        .native_observers
        .iter()
        .any(|node| request["node"] == node.as_str())
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"observer_not_declared"});
    }
    if op == "input" {
      let Some(link) = state.native_inputs.iter().find(|link| {
        request["source_node"] == link.source
          && request["source_port"] == link.output
          && request["node"] == link.target
          && request["port"] == link.input
      }) else {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"input_route_mismatch"});
      };
      if link.source.starts_with("eos/") {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"cross_worker_owner_only"});
      }
      if !link.source.starts_with("ei/") && !link.source.starts_with("isos/") {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"input_source_untrusted"});
      }
      if registry.data_type(&link.data_type).is_none()
        || registry
          .validate_data_payload(&link.data_type, &request["payload"])
          .is_err()
      {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_input_payload"});
      }
    }
    if op == "input" || op == "command" {
      let node = request["node"].as_str().unwrap_or("");
      let port = request["port"].as_str().unwrap_or("");
      if let Some(data_type) = state
        .external_ports
        .get(&(node.to_owned(), port.to_owned()))
      {
        if registry
          .validate_data_payload(data_type, &request["payload"])
          .is_err()
        {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_command_payload"});
        }
        if data_type == "base_twist_command@v0.2"
          && state
            .external_frames
            .get(node)
            .is_none_or(|frame| request["payload"]["frame_id"] != *frame)
        {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"command_frame_mismatch"});
        }
        let limits = state.external_limits.get(node).unwrap_or(&Value::Null);
        if data_type == "base_twist_command@v0.2"
          && request["payload"]["lease_ms"]
            .as_u64()
            .zip(limits["max_velocity_lease_ms"].as_u64())
            .is_none_or(|(lease, max)| lease > max)
        {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"command_lease_invalid"});
        }
        if data_type == "normalized_axes_command@v0.2"
          && request["payload"]["lease_ms"]
            .as_u64()
            .zip(limits["max_velocity_lease_ms"].as_u64())
            .is_none_or(|(lease, max)| lease > max)
        {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"command_lease_invalid"});
        }
        if data_type == "base_twist_command@v0.2"
          && [
            ("vx_m_s", "max_linear_m_s"),
            ("vy_m_s", "max_linear_m_s"),
            ("yaw_rate_rad_s", "max_angular_rad_s"),
          ]
          .iter()
          .any(|(field, limit)| {
            request["payload"][field]
              .as_f64()
              .zip(limits[limit].as_f64())
              .is_none_or(|(value, max)| !value.is_finite() || value.abs() > max)
          })
        {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"command_limit_exceeded"});
        }
        if data_type == "joint_position_command@v0.2" {
          let actual = request["payload"]["joint_names"].as_array();
          let positions = request["payload"]["position_rad"].as_array();
          if actual.is_none_or(|actual| {
            actual.len() != 6
              || positions.is_none_or(|positions| {
                positions.len() != actual.len()
                  || positions.iter().any(|value| {
                    value
                      .as_f64()
                      .zip(limits["max_joint_position_rad"].as_f64())
                      .is_none_or(|(number, max)| !number.is_finite() || number.abs() > max)
                  })
              })
              || state.external_joints.get(node).is_some_and(|declared| {
                actual
                  .iter()
                  .map(|v| v.as_str().unwrap_or(""))
                  .ne(declared.iter().map(String::as_str))
              })
          }) {
            return json!({"schema_version":1,"core":"eos","ok":false,"error":"command_joint_mismatch"});
          }
        }
        let Some(node_type) = state.external_types.get(node).map(String::as_str) else {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"command_node_missing"});
        };
        let Ok(payload) = translate_external_command(node_type, port, &request["payload"]) else {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_command_mapping"});
        };
        let Some(dispatcher) = state.external_dispatcher.as_ref() else {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"dispatcher_unavailable"});
        };
        let command_id = request["command_id"].as_str().unwrap_or("");
        let body = json!({"op":"command","run_id":run_id,"revision":revision,"node":node,"port":port,"payload":payload,"command_id":command_id,"stop_epoch":"0","admission_token":state.admission_token});
        let outcome = dispatcher.submit(command_id, node, port, &request["payload"], || {
          let reply = native_worker_exchange(&state.worker_socket, &body)?;
          let accepted = reply["outputs"].as_array().is_some_and(|outputs| {
            let mut results = outputs.iter().filter(|output| {
              output["node"] == node && output["output"]["port"] == "command_result"
            });
            results
              .next()
              .is_some_and(|result| result["output"]["payload"]["accepted"] == true)
              && results.next().is_none()
          });
          if reply["ok"] == true && accepted {
            Ok(())
          } else {
            Err(
              reply["error"]
                .as_str()
                .unwrap_or("command outcome unknown")
                .into(),
            )
          }
        });
        return json!({"schema_version":1,"core":"eos","ok":outcome.outcome==super::external_dispatcher::ExternalOutcome::Completed,"run_id":run_id,"revision":revision,"command_id":command_id,"outcome":outcome.outcome,"error":outcome.reason});
      }
      if op == "command" {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"command_port_not_declared"});
      }
    }
    let Ok(mut worker) = UnixStream::connect(&state.worker_socket) else {
      if op == "observe" {
        mark_native_observation_error(state, "worker_unavailable");
      }
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    };
    let _ = worker.set_read_timeout(Some(Duration::from_secs(2)));
    let body = if op == "observe" {
      json!({"op":"observe","run_id":run_id,"revision":revision,"node":request["node"]})
    } else {
      json!({"op":"input","run_id":run_id,"revision":revision,"source_node":request["source_node"],"source_port":request["source_port"],"node":request["node"],"port":request["port"],"payload":request["payload"]})
    };
    if serde_json::to_writer(&mut worker, &body).is_err() {
      if op == "observe" {
        mark_native_observation_error(state, "input_failed");
      }
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"input_failed"});
    }
    let _ = worker.shutdown(std::net::Shutdown::Write);
    let mut bytes = Vec::new();
    if worker
      .take(MAX_NATIVE_REPLY + 1)
      .read_to_end(&mut bytes)
      .is_err()
      || bytes.len() as u64 > MAX_NATIVE_REPLY
    {
      if op == "observe" {
        mark_native_observation_error(state, "input_failed");
      }
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"input_failed"});
    }
    let Ok(reply) = serde_json::from_slice::<Value>(&bytes) else {
      if op == "observe" {
        mark_native_observation_error(state, "input_failed");
      }
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"input_failed"});
    };
    if reply["ok"] != true {
      if op == "observe" {
        mark_native_observation_error(
          state,
          reply["error"].as_str().unwrap_or("observation_failed"),
        );
      }
      return reply;
    }
    for emitted in reply["outputs"].as_array().into_iter().flatten() {
      let source = emitted["node"].as_str().unwrap_or("");
      let port = emitted["output"]["port"].as_str().unwrap_or("");
      let Some(data_id) = state
        .native_outputs
        .get(&(source.to_owned(), port.to_owned()))
      else {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"native_output_port_invalid"});
      };
      if registry.data_type(data_id).is_some()
        && registry
          .validate_data_payload(data_id, &emitted["output"]["payload"])
          .is_err()
      {
        return json!({"schema_version":1,"core":"eos","ok":false,"error":"native_output_payload_invalid"});
      }
      for link in state
        .ei_links
        .iter()
        .filter(|link| link.source == source && link.output == port)
      {
        let Some(socket) = state.ei_socket.as_deref() else {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"ei_route_unavailable"});
        };
        if forward_ei_input(
          socket,
          run_id,
          revision,
          link,
          &emitted["output"]["payload"],
        )
        .is_err()
        {
          return json!({"schema_version":1,"core":"eos","ok":false,"error":"ei_forward_failed"});
        }
      }
    }
    return json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"outputs":reply["outputs"]});
  }
  if op == "activate" {
    if !state.demo_activation_enabled {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"activation_path_incomplete"});
    }
    if !live {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    let event_id = request["event_id"].as_str().unwrap_or("");
    if event_id.is_empty()
      || event_id.len() > 64
      || !event_id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
      || request["kind"] != "activate"
      || !request["source_node"]
        .as_str()
        .is_some_and(valid_button_source)
      || request.as_object().is_none_or(|fields| fields.len() != 8)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_activation"});
    }
    let activation_socket = if state.mixed {
      &state.mock_socket
    } else {
      &state.worker_socket
    };
    let Ok(mut worker) = UnixStream::connect(activation_socket) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    };
    let _ = worker.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = worker.set_write_timeout(Some(Duration::from_secs(3)));
    let Ok(bytes) = serde_json::to_vec(request) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_activation"});
    };
    if worker.write_all(&bytes).is_err() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"outcome_unknown"});
    }
    let _ = worker.shutdown(std::net::Shutdown::Write);
    let mut reply = Vec::new();
    if worker.take(4096).read_to_end(&mut reply).is_err() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"outcome_unknown"});
    }
    return serde_json::from_slice(&reply).unwrap_or_else(
      |_| json!({"schema_version":1,"core":"eos","ok":false,"error":"outcome_unknown"}),
    );
  }
  if op == "result" {
    if request.as_object().is_none_or(|fields| fields.len() != 6)
      || !request["event_id"].as_str().is_some_and(|event| {
        !event.is_empty()
          && event.len() <= 64
          && event
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
      })
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_result_request"});
    }
    if !live {
      return journal_result(state, request["event_id"].as_str().unwrap());
    }
    let result_socket = if state.mixed {
      &state.mock_socket
    } else {
      &state.worker_socket
    };
    let Ok(mut worker) = UnixStream::connect(result_socket) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    };
    let _ = worker.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = worker.set_write_timeout(Some(Duration::from_secs(2)));
    let Ok(body) = serde_json::to_vec(request) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_result_request"});
    };
    if worker.write_all(&body).is_err() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    let _ = worker.shutdown(std::net::Shutdown::Write);
    let mut bytes = Vec::new();
    if worker.take(4096).read_to_end(&mut bytes).is_err() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    let Ok(reply) = serde_json::from_slice::<Value>(&bytes) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    };
    if reply["ok"] == true
      && (reply["run_id"] != run_id
        || reply["revision"] != revision
        || reply["event_id"] != request["event_id"])
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    return reply;
  }
  if op == "monitor" {
    if !state.demo_monitor_enabled {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"monitor_path_incomplete"});
    }
    if request.as_object().is_none_or(|fields| fields.len() != 5) {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_monitor_request"});
    }
    if !live {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    let monitor_socket = if state.mixed {
      &state.mock_socket
    } else {
      &state.worker_socket
    };
    let Ok(mut worker) = UnixStream::connect(monitor_socket) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    };
    let _ = worker.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = worker.set_write_timeout(Some(Duration::from_secs(2)));
    let Ok(body) = serde_json::to_vec(request) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_monitor_request"});
    };
    if worker.write_all(&body).is_err() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    let _ = worker.shutdown(std::net::Shutdown::Write);
    let mut bytes = Vec::new();
    if worker.take(4096).read_to_end(&mut bytes).is_err() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    let Ok(reply) = serde_json::from_slice::<Value>(&bytes) else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    };
    if reply["ok"] != true
      || reply["run_id"] != run_id
      || reply["revision"] != revision
      || reply["observation"]["run_id"] != run_id
      || reply["observation"]["revision"] != revision
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    return reply;
  }
  if op == "status" {
    let inhibited = state
      .external_dispatcher
      .as_ref()
      .is_some_and(|dispatcher| dispatcher.uncertain());
    return json!({"schema_version":1,"core":"eos","ok":true,"state":if !live {"failed"} else if inhibited && combined==Some("ready") {"degraded"} else {combined.unwrap_or("failed")},"run_id":run_id,"revision":revision,"worker_pid":state.child.as_ref().map(Child::id),"movement_inhibited":inhibited,"observation_samples_skipped":state.skipped_mock_samples,"mock_hardware":if state.mixed {json!(mock_worker.unwrap_or("failed"))} else if state.native {Value::Null} else {json!(worker.unwrap_or("failed"))}});
  }
  if state
    .external_dispatcher
    .as_ref()
    .is_some_and(|dispatcher| dispatcher.close().is_err())
  {
    return json!({"schema_version":1,"core":"eos","ok":false,"state":"stop_uncertain","run_id":run_id,"revision":revision,"error":"dispatcher_admission_close_failed"});
  }
  stop_cross_sampler(state);
  if let Some(child) = state.mock_child.as_mut() {
    if let Err(error) = stop_child(child, &state.mock_socket) {
      return json!({"schema_version":1,"core":"eos","ok":false,"state":"stop_uncertain","run_id":run_id,"revision":revision,"error":error});
    }
    state.mock_child = None;
  }
  if let Some(child) = state.child.as_mut() {
    if let Err(error) = stop_child(child, &state.worker_socket) {
      return json!({"schema_version":1,"core":"eos","ok":false,"state":"stop_uncertain","run_id":run_id,"revision":revision,"error":error});
    }
    state.child = None;
  }
  if let Some(process) = state.package_child.as_mut() {
    // The native worker has already proved camera sampling quiesced. This
    // closure carries no actuator proof; actuator package nodes remain gated.
    if let Err(error) = process.stop_with_proof(|| Ok(())) {
      return json!({"schema_version":1,"core":"eos","ok":false,"state":"stop_uncertain","run_id":run_id,"revision":revision,"error":error});
    }
  }
  state.package_child = None;
  if !state.package_socket.as_os_str().is_empty() {
    let _ = fs::remove_file(&state.package_socket);
  }
  state.package_socket.clear();
  state.child = None;
  state.run_id.clear();
  state.revision.clear();
  state.worker_loss_reported = false;
  state.native = false;
  state.mixed = false;
  state.demo_activation_enabled = false;
  state.demo_monitor_enabled = false;
  state.mock_child = None;
  state.mock_socket.clear();
  state.mock_observer.clear();
  state.cross_links.clear();
  state.last_mock_sequence = 0;
  state.skipped_mock_samples = 0;
  state.cross_error = None;
  state.service_endpoints.clear();
  state.ei_socket = None;
  state.ei_links.clear();
  state.native_inputs.clear();
  state.native_outputs.clear();
  state.native_observers.clear();
  state.vision_transfers.clear();
  state.external_dispatcher = None;
  state.external_ports.clear();
  state.external_types.clear();
  state.external_frames.clear();
  state.external_joints.clear();
  state.external_limits.clear();
  state.admission_token.clear();
  json!({"schema_version":1,"core":"eos","ok":true,"state":"stopped","run_id":run_id,"revision":revision})
}

fn write_json(stream: &mut UnixStream, status: u16, value: &Value) -> Result<(), Box<dyn Error>> {
  let body = serde_json::to_vec(value)?;
  if body.len() > MAX_REPLY {
    return write_json(stream, 500, &json!({"error":"response_too_large"}));
  }
  let reason = match status {
    200 => "OK",
    400 => "Bad Request",
    404 => "Not Found",
    405 => "Method Not Allowed",
    411 => "Length Required",
    413 => "Payload Too Large",
    422 => "Unprocessable Content",
    431 => "Request Header Fields Too Large",
    501 => "Not Implemented",
    _ => "Internal Server Error",
  };
  write!(
    stream,
    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
    body.len()
  )?;
  stream.write_all(&body)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn owner_reply_can_wrap_the_maximum_registered_image_payload() {
    let (mut writer, mut reader) = UnixStream::pair().unwrap();
    let reading = std::thread::spawn(move || {
      let mut bytes = Vec::new();
      reader.read_to_end(&mut bytes).unwrap();
      bytes
    });
    let reply = json!({"ok":true,"frame":{"data":"a".repeat(20 * 1024 * 1024)}});
    write_json(&mut writer, 200, &reply).unwrap();
    drop(writer);
    let bytes = reading.join().unwrap();
    assert!(bytes.starts_with(b"HTTP/1.1 200 OK\r\n"));
    assert!(bytes.len() > 20 * 1024 * 1024);
  }

  #[test]
  fn mixed_status_requires_both_workers_and_preserves_degraded_state() {
    assert_eq!(
      aggregate_worker_state(Some("ready"), Some("ready"), true),
      Some("ready")
    );
    assert_eq!(
      aggregate_worker_state(Some("ready"), Some("degraded"), true),
      Some("degraded")
    );
    assert_eq!(aggregate_worker_state(Some("ready"), None, true), None);
    assert_eq!(aggregate_worker_state(None, Some("ready"), true), None);
  }

  #[test]
  fn mock_observation_crosses_only_the_declared_typed_link() {
    let dir = tempfile::tempdir().unwrap();
    let mock_socket = dir.path().join("mock.sock");
    let native_socket = dir.path().join("native.sock");
    let mock_listener = UnixListener::bind(&mock_socket).unwrap();
    let native_listener = UnixListener::bind(&native_socket).unwrap();
    let revision = "a".repeat(64);
    let source = "eos/hardware/mock/arm_observer";
    let target = "eos/skill/package/reader";
    let payload = json!({"run_id":"run_1","revision":revision,"source_node":"eos/hardware/mock/arm",
      "sequence":4,"sampled_at_monotonic_ns":1,"joint_names":["a","b","c","d","e","f"],
      "joint_positions_rad":[0.0,0.0,0.0,0.0,0.0,0.0],"state":"ready","reason_code":null});
    let mock_payload = payload.clone();
    let mock = std::thread::spawn(move || {
      for _ in 0..2 {
        let (mut stream, _) = mock_listener.accept().unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        let request: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(request["op"], "observe_output");
        assert_eq!(request["node"], source);
        stream.write_all(json!({"schema_version":1,"core":"eos","ok":true,"run_id":"run_1",
          "revision":mock_payload["revision"],"node":source,"port":"observation","payload":mock_payload}).to_string().as_bytes()).unwrap();
      }
    });
    let native = std::thread::spawn(move || {
      let (mut stream, _) = native_listener.accept().unwrap();
      let mut bytes = Vec::new();
      stream.read_to_end(&mut bytes).unwrap();
      let request: Value = serde_json::from_slice(&bytes).unwrap();
      assert_eq!(request["op"], "input");
      assert_eq!(request["node"], target);
      assert_eq!(request["source_node"], source);
      assert_eq!(request["source_port"], "observation");
      assert_eq!(request["payload"], payload);
      stream.write_all(br#"{"ok":true,"outputs":[]}"#).unwrap();
    });
    let mut state = State {
      run_id: "run_1".into(),
      revision,
      mock_socket,
      worker_socket: native_socket,
      mock_observer: source.into(),
      cross_links: vec![CrossLink {
        source: source.into(),
        output: "observation".into(),
        target: target.into(),
        input: "observation".into(),
        data_type: "arm_monitor_state@v0.2".into(),
      }],
      ..State::default()
    };
    let reply = route_mock_observation(&mut state, &NodeTypeRegistry::builtin()).unwrap();
    assert_eq!(reply["ok"], true);
    assert_eq!(reply["outputs"].as_array().unwrap().len(), 1);
    assert_eq!(state.last_mock_sequence, 4);
    assert_eq!(state.skipped_mock_samples, 3);
    assert_eq!(
      route_mock_observation(&mut state, &NodeTypeRegistry::builtin()).unwrap()["ok"],
      true
    );
    mock.join().unwrap();
    native.join().unwrap();
  }

  #[test]
  fn cross_sampler_does_not_block_owner_and_joins_before_worker_stop() {
    let dir = tempfile::tempdir().unwrap();
    let mock_socket = dir.path().join("mock.sock");
    let native_socket = dir.path().join("native.sock");
    let mock_listener = UnixListener::bind(&mock_socket).unwrap();
    let native_listener = UnixListener::bind(&native_socket).unwrap();
    let revision = "a".repeat(64);
    let source = "eos/hardware/mock/arm_observer";
    let target = "eos/skill/package/reader";
    let payload = json!({"run_id":"run_1","revision":revision,"source_node":"eos/hardware/mock/arm",
      "sequence":1,"sampled_at_monotonic_ns":1,"joint_names":["a","b","c","d","e","f"],
      "joint_positions_rad":[0.0,0.0,0.0,0.0,0.0,0.0],"state":"ready","reason_code":null});
    let mock_payload = payload.clone();
    let mock = std::thread::spawn(move || {
      let (mut stream, _) = mock_listener.accept().unwrap();
      let mut bytes = Vec::new();
      stream.read_to_end(&mut bytes).unwrap();
      assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap()["op"],
        "observe_output"
      );
      stream.write_all(json!({"schema_version":1,"core":"eos","ok":true,"run_id":"run_1",
        "revision":mock_payload["revision"],"node":source,"port":"observation","payload":mock_payload}).to_string().as_bytes()).unwrap();
    });
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let native = std::thread::spawn(move || {
      let (mut stream, _) = native_listener.accept().unwrap();
      let mut bytes = Vec::new();
      stream.read_to_end(&mut bytes).unwrap();
      let request: Value = serde_json::from_slice(&bytes).unwrap();
      assert_eq!(request["node"], target);
      assert_eq!(request["payload"], payload);
      entered_tx.send(()).unwrap();
      std::thread::sleep(Duration::from_millis(100));
      stream.write_all(br#"{"ok":true,"outputs":[]}"#).unwrap();
    });
    let mut state = State {
      run_id: "run_1".into(),
      revision,
      mock_socket,
      worker_socket: native_socket,
      mock_observer: source.into(),
      cross_links: vec![CrossLink {
        source: source.into(),
        output: "observation".into(),
        target: target.into(),
        input: "observation".into(),
        data_type: "arm_monitor_state@v0.2".into(),
      }],
      ..State::default()
    };
    start_cross_sampler(&mut state, &NodeTypeRegistry::builtin());
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let started = Instant::now();
    sync_cross_sampler(&mut state);
    assert!(started.elapsed() < Duration::from_millis(50));
    stop_cross_sampler(&mut state);
    assert_eq!(state.last_mock_sequence, 1);
    assert!(state.cross_sampler.is_none());
    mock.join().unwrap();
    native.join().unwrap();
  }

  #[test]
  fn mixed_start_rollback_retains_children_when_stop_proof_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = State {
      child: Some(Command::new("sleep").arg("30").spawn().unwrap()),
      worker_socket: dir.path().join("native-missing.sock"),
      run_id: "run_1".into(),
      revision: "a".repeat(64),
      native: true,
      ..State::default()
    };
    let mut mock = State {
      child: Some(Command::new("sleep").arg("30").spawn().unwrap()),
      worker_socket: dir.path().join("mock-missing.sock"),
      ..State::default()
    };
    let reply = rollback_mixed_start_failure(
      &mut state,
      &mut mock,
      json!({"schema_version":1,"core":"eos","ok":false,"error":"startup_failed"}),
    );
    assert_eq!(reply["state"], "stop_uncertain");
    assert_eq!(reply["run_id"], "run_1");
    assert_eq!(state.run_id, "run_1");
    assert!(state.mixed);
    assert!(state.mock_child.is_some());
    assert!(state.child.is_some());
    for child in [&mut state.mock_child, &mut state.child] {
      let child = child.as_mut().unwrap();
      child.kill().unwrap();
      child.wait().unwrap();
    }
  }

  #[test]
  fn native_start_rollback_retains_run_when_worker_stop_is_unproven() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = State {
      child: Some(Command::new("sleep").arg("30").spawn().unwrap()),
      worker_socket: dir.path().join("native-missing.sock"),
      run_id: "run_1".into(),
      revision: "a".repeat(64),
      native: true,
      ..State::default()
    };
    assert!(!rollback_native_start(&mut state));
    assert_eq!(state.run_id, "run_1");
    assert_eq!(state.revision, "a".repeat(64));
    assert!(state.native);
    assert!(state.child.is_some());
    let child = state.child.as_mut().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
  }

  #[test]
  fn stale_mock_observation_cannot_reach_native_worker() {
    let dir = tempfile::tempdir().unwrap();
    let mock_socket = dir.path().join("mock.sock");
    let listener = UnixListener::bind(&mock_socket).unwrap();
    let worker = std::thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut bytes = Vec::new();
      stream.read_to_end(&mut bytes).unwrap();
      stream.write_all(br#"{"schema_version":1,"core":"eos","ok":true,"run_id":"old","revision":"old","node":"eos/hardware/mock/arm_observer","port":"observation","payload":{}}"#).unwrap();
    });
    let mut state = State {
      run_id: "run_1".into(),
      revision: "a".repeat(64),
      mock_socket,
      worker_socket: dir.path().join("missing-native.sock"),
      mock_observer: "eos/hardware/mock/arm_observer".into(),
      cross_links: vec![CrossLink {
        source: "eos/hardware/mock/arm_observer".into(),
        output: "observation".into(),
        target: "eos/skill/package/reader".into(),
        input: "observation".into(),
        data_type: "arm_monitor_state@v0.2".into(),
      }],
      ..State::default()
    };
    assert_eq!(
      route_mock_observation(&mut state, &NodeTypeRegistry::builtin()).unwrap_err(),
      "mock_observation_invalid"
    );
    worker.join().unwrap();
  }

  #[test]
  fn external_command_mapping_rejects_unknown_axes_modes_and_ports() {
    let base = json!({"axes":{"x":0.2,"y":0.0,"yaw":-0.4},"lease_ms":50});
    assert_eq!(
      translate_external_command("eos.hardware.base.normalized_axes@v0.2", "command", &base)
        .unwrap()["axes"]["yaw"],
      -0.4
    );
    assert!(translate_external_command(
      "eos.hardware.base.normalized_axes@v0.2",
      "command",
      &json!({"axes":{"roll":0.1},"lease_ms":50})
    )
    .is_err());
    assert!(translate_external_command(
      "eos.hardware.manipulator.normalized_cartesian@v0.2",
      "mode",
      &json!({"mode":"undocumented"})
    )
    .is_err());
    assert!(translate_external_command(
      "eos.hardware.gripper.binary@v0.2",
      "mode",
      &json!({"action":"close"})
    )
    .is_err());
  }

  #[test]
  fn package_handoff_matches_exact_pin_and_rejects_authored_endpoint() {
    let source = fs::read_to_string(concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/../../tests/fixtures/graph/demo-only.nodes.json"
    ))
    .unwrap();
    let registry = NodeTypeRegistry::builtin();
    let mut projection: Value = serde_json::from_str(&source).unwrap();
    let eos = projection["cores"]["eos"].clone();
    projection["cores"] = json!({"eos":eos});
    let profile = Profile::from_json_with_registry(&projection.to_string(), &registry).unwrap();
    let mut node = profile.all_nodes().next().unwrap().clone();
    node.adapter.package =
      Some(serde_json::from_value(json!({"id":"vendor/driver","version":"0.2.1"})).unwrap());
    node.adapter.settings.insert(
      "service_instance".into(),
      serde_json::from_value(json!("native")).unwrap(),
    );
    let endpoint = json!({"package":{"id":"vendor/driver","version":"0.2.1"},"service_instance":"native","socket_path":"/tmp/isos-native.sock","service_generation":"generation-1","config_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"});
    let mut request = json!({"endpoints":{"package_services":{node.path():endpoint}}});
    assert!(service_handoff(&request, &node).unwrap().is_some());
    request["endpoints"]["package_services"][node.path()]["package"]["version"] = json!("0.2.0");
    assert_eq!(
      service_handoff(&request, &node).unwrap_err(),
      "service_handoff_mismatch"
    );
    node.adapter.settings.insert(
      "socket_path".into(),
      serde_json::from_value(json!("/tmp/author.sock")).unwrap(),
    );
    assert_eq!(
      service_handoff(&request, &node).unwrap_err(),
      "service_handoff_mismatch"
    );
  }

  fn exchange(request: &[u8]) -> String {
    let registry = NodeTypeRegistry::with_contribution_files(&[]).unwrap();
    let (mut client, mut server) = UnixStream::pair().unwrap();
    let worker = std::thread::spawn(move || {
      respond(
        &mut server,
        &registry,
        &mut State::default(),
        Path::new("/tmp/eos-owner-test.sock"),
      )
      .unwrap()
    });
    client.write_all(request).unwrap();
    client.shutdown(std::net::Shutdown::Write).unwrap();
    let mut reply = String::new();
    client.read_to_string(&mut reply).unwrap();
    worker.join().unwrap();
    reply
  }

  #[test]
  fn health_is_idle_and_invalid_lifecycle_fails_closed() {
    let health = exchange(b"GET /v1/health HTTP/1.1\r\nHost: owner\r\n\r\n");
    assert!(health.starts_with("HTTP/1.1 200"));
    assert!(health.contains("\"state\":\"idle\""));
    assert!(health.contains("\"run_id\":null"));
    assert!(health.contains("\"revision\":null"));
    let invalid_logs = exchange(b"GET /v1/logs?after=bad HTTP/1.1\r\nHost: owner\r\n\r\n");
    assert!(invalid_logs.starts_with("HTTP/1.1 400"));
    assert!(invalid_logs.contains("invalid_log_cursor"));
    let run = exchange(b"POST /v1/run HTTP/1.1\r\nHost: owner\r\nContent-Length: 0\r\n\r\n");
    assert!(run.starts_with("HTTP/1.1 400"));
    assert!(run.contains("invalid_json"));
  }

  #[test]
  fn worker_health_preserves_degraded_and_stale_states() {
    for expected in ["ready", "degraded", "stale", "stopped"] {
      let dir = tempfile::tempdir().unwrap();
      let socket = dir.path().join("worker.sock");
      let listener = UnixListener::bind(&socket).unwrap();
      let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut command = [0u8; 7];
        stream.read_exact(&mut command).unwrap();
        assert_eq!(&command, b"status\n");
        stream
          .write_all(
            json!({"ok":true,"run_id":"run_1","state":expected})
              .to_string()
              .as_bytes(),
          )
          .unwrap();
      });
      assert_eq!(worker_state(&socket, "run_1"), Some(expected));
      worker.join().unwrap();
    }
  }

  #[test]
  fn owner_health_and_status_expose_live_degraded_worker() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("worker.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let worker = std::thread::spawn(move || {
      for _ in 0..2 {
        let (mut stream, _) = listener.accept().unwrap();
        let mut command = [0u8; 7];
        stream.read_exact(&mut command).unwrap();
        assert_eq!(&command, b"status\n");
        stream
          .write_all(br#"{"ok":true,"run_id":"run_1","state":"degraded"}"#)
          .unwrap();
      }
    });
    let child = Command::new("sleep").arg("5").spawn().unwrap();
    let mut state = State {
      run_id: "run_1".into(),
      revision: "a".repeat(64),
      worker_socket: socket,
      child: Some(child),
      skipped_mock_samples: 3,
      ..State::default()
    };
    let (mut client, mut server) = UnixStream::pair().unwrap();
    client
      .write_all(b"GET /v1/health HTTP/1.1\r\nHost: owner\r\n\r\n")
      .unwrap();
    respond(
      &mut server,
      &NodeTypeRegistry::builtin(),
      &mut state,
      Path::new("/tmp/eos-owner-test.sock"),
    )
    .unwrap();
    drop(server);
    let mut health = String::new();
    client.read_to_string(&mut health).unwrap();
    assert!(health.contains("\"state\":\"degraded\""), "{health}");
    assert!(health.contains("\"run_id\":\"run_1\""), "{health}");
    assert!(
      health.contains(&format!("\"revision\":\"{}\"", "a".repeat(64))),
      "{health}"
    );
    assert!(
      health.contains("\"observation_samples_skipped\":3"),
      "{health}"
    );
    let status = lifecycle(
      "status",
      &json!({"run_id":"run_1","revision":"a".repeat(64)}),
      &NodeTypeRegistry::builtin(),
      &mut state,
      Path::new("/tmp/eos-owner-test.sock"),
    );
    assert_eq!(status["state"], "degraded");
    assert_eq!(status["mock_hardware"], "degraded");
    assert_eq!(status["observation_samples_skipped"], 3);
    state.child.as_mut().unwrap().kill().unwrap();
    state.child.as_mut().unwrap().wait().unwrap();
    worker.join().unwrap();
  }

  #[test]
  fn actuator_bearing_run_request_is_rejected() {
    let body = serde_json::to_vec(&json!({
      "schema_version":1,"core":"eos","op":"run","run_id":"run_1",
      "revision":"a".repeat(64),
      "graph":{"version":"0.2.0","cores":{"isos":{"layers":{"eos_hardware":{"groups":{"joint":{"nodes":{"arm":{"movement":{}}}}}}}}}},
      "packages":[],"launch":{"args":["--run-id","run_1"]}
    })).unwrap();
    let header = format!(
      "POST /v1/run HTTP/1.1\r\nHost: owner\r\nContent-Length: {}\r\n\r\n",
      body.len()
    );
    let mut request = header.into_bytes();
    request.extend(body);
    let reply = exchange(&request);
    assert!(reply.starts_with("HTTP/1.1 422"));
    assert!(reply.contains("invalid_eos_graph"));
  }

  #[test]
  fn setup_route_validates_envelope_and_reuses_catalog() {
    let body = br#"{"schema_version":1,"core":"eos","op":"catalog"}"#;
    let request = format!(
      "POST /v1/catalog HTTP/1.1\r\nHost: owner\r\nContent-Length: {}\r\n\r\n",
      body.len()
    );
    let mut bytes = request.into_bytes();
    bytes.extend_from_slice(body);
    let reply = exchange(&bytes);
    assert!(reply.starts_with("HTTP/1.1 200"));
    assert!(reply.contains("\"schema_version\":3"));
    assert!(reply.contains("\"catalog\""));
    let identity_body = br#"{"schema_version":1,"core":"eos","op":"identity"}"#;
    let identity_request = format!(
      "POST /v1/identity HTTP/1.1\r\nHost: owner\r\nContent-Length: {}\r\n\r\n",
      identity_body.len()
    );
    let mut identity_bytes = identity_request.into_bytes();
    identity_bytes.extend_from_slice(identity_body);
    let identity_reply = exchange(&identity_bytes);
    assert!(identity_reply.starts_with("HTTP/1.1 200"));
    assert!(identity_reply.contains("\"identity\""));
    bytes.splice(
      bytes.len() - body.len()..,
      br#"{"schema_version":1,"core":"bad","op":"catalog"}"#
        .iter()
        .copied(),
    );
    let bad = exchange(&bytes);
    assert!(bad.starts_with("HTTP/1.1 400"));
  }

  #[test]
  fn uncertain_activation_uses_service_unavailable() {
    assert_eq!(
      lifecycle_http_status("activate", &json!({"ok":false,"error":"outcome_unknown"})),
      503
    );
    assert_eq!(
      lifecycle_http_status(
        "activate",
        &json!({"ok":false,"error":"worker_unavailable"})
      ),
      503
    );
    assert_eq!(
      lifecycle_http_status(
        "activate",
        &json!({"ok":false,"error":"invalid_activation"})
      ),
      422
    );
    assert_eq!(
      lifecycle_http_status("monitor", &json!({"ok":false,"error":"worker_unavailable"})),
      503
    );
  }

  #[test]
  fn dead_worker_result_uses_journal_uncertainty() {
    let dir = tempfile::tempdir().unwrap();
    let revision = "a".repeat(64);
    let run_id = "run_1";
    let journal = dir.path().join("eos-commands-run_1.jsonl");
    let accepted = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":"click-1","command_id":"arm-click-1","status":"accepted"});
    fs::write(
      &journal,
      format!(
        "{}\n",
        json!({"event_id":"click-1","source_node":"isos/ui/button/trigger","result":accepted})
      ),
    )
    .unwrap();
    let state = State {
      run_id: run_id.into(),
      revision: revision.clone(),
      worker_socket: dir.path().join("eos-run.sock"),
      ..State::default()
    };
    assert_eq!(
      journal_result(&state, "click-1")["status"],
      "outcome_unknown"
    );
    assert_eq!(
      journal_result(&state, "missing")["error"],
      "event_not_found"
    );
  }

  #[test]
  fn stale_status_and_stop_cannot_claim_a_run() {
    let registry = NodeTypeRegistry::builtin();
    let request =
      json!({"schema_version":1,"core":"eos","run_id":"run_1","revision":"a".repeat(64)});
    let mut state = State::default();
    for op in ["status", "stop"] {
      let reply = lifecycle(
        op,
        &request,
        &registry,
        &mut state,
        Path::new("/tmp/eos-owner-test.sock"),
      );
      assert_eq!(reply["ok"], false);
      assert_eq!(reply["error"], "stale_or_missing_run");
    }
  }

  #[test]
  fn forwards_native_output_to_ei_with_origin_and_run_fence() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("ei.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let peer = std::thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut reader = BufReader::new(stream.try_clone().unwrap());
      let mut length = 0;
      loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" {
          break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
          length = value.trim().parse::<usize>().unwrap();
        }
      }
      let mut body = vec![0; length];
      reader.read_exact(&mut body).unwrap();
      let value: Value = serde_json::from_slice(&body).unwrap();
      assert_eq!(value["source_node"], "eos/skill/custom/source");
      assert_eq!(value["source_port"], "out");
      assert_eq!(value["node"], "ei/io/input/target");
      assert_eq!(value["port"], "in");
      assert_eq!(value["payload"]["value"], 1);
      let reply = json!({"schema_version":1,"core":"ei","ok":true,"run_id":"run_1","revision":"a".repeat(64),"outputs":[]}).to_string();
      write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        reply.len(),
        reply
      )
      .unwrap();
    });
    let link = EiLink {
      source: "eos/skill/custom/source".into(),
      output: "out".into(),
      target: "ei/io/input/target".into(),
      input: "in".into(),
    };
    assert!(forward_ei_input(
      &socket,
      "run_1",
      &"a".repeat(64),
      &link,
      &json!({"value":1})
    )
    .is_ok());
    peer.join().unwrap();
  }

  #[test]
  fn vision_subscription_ticket_requires_exact_run_and_route() {
    let directory = tempfile::tempdir().unwrap();
    let mut state = State {
      child: Some(Command::new("sleep").arg("5").spawn().unwrap()),
      run_id: "run_1".into(),
      revision: "a".repeat(64),
      native: true,
      worker_socket: directory.path().join("eos-run.sock"),
      native_observers: vec!["eos/hardware/camera/front".into()],
      vision_transfers: BTreeMap::from([(
        "eos/interface/egress/front".into(),
        "eos/hardware/camera/front".into(),
      )]),
      vision_token: "b".repeat(64),
      ..State::default()
    };
    let request = json!({"schema_version":1,"core":"eos","op":"vision_subscribe",
      "run_id":"run_1","revision":"a".repeat(64),
      "transfer_path":"eos/interface/egress/front","source_path":"eos/hardware/camera/front"});
    let ticket = lifecycle(
      "vision_subscribe",
      &request,
      &NodeTypeRegistry::builtin(),
      &mut state,
      directory.path(),
    );
    assert_eq!(ticket["ok"], true, "{ticket}");
    assert_eq!(ticket["vision_token"], "b".repeat(64));
    let mut wrong = request.clone();
    wrong["source_path"] = json!("eos/hardware/camera/other");
    assert_eq!(
      lifecycle(
        "vision_subscribe",
        &wrong,
        &NodeTypeRegistry::builtin(),
        &mut state,
        directory.path()
      )["ok"],
      false
    );
    wrong = request.clone();
    wrong["run_id"] = json!("run_2");
    assert_eq!(
      lifecycle(
        "vision_subscribe",
        &wrong,
        &NodeTypeRegistry::builtin(),
        &mut state,
        directory.path()
      )["ok"],
      false
    );
    state.child.as_mut().unwrap().kill().unwrap();
    state.child.as_mut().unwrap().wait().unwrap();
  }

  #[test]
  fn latest_vision_output_reads_source_pool_and_labels_transfer() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("native.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let revision = "a".repeat(64);
    let peer_revision = revision.clone();
    let peer = std::thread::spawn(move || {
      let (mut sample, _) = listener.accept().unwrap();
      let mut body = Vec::new();
      sample.read_to_end(&mut body).unwrap();
      let request: Value = serde_json::from_slice(&body).unwrap();
      assert_eq!(request["op"], "latest_output");
      assert_eq!(request["node"], "eos/hardware/camera/front");
      assert_eq!(request["port"], "frame");
      assert_eq!(request["after_sequence"], 7);
      let reply = json!({"ok":true,"run_id":"run_1","revision":peer_revision,
        "sample":{"node":"eos/hardware/camera/front","sequence":8,
          "output":{"port":"frame","payload":{"sample":{"source_time_ns":1,"clock":"simulation",
            "sequence":73,"frame_id":"camera"},"width_px":1,"height_px":1,"encoding":"rgb8","data":"AAAA"}}}});
      sample.write_all(reply.to_string().as_bytes()).unwrap();
    });
    let child = Command::new("sleep").arg("5").spawn().unwrap();
    let mock_child = Command::new("sleep").arg("5").spawn().unwrap();
    let mut state = State {
      child: Some(child),
      mock_child: Some(mock_child),
      mixed: true,
      run_id: "run_1".into(),
      revision: revision.clone(),
      worker_socket: socket,
      native: true,
      native_observers: vec!["eos/hardware/camera/front".into()],
      vision_transfers: BTreeMap::from([(
        "eos/interface/egress/front".into(),
        "eos/hardware/camera/front".into(),
      )]),
      ..State::default()
    };
    let reply = lifecycle(
      "latest_output",
      &json!({"schema_version":1,"core":"eos","op":"latest_output",
      "run_id":"run_1","revision":revision,"transfer_path":"eos/interface/egress/front",
      "source_path":"eos/hardware/camera/front","node":"eos/interface/egress/front","port":"frame",
      "after_sequence":7,"wait_ms":20}),
      &NodeTypeRegistry::builtin(),
      &mut state,
      directory.path(),
    );
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["sample"]["node"], "eos/interface/egress/front");
    assert_eq!(reply["sample"]["sequence"], 8);
    state.child.as_mut().unwrap().kill().unwrap();
    state.child.as_mut().unwrap().wait().unwrap();
    state.mock_child.as_mut().unwrap().kill().unwrap();
    state.mock_child.as_mut().unwrap().wait().unwrap();
    peer.join().unwrap();
  }
}
