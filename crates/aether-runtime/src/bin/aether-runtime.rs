//! EOS event worker: ingress, built-in skill, dispatcher admission, and mock arm.
use std::{
  collections::BTreeMap,
  env,
  error::Error,
  fs::{self, File, OpenOptions},
  io::{BufRead, BufReader, Read, Write},
  os::unix::{
    fs::PermissionsExt,
    net::{UnixListener, UnixStream},
  },
  path::{Path, PathBuf},
  sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
  },
  time::{Duration, Instant},
};
use serde_json::{json, Value};
#[path = "common/eos_log.rs"]
mod eos_log;
static STOP: AtomicBool = AtomicBool::new(false);
const DEMO_DELTA: [f64; 6] = [0.05, -0.04, 0.03, -0.02, 0.01, -0.01];
const MAX_ABS_JOINT_RAD: f64 = 314.0 / 100.0;
const MAX_EVENTS_PER_RUN: usize = 4096;
struct SamplingConfig {
  interval_ms: u64,
  freshness_timeout_ms: u64,
}
struct SocketGuard(PathBuf);
impl Drop for SocketGuard {
  fn drop(&mut self) {
    let _ = fs::remove_file(&self.0);
  }
}
extern "C" fn on_signal(_: libc::c_int) {
  STOP.store(true, Ordering::SeqCst);
}
fn valid_id(value: &str, max: usize) -> bool {
  !value.is_empty()
    && value.len() <= max
    && value
      .bytes()
      .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
fn valid_button_source(path: &str) -> bool {
  let parts: Vec<_> = path.split('/').collect();
  matches!(parts.as_slice(), ["isos", "ui", "button", id] if !id.is_empty() && id.len() <= 48 && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')))
}
fn valid_revision(value: &str) -> bool {
  value.len() == 64
    && value
      .bytes()
      .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
struct Arm {
  joints: [f64; 6],
  joint_limit_rad: f64,
  joint_names: [String; 6],
  source_node: String,
  sequence: u64,
  sampled_at_monotonic_ns: u64,
  sampled_at: Instant,
  freshness_timeout: Duration,
  sample_state: &'static str,
  sample_reason: Option<&'static str>,
  previous: BTreeMap<String, (String, Value)>,
  journal: Option<File>,
  recovery_uncertain: bool,
}
/// Read-only hardware-layer endpoint bound to the graph's observation source.
/// It shares the sole mock arm adapter's sampler and cannot submit movement.
struct ArmObserver {
  node_path: String,
  payload_source_node: String,
  arm: Arc<Mutex<Arm>>,
}
impl ArmObserver {
  fn observe(&self, run_id: &str, revision: &str) -> Result<Value, &'static str> {
    let arm = self.arm.lock().map_err(|_| "arm state poisoned")?;
    if self.payload_source_node != self.node_path && self.payload_source_node != arm.source_node {
      return Err("observer source mismatch");
    }
    let mut observation = arm.observation(run_id, revision);
    observation["source_node"] = json!(self.payload_source_node);
    Ok(observation)
  }
}
impl Default for Arm {
  fn default() -> Self {
    let mut arm = Self {
      joints: [0.0; 6],
      joint_limit_rad: MAX_ABS_JOINT_RAD,
      joint_names: std::array::from_fn(|index| format!("joint_{}", index + 1)),
      source_node: "eos/hardware/mock/arm".into(),
      sequence: 0,
      sampled_at_monotonic_ns: 0,
      sampled_at: Instant::now(),
      freshness_timeout: Duration::from_millis(50),
      sample_state: "stale",
      sample_reason: Some("not_sampled"),
      previous: BTreeMap::new(),
      journal: None,
      recovery_uncertain: false,
    };
    arm.sample();
    arm
  }
}
fn monotonic_ns() -> Option<u64> {
  let mut time = libc::timespec {
    tv_sec: 0,
    tv_nsec: 0,
  };
  if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) } != 0 {
    return None;
  }
  u64::try_from(time.tv_sec)
    .ok()?
    .checked_mul(1_000_000_000)?
    .checked_add(u64::try_from(time.tv_nsec).ok()?)
}
impl Arm {
  fn sample(&mut self) {
    self.sequence = self.sequence.saturating_add(1);
    self.sampled_at = Instant::now();
    self.sampled_at_monotonic_ns = monotonic_ns().unwrap_or(0);
    let healthy = self.sampled_at_monotonic_ns > 0
      && self
        .joints
        .iter()
        .all(|value| value.is_finite() && value.abs() <= self.joint_limit_rad);
    self.sample_state = if healthy { "ready" } else { "degraded" };
    self.sample_reason = if healthy {
      None
    } else {
      Some("invalid_joint_sample")
    };
  }
  fn monitor_state(&self) -> (&'static str, Option<&'static str>) {
    if STOP.load(Ordering::SeqCst) {
      ("stopped", Some("stopped"))
    } else if self.sampled_at.elapsed() > self.freshness_timeout {
      ("stale", Some("sample_timeout"))
    } else {
      (self.sample_state, self.sample_reason)
    }
  }
  fn observation(&self, run_id: &str, revision: &str) -> Value {
    let (state, reason_code) = self.monitor_state();
    json!({"run_id":run_id,"revision":revision,"source_node":self.source_node,
      "sequence":self.sequence,"sampled_at_monotonic_ns":self.sampled_at_monotonic_ns,
      "joint_names":self.joint_names,"joint_positions_rad":if state == "ready" { json!(self.joints) } else { Value::Null },
      "state":state,"reason_code":reason_code})
  }
  fn record(&mut self, event_id: &str, source_node: &str, result: &Value) -> std::io::Result<()> {
    if let Some(journal) = self.journal.as_mut() {
      serde_json::to_writer(
        &mut *journal,
        &json!({"event_id":event_id,"source_node":source_node,"result":result}),
      )?;
      journal.write_all(b"\n")?;
      journal.sync_all()?;
    }
    Ok(())
  }
  fn load_journal(&mut self, path: &Path, run_id: &str, revision: &str) -> std::io::Result<()> {
    if !path.exists() {
      return Ok(());
    }
    let reader = BufReader::new(File::open(path)?);
    for line in reader.lines() {
      let line = line?;
      let entry: Value = serde_json::from_str(&line)?;
      let event_id = entry["event_id"]
        .as_str()
        .ok_or_else(|| std::io::Error::other("journal event missing"))?;
      let source = entry["source_node"]
        .as_str()
        .ok_or_else(|| std::io::Error::other("journal source missing"))?;
      let result = &entry["result"];
      if !valid_id(event_id, 64)
        || !valid_button_source(source)
        || result["run_id"] != run_id
        || result["revision"] != revision
        || result["event_id"] != event_id
        || result["command_id"] != format!("arm-{event_id}")
        || result["ok"] != true
      {
        return Err(std::io::Error::other("journal identity mismatch"));
      }
      if result["status"] == "succeeded" {
        let values = result["joint_positions_rad"]
          .as_array()
          .ok_or_else(|| std::io::Error::other("journal positions missing"))?;
        if values.len() != 6 {
          return Err(std::io::Error::other("journal positions invalid"));
        }
        let mut positions = [0.0; 6];
        for (index, value) in values.iter().enumerate() {
          positions[index] = value
            .as_f64()
            .filter(|value| value.is_finite() && value.abs() <= self.joint_limit_rad)
            .ok_or_else(|| std::io::Error::other("journal positions invalid"))?;
        }
        self.joints = positions;
      }
      self
        .previous
        .insert(event_id.into(), (source.into(), result.clone()));
    }
    for (_, result) in self.previous.values_mut() {
      if matches!(result["status"].as_str(), Some("accepted" | "running")) {
        let event_id = result["event_id"].as_str().unwrap_or("");
        *result = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"outcome_unknown"});
        self.recovery_uncertain = true;
      }
    }
    self.sample();
    Ok(())
  }
  fn reserve(&mut self, request: &Value, run_id: &str, revision: &str) -> Value {
    let event_id = request["event_id"].as_str().unwrap_or("");
    let source_node = request["source_node"].as_str().unwrap_or("");
    if request["schema_version"] != 1
      || request["op"] != "activate"
      || request["core"] != "eos"
      || request["run_id"] != run_id
      || request["revision"] != revision
      || request["kind"] != "activate"
      || !valid_id(event_id, 64)
      || !valid_button_source(source_node)
      || request.as_object().is_none_or(|fields| fields.len() != 8)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_activation"});
    }
    if let Some((prior_source, _)) = self.previous.get(event_id) {
      return if prior_source == source_node {
        json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"accepted"})
      } else {
        json!({"schema_version":1,"core":"eos","ok":false,"error":"event_conflict"})
      };
    }
    if STOP.load(Ordering::SeqCst) {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"stopping"});
    }
    if self.recovery_uncertain {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"outcome_unknown"});
    }
    let (state, _) = self.monitor_state();
    if state != "ready" {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":if state == "stale" {"monitor_stale"} else {"monitor_unavailable"}});
    }
    if self.previous.len() >= MAX_EVENTS_PER_RUN {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"event_capacity"});
    }
    let accepted = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"accepted"});
    if self.record(event_id, source_node, &accepted).is_err() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"outcome_unknown"});
    }
    self
      .previous
      .insert(event_id.into(), (source_node.into(), accepted.clone()));
    accepted
  }
  fn execute_reserved(&mut self, event_id: &str, run_id: &str, revision: &str) {
    let Some((source_node, prior)) = self.previous.get(event_id).cloned() else {
      return;
    };
    if prior["status"] != "accepted" {
      return;
    }
    if self.recovery_uncertain {
      let unknown = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"outcome_unknown"});
      let _ = self.record(event_id, &source_node, &unknown);
      self
        .previous
        .insert(event_id.into(), (source_node, unknown));
      return;
    }
    let status = if STOP.load(Ordering::SeqCst) {
      "cancelled"
    } else {
      "running"
    };
    let running = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":status});
    if self.record(event_id, &source_node, &running).is_err() {
      self.recovery_uncertain = true;
      let unknown = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"outcome_unknown"});
      self
        .previous
        .insert(event_id.into(), (source_node, unknown));
      return;
    }
    self
      .previous
      .insert(event_id.into(), (source_node.clone(), running));
    if status == "cancelled" {
      return;
    }
    let mut target = self.joints;
    for (joint, delta) in target.iter_mut().zip(DEMO_DELTA) {
      *joint += delta;
    }
    let result = if self.monitor_state().0 != "ready" {
      json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"failed","error":"monitor_unavailable"})
    } else if target
      .iter()
      .any(|joint| !joint.is_finite() || joint.abs() > self.joint_limit_rad)
    {
      json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"failed","error":"joint_limit"})
    } else {
      self.joints = target;
      self.sample();
      json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"succeeded","joint_positions_rad":self.joints})
    };
    if self.record(event_id, &source_node, &result).is_err() {
      self.recovery_uncertain = true;
      let uncertain = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"outcome_unknown"});
      self
        .previous
        .insert(event_id.into(), (source_node, uncertain));
    } else {
      self.previous.insert(event_id.into(), (source_node, result));
    }
  }
  fn result(&self, request: &Value, run_id: &str, revision: &str) -> Value {
    let event_id = request["event_id"].as_str().unwrap_or("");
    if request["schema_version"] != 1
      || request["core"] != "eos"
      || request["op"] != "result"
      || request["run_id"] != run_id
      || request["revision"] != revision
      || !valid_id(event_id, 64)
      || request.as_object().is_none_or(|fields| fields.len() != 6)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_result_request"});
    }
    self
      .previous
      .get(event_id)
      .map(|(_, result)| result.clone())
      .unwrap_or_else(
        || json!({"schema_version":1,"core":"eos","ok":false,"error":"event_not_found"}),
      )
  }
  fn activate(&mut self, request: &Value, run_id: &str, revision: &str) -> Value {
    let event_id = request["event_id"].as_str().unwrap_or("");
    let source_node = request["source_node"].as_str().unwrap_or("");
    if request["schema_version"] != 1
      || request["op"] != "activate"
      || request["core"] != "eos"
      || request["run_id"] != run_id
      || request["revision"] != revision
      || request["kind"] != "activate"
      || !valid_id(event_id, 64)
      || !valid_button_source(source_node)
      || request.as_object().is_none_or(|fields| fields.len() != 8)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_activation"});
    }
    if STOP.load(Ordering::SeqCst) {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"stopping"});
    }
    let (state, _) = self.monitor_state();
    if state != "ready" {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":if state == "stale" { "monitor_stale" } else { "monitor_unavailable" }});
    }
    if let Some((previous_source, result)) = self.previous.get(event_id) {
      return if previous_source == source_node {
        result.clone()
      } else {
        json!({"schema_version":1,"core":"eos","ok":false,"error":"event_conflict"})
      };
    }
    if self.previous.len() >= MAX_EVENTS_PER_RUN {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"event_capacity"});
    }
    // The built-in skill emits a typed six-joint delta. Dispatcher is the only movement gate.
    if DEMO_DELTA
      .iter()
      .any(|value| !value.is_finite() || value.abs() > 0.25)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"delta_invalid"});
    }
    let mut target = self.joints;
    for (joint, delta) in target.iter_mut().zip(DEMO_DELTA) {
      *joint += delta;
    }
    if target
      .iter()
      .any(|joint| !joint.is_finite() || joint.abs() > self.joint_limit_rad)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"joint_limit"});
    }
    self.joints = target;
    self.sample();
    let result = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"succeeded","joint_positions_rad":self.joints});
    self
      .previous
      .insert(event_id.into(), (source_node.into(), result.clone()));
    result
  }
}
fn respond(
  stream: &mut UnixStream,
  run_id: &str,
  revision: &str,
  arm: &Arc<Mutex<Arm>>,
  observer: &ArmObserver,
  commands: Option<&mpsc::SyncSender<String>>,
) -> Result<(), Box<dyn Error>> {
  stream.set_read_timeout(Some(Duration::from_secs(2)))?;
  stream.set_write_timeout(Some(Duration::from_secs(2)))?;
  let mut request = Vec::new();
  let mut chunk = [0u8; 1024];
  loop {
    let count = stream.read(&mut chunk)?;
    if count == 0 {
      break;
    }
    request.extend_from_slice(&chunk[..count]);
    if request.len() > 4096 || (request.first() != Some(&b'{') && request.contains(&b'\n')) {
      break;
    }
  }
  let reply = if request.len() > 4096 {
    json!({"ok":false,"error":"request_too_large"})
  } else if request == b"status\n" {
    let arm = arm.lock().map_err(|_| "arm state poisoned")?;
    let (state, _) = arm.monitor_state();
    json!({"ok":true,"core":"eos","run_id":run_id,"state":state,"mock_hardware":state,"joint_positions_rad":arm.joints})
  } else if request == b"stop\n" {
    STOP.store(true, Ordering::SeqCst);
    json!({"ok":true,"core":"eos","run_id":run_id,"state":"stopping"})
  } else {
    match serde_json::from_slice::<Value>(&request) {
      Ok(value) if value["op"] == "monitor" => {
        if value["schema_version"] != 1
          || value["core"] != "eos"
          || value["run_id"] != run_id
          || value["revision"] != revision
          || value.as_object().is_none_or(|fields| fields.len() != 5)
        {
          json!({"schema_version":1,"core":"eos","ok":false,"error":"monitor_fence_stale"})
        } else {
          json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"observation":observer.observe(run_id, revision)?})
        }
      }
      Ok(value) if value["op"] == "result" => arm
        .lock()
        .map_err(|_| "arm state poisoned")?
        .result(&value, run_id, revision),
      Ok(value) if value["op"] == "activate" && commands.is_some() => {
        let mut arm = arm.lock().map_err(|_| "arm state poisoned")?;
        let was_known = arm
          .previous
          .contains_key(value["event_id"].as_str().unwrap_or(""));
        let reply = arm.reserve(&value, run_id, revision);
        if !was_known
          && reply["ok"] == true
          && arm
            .previous
            .get(value["event_id"].as_str().unwrap_or(""))
            .is_some_and(|(_, result)| result["status"] == "accepted")
        {
          // Each reserved ID enters the bounded executor queue once.
          if commands
            .unwrap()
            .send(value["event_id"].as_str().unwrap().into())
            .is_err()
          {
            return Err("command executor unavailable".into());
          }
        }
        reply
      }
      Ok(value) => arm
        .lock()
        .map_err(|_| "arm state poisoned")?
        .activate(&value, run_id, revision),
      Err(_) => json!({"ok":false,"error":"unknown_command"}),
    }
  };
  stream.write_all(serde_json::to_string(&reply)?.as_bytes())?;
  Ok(())
}

