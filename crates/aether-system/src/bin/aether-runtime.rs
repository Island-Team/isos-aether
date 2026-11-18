//! EOS event worker: ingress, built-in skill, dispatcher admission, and mock arm.
use std::{
  env,
  error::Error,
  fs::{self, OpenOptions},
  io::{Read, Write},
  os::unix::{
    fs::PermissionsExt,
    net::{UnixListener, UnixStream},
  },
  path::{Path, PathBuf},
  sync::{atomic::Ordering, mpsc, Arc, Mutex},
  time::Duration,
};
use serde_json::{json, Value};
#[path = "aether_runtime/native_node.rs"]
mod native_node;
use crate::eos_log;
use aether_node::hardware::mock::{Arm, ArmObserver, MAX_EVENTS_PER_RUN, STOP, valid_id};
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
fn valid_revision(value: &str) -> bool {
  value.len() == 64
    && value
      .bytes()
      .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
struct DemoSkills {
  joint_delta: native_node::NativeNode,
  arm_status: native_node::NativeNode,
}
impl DemoSkills {
  fn load() -> Result<Self, Box<dyn Error>> {
    let executable = env::current_exe()?;
    let library = executable.with_file_name("libaether_runtime.so");
    let mut joint_delta = native_node::NativeNode::open(
      &library,
      br#"{"node_type":"eos.skill.demo.joint_delta@v0.2"}"#,
    )?;
    let mut arm_status = match native_node::NativeNode::open(
      &library,
      br#"{"node_type":"eos.skill.demo.arm_status@v0.2"}"#,
    ) {
      Ok(node) => node,
      Err(error) => {
        let _ = joint_delta.stop();
        return Err(error);
      }
    };
    if joint_delta.health() != 1 || arm_status.health() != 1 {
      let _ = arm_status.stop();
      let _ = joint_delta.stop();
      return Err("demo skill library is not ready".into());
    }
    Ok(Self {
      joint_delta,
      arm_status,
    })
  }
  fn delta(&mut self, request: &Value) -> Result<[f64; 6], Box<dyn Error>> {
    let outputs = self
      .joint_delta
      .input(&json!({"payload":request,"port":"activation"}))?;
    let value = outputs
      .iter()
      .find(|output| output["port"] == "delta")
      .ok_or("demo skill delta output missing")?;
    let values = value["payload"]["joint_positions_rad"]
      .as_array()
      .ok_or("demo skill delta malformed")?;
    if values.len() != 6 {
      return Err("demo skill delta length invalid".into());
    }
    let mut delta = [0.0; 6];
    for (target, value) in delta.iter_mut().zip(values) {
      *target = value.as_f64().ok_or("demo skill delta not numeric")?;
    }
    if delta
      .iter()
      .any(|value| !value.is_finite() || value.abs() > 0.25)
    {
      return Err("demo skill delta exceeds dispatcher limit".into());
    }
    Ok(delta)
  }
  fn status(&mut self, observation: &Value) -> Result<Value, Box<dyn Error>> {
    let outputs = self
      .arm_status
      .input(&json!({"payload":observation,"port":"observation"}))?;
    let output = outputs
      .iter()
      .find(|output| output["port"] == "observation")
      .ok_or("demo status skill output missing")?;
    if output["payload"] != *observation {
      return Err("demo status skill altered observation identity".into());
    }
    Ok(output["payload"].clone())
  }
  fn stop(&mut self) -> bool {
    let status_stopped = self.arm_status.stop();
    let delta_stopped = self.joint_delta.stop();
    status_stopped && delta_stopped
  }
}
fn demo_joint_delta(input: &Value) -> Result<[f64; 6], Box<dyn Error>> {
  let mut skills = DemoSkills::load()?;
  let delta = skills.delta(input);
  if !skills.stop() {
    return Err("demo skill Stop failed".into());
  }
  delta
}
fn demo_arm_status(observation: &Value) -> Result<Value, Box<dyn Error>> {
  let mut skills = DemoSkills::load()?;
  let output = skills.status(observation);
  if !skills.stop() {
    return Err("demo skill Stop failed".into());
  }
  output
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
      Ok(value) if value["op"] == "observe_output" => observer.output(&value, run_id, revision),
      Ok(value) if value["op"] == "monitor" => {
        if value["schema_version"] != 1
          || value["core"] != "eos"
          || value["run_id"] != run_id
          || value["revision"] != revision
          || value.as_object().is_none_or(|fields| fields.len() != 5)
        {
          json!({"schema_version":1,"core":"eos","ok":false,"error":"monitor_fence_stale"})
        } else {
          let observation = observer.observe(run_id, revision)?;
          json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"observation":demo_arm_status(&observation)?})
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
  let mut skills = DemoSkills::load()?;
  if !skills.stop() {
    return Err("demo skill Stop failed during readiness check".into());
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
        arm.execute_reserved(
          &event_id,
          &executor_run,
          &executor_revision,
          demo_joint_delta,
        );
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
pub fn main() -> Result<(), Box<dyn Error>> {
  let args: Vec<_> = env::args().collect();
  if args.get(1).is_some_and(|arg| arg == "--probe-adapter") {
    if args.len() != 3 {
      return Err("usage: aether-runtime --probe-adapter <absolute .so>".into());
    }
    let abi = native_node::probe(Path::new(&args[2]))?;
    println!("{}", json!({"ok":true,"abi":abi}));
    return Ok(());
  }
  if args.get(1).is_some_and(|arg| arg == "--native-graph") {
    if args.len() != 5 {
      return Err("usage: aether-runtime --native-graph <socket> <run-id> <revision>".into());
    }
    let mut bytes = Vec::new();
    std::io::stdin()
      .take(1024 * 1024 + 1)
      .read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
      return Err("native graph config exceeds 1 MiB".into());
    }
    let config: Value = serde_json::from_slice(&bytes)?;
    return native_node::serve_graph(Path::new(&args[2]), &args[3], &args[4], &config);
  }
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
  use std::{fs::File, time::Instant};
  use aether_node::hardware::mock::DEMO_DELTA;
  use super::*;
  static TEST_LOCK: Mutex<()> = Mutex::new(());
  #[test]
  fn observer_output_is_typed_fenced_and_closed_on_stop() {
    let _lock = TEST_LOCK.lock().unwrap();
    STOP.store(false, Ordering::SeqCst);
    let revision = "a".repeat(64);
    let arm = Arc::new(Mutex::new(Arm::default()));
    let observer = ArmObserver {
      node_path: "eos/hardware/mock/arm_observer".into(),
      payload_source_node: "eos/hardware/mock/arm_observer".into(),
      arm: Arc::clone(&arm),
    };
    let request = json!({"schema_version":1,"core":"eos","op":"observe_output","run_id":"run_1","revision":revision,"node":"eos/hardware/mock/arm_observer","port":"observation"});
    let read = |request: &Value| {
      let (mut client, mut server) = UnixStream::pair().unwrap();
      client
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
      client.shutdown(std::net::Shutdown::Write).unwrap();
      respond(&mut server, "run_1", &revision, &arm, &observer, None).unwrap();
      drop(server);
      let mut bytes = Vec::new();
      client.read_to_end(&mut bytes).unwrap();
      serde_json::from_slice::<Value>(&bytes).unwrap()
    };
    let first = read(&request);
    assert_eq!(first["ok"], true);
    assert_eq!(first["node"], request["node"]);
    assert_eq!(first["port"], "observation");
    assert_eq!(first["payload"]["source_node"], request["node"]);
    assert_eq!(first["payload"]["run_id"], "run_1");
    assert_eq!(first["payload"]["revision"], revision);
    assert_eq!(
      first["payload"]["joint_positions_rad"],
      json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    );
    let mut stale = request.clone();
    stale["run_id"] = json!("run_2");
    assert_eq!(read(&stale)["error"], "observation_fence_stale");
    stale = request.clone();
    stale["revision"] = json!("b".repeat(64));
    assert_eq!(read(&stale)["error"], "observation_fence_stale");
    stale = request.clone();
    stale["extra"] = json!(true);
    assert_eq!(read(&stale)["error"], "observation_fence_stale");
    let mut wrong_route = request.clone();
    wrong_route["node"] = json!("eos/hardware/mock/arm");
    assert_eq!(read(&wrong_route)["error"], "observation_route_unsupported");
    wrong_route = request.clone();
    wrong_route["port"] = json!("command");
    assert_eq!(read(&wrong_route)["error"], "observation_route_unsupported");
    assert_eq!(arm.lock().unwrap().joints, [0.0; 6]);
    STOP.store(true, Ordering::SeqCst);
    assert_eq!(read(&request)["error"], "worker_stopping");
    STOP.store(false, Ordering::SeqCst);
  }
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
    arm.execute_reserved("click-1", "run_1", &revision, demo_joint_delta);
    assert_eq!(arm.result(&json!({"schema_version":1,"core":"eos","op":"result","run_id":"run_1","revision":revision,"event_id":"click-1"}), "run_1", &revision)["status"], "succeeded");
    assert_eq!(arm.joints, DEMO_DELTA);
    assert_eq!(fs::read_to_string(&path).unwrap().lines().count(), 3);
    arm.execute_reserved("click-1", "run_1", &revision, demo_joint_delta);
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
    restarted.execute_reserved("click-1", "run_1", &revision, demo_joint_delta);
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
    arm.execute_reserved("click-1", "run_1", &revision, demo_joint_delta);
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
