//! Persistent EOS setup and arm activation runtime owner endpoint.

use std::{
  error::Error,
  fs,
  io::{BufRead, BufReader, Read, Write},
  os::unix::{
    fs::{FileTypeExt, PermissionsExt},
    net::{UnixListener, UnixStream},
    process::CommandExt,
  },
  path::Path,
  process::{Child, Command},
  sync::atomic::{AtomicBool, Ordering},
  time::Duration,
};

use aether_hal::{NodeTypeRegistry, Profile};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_REQUEST: usize = 1024 * 1024;
const MAX_REPLY: usize = 4 * 1024 * 1024;
const MAX_HEADER: usize = 8192;
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn request_shutdown(_: libc::c_int) {
  SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

#[derive(Default)]
struct State {
  child: Option<Child>,
  run_id: String,
  revision: String,
  worker_socket: std::path::PathBuf,
  worker_loss_reported: bool,
}

fn stop_child(child: &mut Child, socket: &Path) -> Result<(), String> {
  if let Ok(mut worker) = UnixStream::connect(socket) {
    let _ = worker.set_read_timeout(Some(Duration::from_millis(500)));
    if worker.write_all(b"stop\n").is_ok() {
      let mut reply = [0u8; 512];
      let _ = worker.read(&mut reply);
    }
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

fn valid_button_source(path: &str) -> bool {
  let parts: Vec<_> = path.split('/').collect();
  matches!(parts.as_slice(), ["isos", "ui", "button", id] if !id.is_empty() && id.len() <= 48 && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')))
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
  let path = state
    .worker_socket
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

pub fn serve(socket: &Path, registry: &NodeTypeRegistry) -> Result<(), Box<dyn Error>> {
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
  let mut state = State::default();
  while !SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
    match listener.accept() {
      Ok((mut stream, _)) => {
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let _ = respond(&mut stream, registry, &mut state, socket);
      }
      Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
        std::thread::sleep(Duration::from_millis(20));
      }
      Err(error) => return Err(error.into()),
    }
  }
  if let Some(child) = state.child.as_mut() {
    stop_child(child, &state.worker_socket)
      .map_err(|error| format!("emergency EOS worker stop failed: {error}"))?;
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
    let child_live = state
      .child
      .as_mut()
      .is_some_and(|child| child.try_wait().ok().flatten().is_none());
    let worker = child_live
      .then(|| worker_state(&state.worker_socket, &state.run_id))
      .flatten();
    report_worker_loss(state, worker.is_some());
    return write_json(
      stream,
      200,
      &json!({"schema_version":1,"core":"eos","state":worker.unwrap_or(if state.run_id.is_empty() {"idle"} else {"failed"}),"setup_schema_version":2}),
    );
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
  if matches!(
    op,
    "run" | "status" | "stop" | "activate" | "result" | "monitor"
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
  // The persistent transport has its own v1 envelope; owner setup retains v2.
  request["schema_version"] = json!(2);
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

fn lifecycle(
  op: &str,
  request: &Value,
  registry: &NodeTypeRegistry,
  state: &mut State,
  socket: &Path,
) -> Value {
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
  let worker = child_live
    .then(|| worker_state(&state.worker_socket, &state.run_id))
    .flatten();
  let live = worker.is_some();
  report_worker_loss(state, live);
  if op == "run" {
    if !state.run_id.is_empty() {
      let same = state.run_id == run_id && state.revision == revision;
      return json!({"schema_version":1,"core":"eos","ok":same && live,"state":worker.unwrap_or("failed"),"run_id":state.run_id,"revision":state.revision,"error":if same && live {Value::Null} else {json!("run_conflict")}});
    }
    let Some(graph) = request.get("graph") else {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"missing_graph"});
    };
    let parsed = serde_json::to_string(graph)
      .ok()
      .and_then(|source| Profile::from_json_with_registry(&source, registry).ok());
    if parsed
      .as_ref()
      .is_none_or(|profile| profile.validate_demo_chain().is_err())
      || graph["cores"]
        .as_object()
        .is_none_or(|cores| cores.len() != 1 || !cores.contains_key("eos"))
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_eos_graph"});
    }
    let Some(arm_node) = parsed.as_ref().and_then(|profile| {
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
    let Some((observer_path, legacy_output)) = parsed
      .as_ref()
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
    let joint_limit_rad = joint_limit_rad.min(314.0 / 100.0);
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
    let worker_socket = socket.with_file_name("eos-run.sock");
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
    return json!({"schema_version":1,"core":"eos","ok":ready,"state":if ready {"ready"} else {"failed"},"run_id":run_id,"revision":revision,"worker_pid":state.child.as_ref().map(Child::id),"error":if ready {Value::Null} else {json!("runtime_start_failed")}});
  }
  if state.child.is_none() || state.run_id != run_id || state.revision != revision {
    return json!({"schema_version":1,"core":"eos","ok":false,"error":"stale_or_missing_run"});
  }
  if op == "activate" {
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
    let Ok(mut worker) = UnixStream::connect(&state.worker_socket) else {
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
    let Ok(mut worker) = UnixStream::connect(&state.worker_socket) else {
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
    if request.as_object().is_none_or(|fields| fields.len() != 5) {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_monitor_request"});
    }
    if !live {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_unavailable"});
    }
    let Ok(mut worker) = UnixStream::connect(&state.worker_socket) else {
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
    return json!({"schema_version":1,"core":"eos","ok":true,"state":worker.unwrap_or("failed"),"run_id":run_id,"revision":revision,"worker_pid":state.child.as_ref().map(Child::id),"mock_hardware":worker.unwrap_or("failed")});
  }
  if let Some(child) = state.child.as_mut() {
    if let Err(error) = stop_child(child, &state.worker_socket) {
      return json!({"schema_version":1,"core":"eos","ok":false,"state":"stop_uncertain","run_id":run_id,"revision":revision,"error":error});
    }
  }
  state.child = None;
  state.run_id.clear();
  state.revision.clear();
  state.worker_loss_reported = false;
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
    let status = lifecycle(
      "status",
      &json!({"run_id":"run_1","revision":"a".repeat(64)}),
      &NodeTypeRegistry::builtin(),
      &mut state,
      Path::new("/tmp/eos-owner-test.sock"),
    );
    assert_eq!(status["state"], "degraded");
    assert_eq!(status["mock_hardware"], "degraded");
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
    assert!(reply.contains("\"schema_version\":2"));
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
}