fn serve(
  socket: &Path,
  run_id: &str,
  revision: &str,
  joint_limit_rad: f64,
  source_nodes: (&str, &str, &str),
  joint_names: [String; 6],
  sampling: SamplingConfig,
) -> Result<(), Box<dyn Error>> {
  if !socket.is_absolute()
    || !valid_id(run_id, 128)
    || !valid_revision(revision)
    || socket.exists()
    || !(1..=1000).contains(&sampling.interval_ms)
    || !(2..=5000).contains(&sampling.freshness_timeout_ms)
    || sampling.freshness_timeout_ms < sampling.interval_ms * 2
  {
    return Err("invalid worker socket or run identity".into());
  }
  STOP.store(false, Ordering::SeqCst);
  unsafe {
    libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
  }
  let listener = UnixListener::bind(socket)?;
  let _guard = SocketGuard(socket.to_path_buf());
  fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;
  eos_log::append_with_run(
    "info",
    "eos.aether_runtime.worker",
    "EOS arm worker started",
    Some(run_id),
  )?;
  eos_log::append_with_run(
    "info",
    "eos.hardware.mock.arm",
    "Mock arm ready",
    Some(run_id),
  )?;
  listener.set_nonblocking(true)?;
  let arm = Arc::new(Mutex::new(Arm {
    joint_limit_rad,
    source_node: source_nodes.0.into(),
    joint_names,
    freshness_timeout: Duration::from_millis(sampling.freshness_timeout_ms),
    ..Arm::default()
  }));
  let observer = ArmObserver {
    node_path: source_nodes.1.into(),
    payload_source_node: source_nodes.2.into(),
    arm: Arc::clone(&arm),
  };
  let journal_path = socket.with_file_name(format!("eos-commands-{run_id}.jsonl"));
  {
    let mut arm = arm.lock().map_err(|_| "arm state poisoned")?;
    arm.load_journal(&journal_path, run_id, revision)?;
    arm.journal = Some(
      OpenOptions::new()
        .append(true)
        .create(true)
        .open(&journal_path)?,
    );
  }
  let (command_tx, command_rx) = mpsc::sync_channel::<String>(MAX_EVENTS_PER_RUN);
  let executor_arm = Arc::clone(&arm);
  let executor_run = run_id.to_owned();
  let executor_revision = revision.to_owned();
  let executor = std::thread::spawn(move || {
    while let Ok(event_id) = command_rx.recv() {
      let Ok(mut arm) = executor_arm.lock() else {
        break;
      };
      if arm
        .previous
        .get(&event_id)
        .is_some_and(|(_, result)| result["status"] == "accepted")
      {
        arm.execute_reserved(&event_id, &executor_run, &executor_revision);
      }
    }
  });
  let sampler = Arc::clone(&arm);
  let sampling = std::thread::spawn(move || {
    while !STOP.load(Ordering::SeqCst) {
      if let Ok(mut arm) = sampler.lock() {
        arm.sample();
      } else {
        break;
      }
      std::thread::sleep(Duration::from_millis(sampling.interval_ms));
    }
  });
  while !STOP.load(Ordering::SeqCst) {
    match listener.accept() {
      Ok((mut stream, _)) => {
        if let Err(error) = respond(
          &mut stream,
          run_id,
          revision,
          &arm,
          &observer,
          Some(&command_tx),
        ) {
          let _ = eos_log::append_with_run(
            "error",
            "eos.aether_runtime.worker",
            &format!("Worker request failed: {error}"),
            Some(run_id),
          );
        }
      }
      Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
        std::thread::sleep(Duration::from_millis(20))
      }
      Err(error) => return Err(error.into()),
    }
  }
  drop(command_tx);
  let _ = executor.join();
  let _ = sampling.join();
  eos_log::append_with_run(
    "info",
    "eos.aether_runtime.worker",
    "EOS arm worker stopped",
    Some(run_id),
  )?;
  Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
  let args: Vec<_> = env::args().collect();
  if args.len() != 21
    || args[1] != "--serve"
    || args[3] != "--run-id"
    || args[5] != "--revision"
    || args[7] != "--joint-limit"
    || args[9] != "--source-node"
    || args[11] != "--observer-node"
    || args[13] != "--observation-source-node"
    || args[15] != "--joint-names"
    || args[17] != "--sample-interval-ms"
    || args[19] != "--freshness-timeout-ms"
  {
    return Err("usage: aether-runtime --serve <socket> --run-id <id> --revision <sha256> --joint-limit <rad> --source-node <path> --observer-node <path> --observation-source-node <path> --joint-names <csv> --sample-interval-ms <ms> --freshness-timeout-ms <ms>".into());
  }
  let names: Vec<String> = args[16].split(',').map(str::to_owned).collect();
  let joint_names: [String; 6] = names.try_into().map_err(|_| "six joint names required")?;
  if !args[10].starts_with("eos/hardware/mock/")
    || !args[12].starts_with("eos/hardware/mock/")
    || args[12] == args[10]
    || (args[14] != args[10] && args[14] != args[12])
    || !args[12]
      .split('/')
      .next_back()
      .is_some_and(|id| valid_id(id, 48))
    || joint_names.iter().any(|name| !valid_id(name, 48))
  {
    return Err("invalid mock arm identity".into());
  }
  serve(
    &PathBuf::from(&args[2]),
    &args[4],
    &args[6],
    args[8].parse()?,
    (&args[10], &args[12], &args[14]),
    joint_names,
    SamplingConfig {
      interval_ms: args[18].parse()?,
      freshness_timeout_ms: args[20].parse()?,
    },
  )
}
#[cfg(test)]
mod tests {
  use super::*;
  static TEST_LOCK: Mutex<()> = Mutex::new(());
  #[test]
  fn observation_is_fenced_and_fails_closed_until_recovery() {
    let _lock = TEST_LOCK.lock().unwrap();
    STOP.store(false, Ordering::SeqCst);
    let revision = "a".repeat(64);
    let mut arm = Arm::default();
    let initial = arm.observation("run_1", &revision);
    assert_eq!(initial["state"], "ready");
    assert_eq!(initial["reason_code"], Value::Null);
    assert_eq!(
      initial["joint_positions_rad"],
      json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    );
    assert_eq!(
      initial["joint_names"],
      json!(["joint_1", "joint_2", "joint_3", "joint_4", "joint_5", "joint_6"])
    );
    assert_eq!(initial["source_node"], "eos/hardware/mock/arm");
    assert_eq!(initial["run_id"], "run_1");
    assert_eq!(initial["revision"], revision);
    assert!(initial["sampled_at_monotonic_ns"].as_u64().unwrap() > 0);
    let request = json!({"schema_version":1,"core":"eos","op":"activate","run_id":"run_1","revision":revision,"source_node":"isos/ui/button/trigger","event_id":"click-1","kind":"activate"});

    arm.sampled_at = Instant::now() - Duration::from_millis(51);
    let stale = arm.observation("run_1", &revision);
    assert_eq!(stale["state"], "stale");
    assert_eq!(stale["reason_code"], "sample_timeout");
    assert!(stale["joint_positions_rad"].is_null());
    assert_eq!(
      arm.activate(&request, "run_1", &revision)["error"],
      "monitor_stale"
    );

    arm.sample();
    assert_eq!(arm.observation("run_1", &revision)["state"], "ready");
    arm.joints[0] = f64::NAN;
    arm.sample();
    let degraded = arm.observation("run_1", &revision);
    assert_eq!(degraded["state"], "degraded");
    assert_eq!(degraded["reason_code"], "invalid_joint_sample");
    assert!(degraded["joint_positions_rad"].is_null());
    assert_eq!(
      arm.activate(&request, "run_1", &revision)["error"],
      "monitor_unavailable"
    );

    arm.joints = [0.0; 6];
    arm.sample();
    let recovered = arm.observation("run_1", &revision);
    assert_eq!(recovered["state"], "ready");
    assert!(recovered["sequence"].as_u64().unwrap() > initial["sequence"].as_u64().unwrap());
    assert!(
      recovered["sampled_at_monotonic_ns"].as_u64().unwrap()
        >= initial["sampled_at_monotonic_ns"].as_u64().unwrap()
    );
    assert_eq!(arm.activate(&request, "run_1", &revision)["ok"], true);
    let moved = arm.observation("run_1", &revision);
    assert_eq!(moved["joint_positions_rad"], json!(DEMO_DELTA));
    assert!(moved["sequence"].as_u64().unwrap() > recovered["sequence"].as_u64().unwrap());

    STOP.store(true, Ordering::SeqCst);
    let stopped = arm.observation("run_1", &revision);
    assert_eq!(stopped["state"], "stopped");
    assert_eq!(stopped["reason_code"], "stopped");
    assert!(stopped["joint_positions_rad"].is_null());
    assert_eq!(
      arm.activate(&request, "run_1", &revision)["error"],
      "stopping"
    );
    STOP.store(false, Ordering::SeqCst);
  }
  #[test]
  fn activation_moves_once_and_rejects_stale_or_limit() {
    let _lock = TEST_LOCK.lock().unwrap();
    STOP.store(false, Ordering::SeqCst);
    let revision = "a".repeat(64);
    let mut arm = Arm::default();
    let request = json!({"schema_version":1,"core":"eos","op":"activate","run_id":"run_1","revision":revision,"source_node":"isos/ui/button/trigger","event_id":"click-1","kind":"activate"});
    let first = arm.activate(&request, "run_1", &revision);
    assert_eq!(first["joint_positions_rad"], json!(DEMO_DELTA));
    assert_eq!(arm.activate(&request, "run_1", &revision), first);
    let mut stale = request.clone();
    stale["revision"] = json!("b".repeat(64));
    assert_eq!(
      arm.activate(&stale, "run_1", &revision)["error"],
      "invalid_activation"
    );
    let mut bounded = Arm {
      joint_limit_rad: 0.06,
      ..Arm::default()
    };
    assert_eq!(bounded.activate(&request, "run_1", &revision)["ok"], true);
    let mut second = request.clone();
    second["event_id"] = json!("click-2");
    assert_eq!(
      bounded.activate(&second, "run_1", &revision)["error"],
      "joint_limit"
    );
    arm.joints = [3.13; 6];
    let mut next = request.clone();
    next["event_id"] = json!("click-2");
    assert_eq!(
      arm.activate(&next, "run_1", &revision)["error"],
      "joint_limit"
    );
  }

