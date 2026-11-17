use super::*;
use std::{
  collections::HashMap,
  path::PathBuf,
  sync::{Arc, Condvar, Mutex, OnceLock},
};

type PollResult = std::result::Result<Value, String>;
#[derive(Default)]
struct PollState {
  result: Option<PollResult>,
  waiters: usize,
}
type PollFlight = Arc<(Mutex<PollState>, Condvar)>;
type PollKey = (PathBuf, String);

fn poll_flights() -> &'static Mutex<HashMap<PollKey, PollFlight>> {
  static FLIGHTS: OnceLock<Mutex<HashMap<PollKey, PollFlight>>> = OnceLock::new();
  FLIGHTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Coalesce simultaneous read-only UI polls without sharing a snapshot with
/// command admission. The flight is removed immediately after its reply.
pub(crate) fn manager_status_for_poll(paths: &config::Paths, name: &str) -> Result<Value> {
  let key = (paths.socket(), name.to_owned());
  let (flight, leader) = {
    let mut flights = poll_flights()
      .lock()
      .map_err(|_| anyhow::anyhow!("status flights are poisoned"))?;
    match flights.get(&key) {
      Some(flight) => (Arc::clone(flight), false),
      None => {
        let flight = Arc::new((Mutex::new(PollState::default()), Condvar::new()));
        flights.insert(key.clone(), Arc::clone(&flight));
        (flight, true)
      }
    }
  };
  if leader {
    let result = manager_status_for_http(paths, name);
    let shared = result.as_ref().cloned().map_err(ToString::to_string);
    poll_flights()
      .lock()
      .map_err(|_| anyhow::anyhow!("status flights are poisoned"))?
      .remove(&key);
    {
      let mut ready = flight
        .0
        .lock()
        .map_err(|_| anyhow::anyhow!("status flight is poisoned"))?;
      ready.result = Some(shared);
      flight.1.notify_all();
    }
    result
  } else {
    let mut ready = flight
      .0
      .lock()
      .map_err(|_| anyhow::anyhow!("status flight is poisoned"))?;
    ready.waiters += 1;
    flight.1.notify_all();
    while ready.result.is_none() {
      ready = flight
        .1
        .wait(ready)
        .map_err(|_| anyhow::anyhow!("status flight is poisoned"))?;
    }
    ready
      .result
      .as_ref()
      .unwrap()
      .clone()
      .map_err(anyhow::Error::msg)
  }
}

pub(crate) fn manager_call(paths: &config::Paths, name: &str, op: &str) -> Result<Value> {
  manager_call_revision(paths, name, op, None)
}

/// Read-only HTTP status shares the serialized manager queue with Run and Stop.
/// A valid Run may occupy that queue for the full 120-second command budget;
/// allow that budget plus queued owner-health probes before declaring status lost.
/// This preserves the same lifecycle ordering as command admission.
pub(crate) fn manager_status_for_http(paths: &config::Paths, name: &str) -> Result<Value> {
  let deadline = Instant::now() + Duration::from_secs(180);
  let request = WireRequest::new(WireCommand::DeploymentStatus(DeploymentTarget {
    deployment: name.to_owned(),
  }));
  let mut stream = UnixStream::connect(paths.socket()).map_err(|error| {
    anyhow::anyhow!("IsOS manager is not running; run `isos serve` first: {error}")
  })?;
  let mut request_bytes = serde_json::to_vec(&request)?;
  request_bytes.push(b'\n');
  let remaining = deadline.saturating_duration_since(Instant::now());
  ensure!(!remaining.is_zero(), "manager status deadline expired");
  stream.set_write_timeout(Some(remaining))?;
  stream.write_all(&request_bytes)?;
  ensure!(
    Instant::now() <= deadline,
    "manager status deadline expired"
  );
  let mut line = Vec::new();
  loop {
    let remaining = deadline.saturating_duration_since(Instant::now());
    ensure!(!remaining.is_zero(), "manager status deadline expired");
    stream.set_read_timeout(Some(remaining))?;
    let mut chunk = [0u8; 4096];
    let count = stream.read(&mut chunk)?;
    ensure!(count > 0, "manager status response ended early");
    line.extend_from_slice(&chunk[..count]);
    ensure!(
      line.len() <= 1024 * 1024,
      "manager status response is too large"
    );
    if let Some(end) = line.iter().position(|byte| *byte == b'\n') {
      line.truncate(end);
      break;
    }
  }
  ensure!(
    Instant::now() <= deadline,
    "manager status deadline expired"
  );
  let response: ApiResponse = serde_json::from_slice(&line)?;
  ensure!(
    response.schema_version == "0.1.0",
    "manager response schema_version must be 0.1.0"
  );
  if !response.ok {
    return Err(anyhow::Error::new(RemoteDiagnosticError {
      message: response
        .error
        .map_or_else(|| "manager error".into(), |error| error.message),
      data: response.data,
    }));
  }
  Ok(response.data)
}

