//! Durable identity for the two independently supervised owner workers.
use super::*;
use sha2::{Digest, Sha256};
use std::os::{
  fd::AsRawFd,
  unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
};

struct ReceiptLock(fs::File);

impl ReceiptLock {
  fn take(paths: &config::Paths, name: &str) -> Result<Self> {
    fs::create_dir_all(&paths.runtime)?;
    let file = fs::OpenOptions::new()
      .create(true)
      .truncate(false)
      .read(true)
      .write(true)
      .mode(0o600)
      .open(paths.runtime.join(format!("{name}.owner-run.lock")))?;
    ensure!(
      unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0,
      "cannot lock owner-run receipt"
    );
    Ok(Self(file))
  }
}

impl Drop for ReceiptLock {
  fn drop(&mut self) {
    unsafe {
      libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
    }
  }
}

fn state_path(paths: &config::Paths, name: &str) -> std::path::PathBuf {
  paths.runtime.join(format!("{name}.owner-run.json"))
}

fn emergency_marker_path(paths: &config::Paths, name: &str) -> std::path::PathBuf {
  paths.runtime.join(format!("{name}.emergency-stopped"))
}

pub(crate) fn emergency_stopped(paths: &config::Paths, name: &str) -> bool {
  fs::read(emergency_marker_path(paths, name)).is_ok_and(|bytes| bytes == b"emergency_stop_v1")
    && interrupted_receipt(paths, name).is_ok_and(|receipt| receipt.is_none())
}

pub(super) fn is_current(source: &str) -> bool {
  serde_json::from_str::<Value>(source).is_ok_and(|graph| {
    graph["version"] == "0.2.0"
      && graph["cores"].get("isos").is_some()
      && graph["cores"].get("ei").is_some()
      && graph["cores"].get("eos").is_some()
  })
}

fn selected(paths: &config::Paths, name: &str) -> Result<Option<Value>> {
  match fs::read(state_path(paths, name)) {
    Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(error) => Err(error.into()),
  }
}