  #[test]
  fn reserved_event_is_durable_and_repeated_once() {
    let _lock = TEST_LOCK.lock().unwrap();
    STOP.store(false, Ordering::SeqCst);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("commands.jsonl");
    let mut arm = Arm {
      journal: Some(
        OpenOptions::new()
          .create_new(true)
          .write(true)
          .open(&path)
          .unwrap(),
      ),
      ..Arm::default()
    };
    let revision = "a".repeat(64);
    let request = json!({"schema_version":1,"core":"eos","op":"activate","run_id":"run_1","revision":revision,"source_node":"isos/ui/button/trigger","event_id":"click-1","kind":"activate"});
    let first = arm.reserve(&request, "run_1", &revision);
    assert_eq!(first["status"], "accepted");
    assert_eq!(arm.joints, [0.0; 6]);
    assert_eq!(arm.reserve(&request, "run_1", &revision), first);
    assert_eq!(fs::read_to_string(&path).unwrap().lines().count(), 1);
    arm.execute_reserved("click-1", "run_1", &revision);
    assert_eq!(arm.result(&json!({"schema_version":1,"core":"eos","op":"result","run_id":"run_1","revision":revision,"event_id":"click-1"}), "run_1", &revision)["status"], "succeeded");
    assert_eq!(arm.joints, DEMO_DELTA);
    assert_eq!(fs::read_to_string(&path).unwrap().lines().count(), 3);
    arm.execute_reserved("click-1", "run_1", &revision);
    assert_eq!(arm.joints, DEMO_DELTA);
    let mut conflict = request.clone();
    conflict["source_node"] = json!("isos/ui/button/other");
    assert_eq!(
      arm.reserve(&conflict, "run_1", &revision)["error"],
      "event_conflict"
    );
  }