#[cfg(test)]
mod status_tests {
  use super::*;

  #[test]
  fn status_waits_for_manager_command_longer_than_two_seconds() {
    let temp = tempfile::tempdir().unwrap();
    let paths = config::Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("runtime"),
    };
    fs::create_dir_all(&paths.runtime).unwrap();
    let listener = UnixListener::bind(paths.socket()).unwrap();
    let server = std::thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut line = String::new();
      BufReader::new(stream.try_clone().unwrap())
        .read_line(&mut line)
        .unwrap();
      assert!(line.contains("deployment_status"));
      std::thread::sleep(Duration::from_millis(2200));
      stream.write_all(b"{\"schema_version\":\"0.1.0\",\"ok\":true,\"data\":{\"state\":\"running\"},\"error\":null}\n").unwrap();
    });
    let status = manager_status_for_http(&paths, "default").unwrap();
    assert_eq!(status["state"], "running");
    server.join().unwrap();
  }

  #[test]
  fn simultaneous_polls_share_one_manager_status_request() {
    let temp = tempfile::tempdir().unwrap();
    let paths = config::Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("runtime"),
    };
    fs::create_dir_all(&paths.runtime).unwrap();
    let listener = UnixListener::bind(paths.socket()).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(9));
    let callers: Vec<_> = (0..8)
      .map(|_| {
        let paths = paths.clone();
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
          barrier.wait();
          manager_status_for_poll(&paths, "default").unwrap()
        })
      })
      .collect();
    barrier.wait();
    let (mut stream, _) = listener.accept().unwrap();
    let mut line = String::new();
    BufReader::new(stream.try_clone().unwrap())
      .read_line(&mut line)
      .unwrap();
    let key = (paths.socket(), "default".to_owned());
    let flight = poll_flights().lock().unwrap().get(&key).unwrap().clone();
    let mut state = flight.0.lock().unwrap();
    while state.waiters != 7 {
      let (next, deadline) = flight
        .1
        .wait_timeout(state, Duration::from_secs(5))
        .unwrap();
      assert!(
        !deadline.timed_out(),
        "poll followers did not join the flight"
      );
      state = next;
    }
    drop(state);
    stream.write_all(b"{\"schema_version\":\"0.1.0\",\"ok\":true,\"data\":{\"run_id\":\"same\"},\"error\":null}\n").unwrap();
    for caller in callers {
      assert_eq!(caller.join().unwrap()["run_id"], "same");
    }
    listener.set_nonblocking(true).unwrap();
    assert!(
      matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
    let fresh = std::thread::spawn({
      let paths = paths.clone();
      move || manager_status_for_poll(&paths, "default").unwrap()
    });
    listener.set_nonblocking(false).unwrap();
    let (mut stream, _) = listener.accept().unwrap();
    let mut line = String::new();
    BufReader::new(stream.try_clone().unwrap())
      .read_line(&mut line)
      .unwrap();
    stream.write_all(b"{\"schema_version\":\"0.1.0\",\"ok\":true,\"data\":{\"run_id\":\"after-stop\"},\"error\":null}\n").unwrap();
    assert_eq!(fresh.join().unwrap()["run_id"], "after-stop");
  }

  #[test]
  fn simultaneous_polls_keep_deployments_separate() {
    let temp = tempfile::tempdir().unwrap();
    let paths = config::Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("runtime"),
    };
    fs::create_dir_all(&paths.runtime).unwrap();
    let listener = UnixListener::bind(paths.socket()).unwrap();
    let first = {
      let paths = paths.clone();
      std::thread::spawn(move || manager_status_for_poll(&paths, "first").unwrap())
    };
    let second = {
      let paths = paths.clone();
      std::thread::spawn(move || manager_status_for_poll(&paths, "second").unwrap())
    };
    for _ in 0..2 {
      let (mut stream, _) = listener.accept().unwrap();
      let mut line = String::new();
      BufReader::new(stream.try_clone().unwrap())
        .read_line(&mut line)
        .unwrap();
      let name = if line.contains("first") {
        "first"
      } else {
        "second"
      };
      let reply = format!(
        "{{\"schema_version\":\"0.1.0\",\"ok\":true,\"data\":{{\"name\":\"{name}\"}},\"error\":null}}\n"
      );
      stream.write_all(reply.as_bytes()).unwrap();
    }
    assert_eq!(first.join().unwrap()["name"], "first");
    assert_eq!(second.join().unwrap()["name"], "second");
  }
}