fn interrupted_receipt(paths: &config::Paths, name: &str) -> Result<Option<Vec<u8>>> {
  let path = state_path(paths, name);
  let metadata = match fs::symlink_metadata(&path) {
    Ok(metadata) => metadata,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
    Err(error) => return Err(error.into()),
  };
  ensure!(
    metadata.file_type().is_file()
      && metadata.uid() == unsafe { libc::geteuid() }
      && metadata.nlink() == 1
      && metadata.permissions().mode() & 0o7777 == 0o600
      && metadata.len() <= 4096,
    "unsafe owner-run receipt"
  );
  let file = fs::OpenOptions::new()
    .read(true)
    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
    .open(path)?;
  let opened = file.metadata()?;
  ensure!(
    metadata.ino() == opened.ino() && metadata.dev() == opened.dev(),
    "owner-run receipt changed while opening"
  );
  let mut bytes = Vec::new();
  file.take(4097).read_to_end(&mut bytes)?;
  ensure!(bytes.len() <= 4096, "owner-run receipt changed while read");
  let value: Value = serde_json::from_slice(&bytes)?;
  ensure!(
    value.as_object().is_some_and(|fields| {
      fields.len() == 5
        && fields.contains_key("run_id")
        && fields.contains_key("revision")
        && fields.contains_key("pending")
        && fields.contains_key("attempted")
        && fields.contains_key("stopped")
    }) && value["run_id"].as_str().is_some_and(|id| {
      id.starts_with('r')
        && id.len() <= 64
        && id
          .bytes()
          .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    }) && value["revision"]
      .as_str()
      .is_some_and(|revision| revision.len() == 64
        && revision
          .bytes()
          .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
      && value["pending"].is_boolean()
      && value["attempted"].as_array().is_some_and(|items| items
        .iter()
        .all(|item| matches!(item.as_str(), Some("ei" | "eos"))))
      && value["stopped"].as_array().is_some_and(|items| items
        .iter()
        .all(|item| matches!(item.as_str(), Some("ei" | "eos")))),
    "invalid owner-run receipt"
  );
  Ok(Some(bytes))
}

pub(crate) fn interrupted_run_id(paths: &config::Paths, name: &str) -> Result<Option<String>> {
  interrupted_receipt(paths, name)?.map_or(Ok(None), |bytes| {
    let value: Value = serde_json::from_slice(&bytes)?;
    Ok(value["run_id"].as_str().map(str::to_owned))
  })
}

/// Persist the operator's recovery decision before retiring an unconfirmed receipt.
pub(crate) fn publish_operator_recovery(
  paths: &config::Paths,
  name: &str,
  run_id: &str,
) -> Result<()> {
  let _lock = ReceiptLock::take(paths, name)?;
  ensure!(
    std::os::unix::net::UnixStream::connect(paths.socket()).is_err(),
    "stop the IsOS manager before operator recovery"
  );
  let bytes = interrupted_receipt(paths, name)?.context("owner-run receipt is absent")?;
  let receipt: Value = serde_json::from_slice(&bytes)?;
  ensure!(receipt["run_id"] == run_id, "owner-run identity changed");
  let record = json!({
    "schema_version": 1,
    "deployment": name,
    "run_id": run_id,
    "revision": receipt["revision"],
    "receipt_sha256": hex::encode(Sha256::digest(&bytes)),
    "recovery_method": "operator_attestation"
  });
  let audit = paths
    .data
    .join("stops")
    .join(name)
    .join(format!("{run_id}.operator-recovery.json"));
  ensure!(
    interrupted_receipt(paths, name)?.as_deref() == Some(bytes.as_slice()),
    "owner-run receipt changed during recovery"
  );
  ensure!(
    fs::symlink_metadata(&audit).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
    "owner-run recovery already recorded"
  );
  config::atomic_write(&audit, &serde_json::to_vec(&record)?)?;
  fs::remove_file(state_path(paths, name))?;
  Ok(())
}

/// Retire an interrupted receipt only after the emergency process sweep succeeds.
/// This audit records an unproven physical outcome; it is not owner Stop proof.
pub(crate) fn publish_emergency_recovery(paths: &config::Paths, name: &str) -> Result<()> {
  let _lock = ReceiptLock::take(paths, name)?;
  ensure!(
    std::os::unix::net::UnixStream::connect(paths.socket()).is_err(),
    "IsOS manager remains available after emergency Stop"
  );
  let Some(bytes) = interrupted_receipt(paths, name)? else {
    config::atomic_write(&emergency_marker_path(paths, name), b"emergency_stop_v1")?;
    return Ok(());
  };
  let receipt: Value = serde_json::from_slice(&bytes)?;
  let run_id = receipt["run_id"]
    .as_str()
    .context("owner-run ID is absent")?;
  let record = json!({
    "schema_version": 1,
    "deployment": name,
    "run_id": run_id,
    "revision": receipt["revision"],
    "receipt_sha256": hex::encode(Sha256::digest(&bytes)),
    "recovery_method": "emergency_signal_unproven",
    "physical_safety": "unproven"
  });
  let audit = paths
    .data
    .join("stops")
    .join(name)
    .join(format!("{run_id}.emergency-stop.json"));
  ensure!(
    interrupted_receipt(paths, name)?.as_deref() == Some(bytes.as_slice()),
    "owner-run receipt changed during emergency Stop"
  );
  ensure!(
    fs::symlink_metadata(&audit).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
    "emergency Stop was already recorded"
  );
  config::atomic_write(&audit, &serde_json::to_vec(&record)?)?;
  fs::remove_file(state_path(paths, name))?;
  config::atomic_write(&emergency_marker_path(paths, name), b"emergency_stop_v1")?;
  Ok(())
}

fn call(paths: &config::Paths, core: &str, request: Value) -> Result<Value> {
  let reply = crate::runtime::owners::owner_services::setup(paths, core, &request)?;
  ensure!(
    reply["ok"] == true && reply["core"] == core,
    "{core} owner rejected {}: {}",
    request["op"],
    reply
  );
  Ok(reply)
}

fn identity_request(core: &str, op: &str, state: &Value) -> Value {
  json!({"core":core,"op":op,"run_id":state["run_id"],"revision":state["revision"]})
}

pub(super) fn status(paths: &config::Paths, name: &str) -> Result<Value> {
  let saved_revision = starter::revision(&paths.node_graph_path(name)).ok();
  let Some(state) = selected(paths, name)? else {
    return Ok(
      json!({"schema_version":"0.1.0","saved_revision":saved_revision,"active_revision":null,"state":"stopped","last_error":null,"packages":[],"processes":[],"run_id":null,"run_id_state":"unknown","terminal":null}),
    );
  };
  let mut processes = Vec::new();
  let mut healthy = state["pending"] != true;
  for core in ["ei", "eos"] {
    if state["stopped"]
      .as_array()
      .is_some_and(|items| items.iter().any(|item| item == core))
    {
      healthy = false;
      processes.push(json!({"id":core,"core":core,"state":"stopped"}));
      continue;
    }
    match call(paths, core, identity_request(core, "status", &state)) {
      Ok(reply) => {
        let state_name = reply["state"].as_str().unwrap_or("unknown");
        healthy &= matches!(state_name, "running" | "ready");
        processes.push(json!({"id":core,"core":core,"state":state_name,"health":reply}));
      }
      Err(error) => {
        healthy = false;
        processes
          .push(json!({"id":core,"core":core,"state":"unavailable","error":error.to_string()}));
      }
    }
  }
  Ok(
    json!({"schema_version":"0.1.0","saved_revision":saved_revision,"active_revision":state["revision"],"state":if healthy {"running"} else {"degraded"},"last_error":if healthy {Value::Null} else {json!("one or more owner processes are unavailable")},"packages":[],"processes":processes,"run_id":state["run_id"],"run_id_state":"known","terminal":null}),
  )
}

pub(super) fn stop(paths: &config::Paths, name: &str) -> Result<Value> {
  let _lock = ReceiptLock::take(paths, name)?;
  stop_unlocked(paths, name)
}

fn stop_unlocked(paths: &config::Paths, name: &str) -> Result<Value> {
  let Some(mut state) = selected(paths, name)? else {
    return Ok(json!({"stopped":[],"terminal":null}));
  };
  let mut stopped: Vec<String> = state["stopped"]
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .map(str::to_owned)
    .collect();
  let mut failures = Vec::new();
  for core in ["ei", "eos"] {
    if state["attempted"]
      .as_array()
      .is_some_and(|items| !items.iter().any(|item| item == core))
    {
      continue;
    }
    if stopped.iter().any(|item| item == core) {
      continue;
    }
    match call(paths, core, identity_request(core, "stop", &state)) {
      Ok(_) => {
        stopped.push(core.to_owned());
        state["stopped"] = json!(stopped);
        config::atomic_write(&state_path(paths, name), &serde_json::to_vec(&state)?)?;
      }
      Err(error) => failures.push(format!("{core}: {error}")),
    }
  }
  ensure!(
    failures.is_empty(),
    "owner Stop unconfirmed: {}",
    failures.join("; ")
  );
  fs::remove_file(state_path(paths, name))?;
  Ok(json!({"stopped":stopped,"terminal":{"outcome":"confirmed","run_id":state["run_id"]}}))
}

pub(super) fn run(
  paths: &config::Paths,
  name: &str,
  expected: Option<&str>,
  restart: bool,
) -> Result<Value> {
  let _lock = ReceiptLock::take(paths, name)?;
  let path = paths.node_graph_path(name);
  let source = fs::read_to_string(&path)?;
  ensure!(is_current(&source), "saved graph is not a three-core graph");
  let revision = graph_contract::revision(source.as_bytes());
  if let Some(expected) = expected {
    ensure!(
      expected == revision,
      "saved graph revision changed; reload before run"
    );
  }
  validate_saved_graph(paths, name, &source)?;
  core_setup::prepare_run(paths, name, &revision)?;
  ensure!(
    starter::revision(&path)? == revision,
    "saved graph revision changed during core setup"
  );
  if selected(paths, name)?.is_some() {
    ensure!(restart, "deployment is already running; use Restart");
    stop_unlocked(paths, name)?;
  }
  let graph: Value = serde_json::from_str(&source)?;
  let run_id = next_run_id();
  let mut requests = Vec::new();
  for core in ["eos", "ei"] {
    let executable = if core == "eos" {
      crate::runtime::process::core_bins::resolve("aether-runtime")?
    } else {
      std::path::PathBuf::from(
        std::env::var_os("ISOS_EI_BIN_DIR").context("ISOS_EI_BIN_DIR is required")?,
      )
      .join("ei")
    };
    let digest = hex::encode(Sha256::digest(fs::read(&executable)?));
    let projection = crate::domain::graph::single_core::owner_projection(&graph, core)?;
    let mut endpoints = core_setup::endpoints(paths, name, &graph)?;
    if core == "ei" {
      endpoints["eos_owner"] = json!({"transport":"uds","path":crate::runtime::owners::owner_services::socket(paths, "eos")});
      if graph
        .pointer("/cores/ei/layers/governance/groups/monitor/nodes/arm_status")
        .is_some()
      {
        let names = graph
          .pointer("/cores/eos/layers/hardware/groups/mock/nodes/arm/joint_names")
          .context("monitor arm joint names are required")?;
        endpoints["arm_monitor"] = json!({"joint_names": names});
      }
    }
    let request = json!({"core":core,"op":"run","run_id":run_id,"revision":revision,"graph":projection,"endpoints":endpoints,"packages":[],"launch":{"sha256":digest,"args":[],"env":{}}});
    requests.push((core, request));
  }
  let mut state =
    json!({"run_id":run_id,"revision":revision,"pending":true,"attempted":[],"stopped":[]});
  config::atomic_write(&state_path(paths, name), &serde_json::to_vec(&state)?)?;
  match fs::remove_file(emergency_marker_path(paths, name)) {
    Ok(()) => {}
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
    Err(error) => return Err(error.into()),
  }
  let mut started = Vec::new();
  for (core, request) in requests {
    state["attempted"].as_array_mut().unwrap().push(json!(core));
    config::atomic_write(&state_path(paths, name), &serde_json::to_vec(&state)?)?;
    match call(paths, core, request) {
      Ok(_) => started.push(core),
      Err(error) => {
        if started.is_empty() {
          // The failed owner may still have accepted the run before its reply was lost.
          return Err(error);
        }
        let _ = stop_unlocked(paths, name);
        return Err(error);
      }
    }
  }
  state["pending"] = json!(false);
  config::atomic_write(&state_path(paths, name), &serde_json::to_vec(&state)?)?;
  Ok(json!({"started":started,"active_revision":revision,"run_id":run_id}))
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Read, Write};
  use std::os::unix::net::UnixListener;

  fn paths(root: &std::path::Path) -> config::Paths {
    let runtime = root.join("run");
    fs::create_dir_all(&runtime).unwrap();
    config::Paths {
      config: root.join("config"),
      data: root.join("data"),
      runtime,
    }
  }

  #[test]
  fn operator_recovery_requires_exact_receipt_and_persists_audit() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let receipt = json!({"run_id":"r42","revision":"a".repeat(64),"pending":false,"attempted":["eos","ei"],"stopped":[]});
    config::atomic_write(
      &state_path(&paths, "test"),
      &serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    assert_eq!(
      interrupted_run_id(&paths, "test").unwrap().as_deref(),
      Some("r42")
    );
    assert!(publish_operator_recovery(&paths, "test", "r43").is_err());
    assert!(state_path(&paths, "test").exists());
    publish_operator_recovery(&paths, "test", "r42").unwrap();
    assert!(!state_path(&paths, "test").exists());
    let audit: Value = serde_json::from_slice(
      &fs::read(paths.data.join("stops/test/r42.operator-recovery.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(audit["recovery_method"], "operator_attestation");
    assert_eq!(audit["revision"], "a".repeat(64));
    assert!(publish_operator_recovery(&paths, "test", "r42").is_err());
  }

  #[test]
  fn emergency_recovery_records_unproven_outcome_without_matching_run_id() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let receipt = json!({"run_id":"r42","revision":"a".repeat(64),"pending":false,"attempted":["eos","ei"],"stopped":[]});
    config::atomic_write(
      &state_path(&paths, "test"),
      &serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    publish_emergency_recovery(&paths, "test").unwrap();
    assert!(!state_path(&paths, "test").exists());
    let audit: Value = serde_json::from_slice(
      &fs::read(paths.data.join("stops/test/r42.emergency-stop.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(audit["recovery_method"], "emergency_signal_unproven");
    assert_eq!(audit["physical_safety"], "unproven");
  }

  #[test]
  fn rejected_new_run_preserves_emergency_stop_marker() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    config::atomic_write(&emergency_marker_path(&paths, "test"), b"emergency_stop_v1").unwrap();
    let graph = paths.node_graph_path("test");
    config::atomic_write(&graph, b"{}").unwrap();
    assert!(run(&paths, "test", None, false).is_err());
    assert!(emergency_stopped(&paths, "test"));
  }

  fn owner(
    paths: &config::Paths,
    core: &'static str,
    replies: Vec<Value>,
  ) -> std::thread::JoinHandle<()> {
    let socket = crate::runtime::owners::owner_services::socket(paths, core);
    let listener = UnixListener::bind(socket).unwrap();
    std::thread::spawn(move || {
      for reply in replies {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        loop {
          let mut chunk = [0; 4096];
          let count = stream.read(&mut chunk).unwrap();
          assert!(count > 0);
          request.extend_from_slice(&chunk[..count]);
          let Some(separator) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
            continue;
          };
          let headers = std::str::from_utf8(&request[..separator]).unwrap();
          let length: usize = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .unwrap()
            .parse()
            .unwrap();
          if request.len() >= separator + 4 + length {
            break;
          }
        }
        assert!(
          std::str::from_utf8(&request)
            .unwrap()
            .starts_with("POST /v1/stop ")
        );
        let body = reply.to_string();
        write!(
          stream,
          "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
          body.len()
        )
        .unwrap();
      }
    })
  }

  #[test]
  fn partial_stop_keeps_proof_and_retries_only_unconfirmed_owner() {
    let dir = tempfile::tempdir().unwrap();
    let paths = paths(dir.path());
    let name = "test";
    let state = json!({"run_id":"run-1","revision":"rev-1","pending":false,"attempted":["ei","eos"],"stopped":[]});
    config::atomic_write(
      &state_path(&paths, name),
      &serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    let ei = owner(&paths, "ei", vec![json!({"ok":true,"core":"ei"})]);
    let eos = owner(
      &paths,
      "eos",
      vec![
        json!({"ok":false,"core":"eos"}),
        json!({"ok":true,"core":"eos"}),
      ],
    );

    assert!(stop(&paths, name).unwrap_err().to_string().contains("eos"));
    let persisted: Value =
      serde_json::from_slice(&fs::read(state_path(&paths, name)).unwrap()).unwrap();
    assert_eq!(persisted["stopped"], json!(["ei"]));
    let result = stop(&paths, name).unwrap();
    assert_eq!(result["stopped"], json!(["ei", "eos"]));
    assert!(!state_path(&paths, name).exists());
    ei.join().unwrap();
    eos.join().unwrap();
  }

  #[test]
  fn interrupted_run_remains_degraded_until_stop_is_confirmed() {
    let dir = tempfile::tempdir().unwrap();
    let paths = paths(dir.path());
    let name = "test";
    let state =
      json!({"run_id":"run-2","revision":"rev-2","pending":true,"attempted":["eos"],"stopped":[]});
    config::atomic_write(
      &state_path(&paths, name),
      &serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    assert_eq!(status(&paths, name).unwrap()["state"], "degraded");
    let eos = owner(&paths, "eos", vec![json!({"ok":true,"core":"eos"})]);
    let result = stop(&paths, name).unwrap();
    assert_eq!(result["stopped"], json!(["eos"]));
    assert!(!state_path(&paths, name).exists());
    eos.join().unwrap();
  }
}