  #[test]
  fn journal_replay_marks_interrupted_command_unknown() {
    let _lock = TEST_LOCK.lock().unwrap();
    STOP.store(false, Ordering::SeqCst);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("commands.jsonl");
    let revision = "a".repeat(64);
    let request = json!({"schema_version":1,"core":"eos","op":"activate","run_id":"run_1","revision":revision,"source_node":"isos/ui/button/trigger","event_id":"click-1","kind":"activate"});
    let mut first = Arm {
      journal: Some(
        OpenOptions::new()
          .create_new(true)
          .write(true)
          .open(&path)
          .unwrap(),
      ),
      ..Arm::default()
    };
    assert_eq!(
      first.reserve(&request, "run_1", &revision)["status"],
      "accepted"
    );
    drop(first);
    let mut restarted = Arm::default();
    restarted.load_journal(&path, "run_1", &revision).unwrap();
    let result_request = json!({"schema_version":1,"core":"eos","op":"result","run_id":"run_1","revision":revision,"event_id":"click-1"});
    assert_eq!(
      restarted.result(&result_request, "run_1", &revision)["status"],
      "outcome_unknown"
    );
    assert_eq!(
      restarted.reserve(&request, "run_1", &revision)["status"],
      "accepted"
    );
    let mut fresh = request.clone();
    fresh["event_id"] = json!("click-2");
    assert_eq!(
      restarted.reserve(&fresh, "run_1", &revision)["error"],
      "outcome_unknown"
    );
    restarted.execute_reserved("click-1", "run_1", &revision);
    assert_eq!(restarted.joints, [0.0; 6]);
  }