pub(crate) fn manager_call_revision(
  paths: &config::Paths,
  name: &str,
  op: &str,
  revision: Option<String>,
) -> Result<Value> {
  let started_at = Instant::now();
  if matches!(
    op,
    "deployment_start" | "deployment_restart" | "services_start" | "services_restart"
  ) {
    check_manager_binary(paths, name)?;
    emit_phase_timing(
      op,
      "cli_manager_identity",
      started_at.elapsed().as_millis() as u64,
      started_at.elapsed().as_millis() as u64,
    );
  }
  let request = WireCommand::deployment_call(op, name, revision)?;
  manager_request(paths, request)
}

pub(crate) fn manager_request(paths: &config::Paths, request: WireCommand) -> Result<Value> {
  let op = request.op();
  let timed = matches!(request.op(), "deployment_start" | "deployment_restart");
  let started_at = Instant::now();
  let mut stream = UnixStream::connect(paths.socket()).map_err(|error| {
    anyhow::anyhow!("IsOS manager is not running; run `isos serve` first: {error}")
  })?;
  stream.set_write_timeout(Some(Duration::from_secs(5)))?;
  stream.set_read_timeout(Some(if timed {
    Duration::from_secs(120)
  } else {
    Duration::from_secs(15)
  }))?;
  serde_json::to_writer(&mut stream, &WireRequest::new(request))?;
  stream.write_all(b"\n")?;
  if timed {
    emit_phase_timing(
      op,
      "cli_socket_send",
      started_at.elapsed().as_millis() as u64,
      started_at.elapsed().as_millis() as u64,
    );
  }
  let mut line = String::new();
  BufReader::new(stream)
    .take(1_048_577)
    .read_line(&mut line)?;
  ensure!(line.len() <= 1_048_576, "manager response exceeds 1 MB");
  ensure!(line.ends_with('\n'), "manager response ended early");
  if timed {
    emit_phase_timing(
      op,
      "cli_response",
      started_at.elapsed().as_millis() as u64,
      started_at.elapsed().as_millis() as u64,
    );
  }
  let response: ApiResponse = serde_json::from_str(&line)?;
  ensure!(
    response.schema_version == "0.1.0",
    "manager response schema_version must be 0.1.0"
  );
  if !response.ok {
    return Err(anyhow::Error::new(RemoteDiagnosticError {
      message: response
        .error
        .map_or_else(|| "manager error".into(), |error| error.message),
      data: response.data,
    }));
  }
  Ok(response.data)
}

#[derive(Debug)]
pub(crate) struct RemoteDiagnosticError {
  pub message: String,
  pub data: Value,
}
impl std::fmt::Display for RemoteDiagnosticError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}
impl std::error::Error for RemoteDiagnosticError {}

pub(crate) fn check_manager_binary(paths: &config::Paths, name: &str) -> Result<()> {
  let active = manager_call(paths, name, "manager_identity")?;
  let own = hex::encode(Sha256::digest(fs::read(std::env::current_exe()?)?));
  ensure!(
    active["executable_sha256"] == own,
    "running IsOS manager uses a different binary; run `isos stop`, then `isos serve` with this version"
  );
  Ok(())
}

pub(crate) fn service_call(
  paths: &config::Paths,
  name: &str,
  op: &str,
  id: Option<String>,
  limit: Option<usize>,
) -> Result<Value> {
  if matches!(op, "services_start" | "services_restart") {
    check_manager_binary(paths, name)?;
  }
  manager_request(paths, WireCommand::service_call(op, id, limit)?)
}