  #[test]
  fn running_journal_failure_fences_movement() {
    let _lock = TEST_LOCK.lock().unwrap();
    STOP.store(false, Ordering::SeqCst);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("commands.jsonl");
    let revision = "a".repeat(64);
    let request = json!({"schema_version":1,"core":"eos","op":"activate","run_id":"run_1","revision":revision,"source_node":"isos/ui/button/trigger","event_id":"click-1","kind":"activate"});
    let mut arm = Arm {
      journal: Some(
        OpenOptions::new()
          .create_new(true)
          .write(true)
          .open(&path)
          .unwrap(),
      ),
      ..Arm::default()
    };
    assert_eq!(
      arm.reserve(&request, "run_1", &revision)["status"],
      "accepted"
    );
    arm.journal = Some(File::open(&path).unwrap());
    arm.execute_reserved("click-1", "run_1", &revision);
    assert_eq!(arm.joints, [0.0; 6]);
    assert_eq!(arm.previous["click-1"].1["status"], "outcome_unknown");
    let mut fresh = request.clone();
    fresh["event_id"] = json!("click-2");
    assert_eq!(
      arm.reserve(&fresh, "run_1", &revision)["error"],
      "outcome_unknown"
    );
  }

  #[test]
  fn duplicate_receipts_enqueue_only_one_command() {
    let _lock = TEST_LOCK.lock().unwrap();
    STOP.store(false, Ordering::SeqCst);
    let revision = "a".repeat(64);
    let arm = Arc::new(Mutex::new(Arm::default()));
    let observer = ArmObserver {
      node_path: "eos/hardware/mock/arm_observer".into(),
      payload_source_node: "eos/hardware/mock/arm_observer".into(),
      arm: Arc::clone(&arm),
    };
    let (sender, receiver) = mpsc::sync_channel(2);
    let request = json!({"schema_version":1,"core":"eos","op":"activate","run_id":"run_1","revision":revision,"source_node":"isos/ui/button/trigger","event_id":"click-1","kind":"activate"});
    for _ in 0..20 {
      let (mut client, mut server) = UnixStream::pair().unwrap();
      client
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
      client.shutdown(std::net::Shutdown::Write).unwrap();
      respond(
        &mut server,
        "run_1",
        &revision,
        &arm,
        &observer,
        Some(&sender),
      )
      .unwrap();
      drop(server);
      let mut reply = Vec::new();
      client.read_to_end(&mut reply).unwrap();
      assert_eq!(
        serde_json::from_slice::<Value>(&reply).unwrap()["status"],
        "accepted"
      );
    }
    assert_eq!(receiver.try_recv().unwrap(), "click-1");
    assert!(receiver.try_recv().is_err());
  }

  #[test]
  fn activation_waits_for_complete_socket_request() {
    let _lock = TEST_LOCK.lock().unwrap();
    STOP.store(false, Ordering::SeqCst);
    let revision = "a".repeat(64);
    let request = json!({"schema_version":1,"core":"eos","op":"activate","run_id":"run_1","revision":revision,"source_node":"isos/ui/button/trigger","event_id":"click-split","kind":"activate"});
    let bytes = serde_json::to_vec(&request).unwrap();
    let (mut client, mut server) = UnixStream::pair().unwrap();
    let worker_revision = revision.clone();
    let thread = std::thread::spawn(move || {
      let arm = Arc::new(Mutex::new(Arm::default()));
      let observer = ArmObserver {
        node_path: "eos/hardware/mock/arm_observer".into(),
        payload_source_node: "eos/hardware/mock/arm_observer".into(),
        arm: Arc::clone(&arm),
      };
      respond(
        &mut server,
        "run_1",
        &worker_revision,
        &arm,
        &observer,
        None,
      )
      .unwrap();
      let joints = arm.lock().unwrap().joints;
      joints
    });
    let middle = bytes.len() / 2;
    client.write_all(&bytes[..middle]).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    client.write_all(&bytes[middle..]).unwrap();
    client.shutdown(std::net::Shutdown::Write).unwrap();
    let mut reply = Vec::new();
    client.read_to_end(&mut reply).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&reply).unwrap()["ok"], true);
    assert_eq!(thread.join().unwrap(), DEMO_DELTA);
  }

  #[test]
  fn activation_history_has_fixed_capacity_and_keeps_duplicate_result() {
    let _lock = TEST_LOCK.lock().unwrap();
    STOP.store(false, Ordering::SeqCst);
    let revision = "a".repeat(64);
    let mut arm = Arm::default();
    let previous = json!({"schema_version":1,"core":"eos","ok":true,"event_id":"click-old"});
    arm.previous.insert(
      "click-old".into(),
      ("isos/ui/button/trigger".into(), previous.clone()),
    );
    for index in 1..MAX_EVENTS_PER_RUN {
      arm.previous.insert(
        format!("click-{index}"),
        ("isos/ui/button/trigger".into(), previous.clone()),
      );
    }
    let old = json!({"schema_version":1,"core":"eos","op":"activate","run_id":"run_1","revision":revision,"source_node":"isos/ui/button/trigger","event_id":"click-old","kind":"activate"});
    assert_eq!(arm.activate(&old, "run_1", &revision), previous);
    let mut fresh = old.clone();
    fresh["event_id"] = json!("click-new");
    assert_eq!(
      arm.activate(&fresh, "run_1", &revision)["error"],
      "event_capacity"
    );
    assert_eq!(arm.joints, [0.0; 6]);
  }
}
