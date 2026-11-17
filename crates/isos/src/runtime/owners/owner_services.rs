//! Idle owner setup services. Runtime workers remain behind the admitted Run path.

use crate::{
  app::config::{self, Paths},
  runtime::{logs::LogStore, process::core_bins},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
  fs,
  io::{Read, Write},
  os::unix::{
    fs::{FileTypeExt, PermissionsExt},
    net::UnixStream,
    process::CommandExt,
  },
  path::{Path, PathBuf},
  process::{Child, Command, Stdio},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread::JoinHandle,
  time::{Duration, Instant},
};

const PROBE_TIMEOUT: Duration = Duration::from_millis(500);
const START_TIMEOUT: Duration = Duration::from_secs(5);

struct OwnerService {
  core: &'static str,
  socket: PathBuf,
  child: Option<Child>,
}

pub(crate) struct OwnerServices {
  active: Vec<OwnerService>,
  pending: Vec<(&'static str, JoinHandle<Result<OwnerService>>)>,
  issues: Vec<(&'static str, String)>,
}

pub(crate) fn socket(paths: &Paths, core: &str) -> PathBuf {
  paths.runtime.join(format!("{core}-owner.sock"))
}

fn executable(core: &str) -> Result<PathBuf> {
  match core {
    "eos" => core_bins::resolve("aether-profile"),
    "ei" => {
      let directory = std::env::var_os("ISOS_EI_BIN_DIR")
        .context("configure paths.ei_bin_dir for the EI owner service")?;
      let path = PathBuf::from(directory).join("ei-config");
      ensure!(
        path.is_file(),
        "EI owner service is unavailable at {}",
        path.display()
      );
      Ok(path)
    }
    _ => anyhow::bail!("unknown owner core {core}"),
  }
}

fn probe_state(path: &Path, core: &str) -> Result<Value> {
  let mut stream = UnixStream::connect(path)?;
  stream.set_read_timeout(Some(PROBE_TIMEOUT))?;
  stream.set_write_timeout(Some(PROBE_TIMEOUT))?;
  stream.write_all(b"GET /v1/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
  let mut reply = Vec::new();
  stream.take(8192).read_to_end(&mut reply)?;
  let text = std::str::from_utf8(&reply)?;
  let (head, body) = text
    .split_once("\r\n\r\n")
    .context("owner health reply lacks HTTP body")?;
  ensure!(
    head.starts_with("HTTP/1.1 200 ") || head.starts_with("HTTP/1.0 200 "),
    "owner health rejected"
  );
  let value: Value = serde_json::from_str(body)?;
  ensure!(
    value["schema_version"] == 1
      && value["core"] == core
      && matches!(
        value["state"].as_str(),
        Some("idle" | "ready" | "running" | "exited" | "degraded" | "failed")
      ),
    "owner health identity mismatch"
  );
  Ok(value)
}

fn probe(path: &Path, core: &str) -> Result<()> {
  probe_state(path, core).map(|_| ())
}

pub(crate) fn selected_identity(paths: &Paths, core: &str) -> Result<Value> {
  probe_state(&socket(paths, core), core)
}

fn identity_matches(paths: &Paths, deployment: &str, health: &Value) -> Result<bool> {
  let path = paths.runtime.join(format!("{deployment}.owner-run.json"));
  let receipt = match fs::File::open(path) {
    Ok(file) => {
      let mut bytes = Vec::new();
      file.take(4097).read_to_end(&mut bytes)?;
      ensure!(bytes.len() <= 4096, "owner run receipt exceeds bound");
      Some(serde_json::from_slice::<Value>(&bytes)?)
    }
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
    Err(error) => return Err(error.into()),
  };
  Ok(match receipt {
    Some(receipt) => {
      health["run_id"] == receipt["run_id"] && health["revision"] == receipt["revision"]
    }
    None => health["run_id"].is_null() && health["revision"].is_null(),
  })
}

fn launch(paths: &Paths, deployment: &str, core: &'static str) -> Result<OwnerService> {
  let socket = socket(paths, core);
  let metadata = fs::symlink_metadata(&socket);
  if let Ok(metadata) = metadata {
    ensure!(
      metadata.file_type().is_socket(),
      "owner socket path is not a socket: {}",
      socket.display()
    );
    if let Ok(health) = probe_state(&socket, core) {
      ensure!(
        identity_matches(paths, deployment, &health)?,
        "{core} owner run identity differs from IsOS receipt"
      );
      return Ok(OwnerService {
        core,
        socket,
        child: None,
      });
    }
    ensure!(
      UnixStream::connect(&socket).is_err(),
      "owner socket has an incompatible active service: {}",
      socket.display()
    );
    fs::remove_file(&socket)?;
  }
  let binary = executable(core)?;
  let log_dir = paths.owner_data(core).join("log").join(deployment);
  fs::create_dir_all(&log_dir)?;
  fs::set_permissions(&log_dir, fs::Permissions::from_mode(0o700))?;
  let mut command = Command::new(binary);
  command
    .arg("owner")
    .arg("--socket")
    .arg(&socket)
    .env(format!("{}_LOG_DIR", core.to_ascii_uppercase()), &log_dir)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
  unsafe {
    command.pre_exec(|| {
      if libc::setsid() < 0 {
        return Err(std::io::Error::last_os_error());
      }
      Ok(())
    });
  }
  let mut child = command
    .spawn()
    .with_context(|| format!("start {core} owner service"))?;
  let deadline = Instant::now() + START_TIMEOUT;
  loop {
    if probe(&socket, core).is_ok() {
      return Ok(OwnerService {
        core,
        socket,
        child: Some(child),
      });
    }
    if let Some(status) = child.try_wait()? {
      anyhow::bail!("{core} owner service exited before ready: {status}");
    }
    if Instant::now() >= deadline {
      let _ = child.kill();
      let _ = child.wait();
      anyhow::bail!("{core} owner service did not become ready");
    }
    std::thread::sleep(Duration::from_millis(50));
  }
}

impl OwnerServices {
  pub(crate) fn start(paths: &Paths, deployment: &str) -> Self {
    let mut services = Self {
      active: Vec::new(),
      pending: Vec::new(),
      issues: Vec::new(),
    };
    for core in ["eos", "ei"] {
      let paths = paths.clone();
      let deployment = deployment.to_owned();
      services.pending.push((
        core,
        std::thread::spawn(move || launch(&paths, &deployment, core)),
      ));
    }
    services
  }

  pub(crate) fn poll(&mut self) {
    let mut index = 0;
    while index < self.pending.len() {
      if !self.pending[index].1.is_finished() {
        index += 1;
        continue;
      }
      let (core, task) = self.pending.remove(index);
      match task.join() {
        Ok(Ok(service)) => {
          tracing::info!(
            event_code = "owner_service_connected",
            core,
            "owner service ready"
          );
          self.active.push(service);
        }
        Ok(Err(error)) => {
          tracing::warn!(event_code = "owner_service_unavailable", core, "{error:#}");
          self.issues.push((core, error.to_string()));
        }
        Err(_) => {
          tracing::error!(event_code = "owner_service_launch_panicked", core);
          self
            .issues
            .push((core, "owner service launch panicked".into()));
        }
      }
    }
    let mut index = 0;
    while index < self.active.len() {
      let service = &mut self.active[index];
      let failure = match service.child.as_mut() {
        Some(child) => match child.try_wait() {
          Ok(Some(status)) => Some(format!("exited: {status}")),
          Err(error) => Some(format!("poll failed: {error}")),
          Ok(None) => None,
        },
        None => None,
      };
      if let Some(reason) = failure {
        let mut service = self.active.remove(index);
        if let Some(child) = service.child.as_mut()
          && child.try_wait().ok().flatten().is_some()
        {
          let _ = fs::remove_file(&service.socket);
        }
        tracing::error!(
          event_code = "owner_service_failed",
          core = service.core,
          "{reason}"
        );
        self.issues.push((service.core, reason));
        // Never restart an owner automatically after uncertain failure.
      } else {
        index += 1;
      }
    }
  }
}

pub(crate) fn health_report(paths: &Paths) -> Value {
  let owners = ["eos", "ei"]
    .into_iter()
    .map(|core| {
      let path = socket(paths, core);
      match probe_state(&path, core) {
        Ok(value) => json!({"core":core,"state":value["state"],"run_id":value["run_id"],"revision":value["revision"]}),
        Err(error) => json!({"core":core,"state":"unavailable","reason":error.to_string()}),
      }
    })
    .collect::<Vec<_>>();
  json!({"owners": owners})
}

pub(crate) fn log_page(paths: &Paths, core: &str, after: u64) -> Result<Value> {
  let mut stream = UnixStream::connect(socket(paths, core))?;
  stream.set_read_timeout(Some(PROBE_TIMEOUT))?;
  stream.set_write_timeout(Some(PROBE_TIMEOUT))?;
  write!(
    stream,
    "GET /v1/logs?after={after} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
  )?;
  let mut reply = Vec::new();
  stream.take(131_073).read_to_end(&mut reply)?;
  ensure!(reply.len() <= 131_072, "owner log reply exceeds bound");
  let split = reply
    .windows(4)
    .position(|part| part == b"\r\n\r\n")
    .context("owner log reply lacks headers")?;
  let (head, body) = reply.split_at(split + 4);
  ensure!(
    std::str::from_utf8(head)?.starts_with("HTTP/1.1 200 "),
    "owner log request rejected"
  );
  let page: Value = serde_json::from_slice(body)?;
  ensure!(
    page["schema_version"] == 1 && page["core"] == core && page["next_cursor"].as_u64().is_some(),
    "owner log identity mismatch"
  );
  ensure!(
    page["entries"]
      .as_array()
      .is_some_and(|entries| entries.len() <= 2048),
    "owner log page exceeds record bound"
  );
  Ok(page)
}

pub(crate) struct OwnerLogPump {
  stop: Arc<AtomicBool>,
  task: Option<JoinHandle<()>>,
}

impl OwnerLogPump {
  pub(crate) fn start(paths: &Paths, deployment: &str) -> Result<Self> {
    let store = LogStore::new(paths, deployment)?;
    let paths = paths.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    let task = std::thread::spawn(move || {
      let mut cursors = [0_u64; 2];
      for (index, core) in ["eos", "ei"].iter().enumerate() {
        let file = store
          .root()
          .join("owner-pool")
          .join(format!("{core}.cursor"));
        cursors[index] = fs::read_to_string(file)
          .ok()
          .and_then(|raw| raw.trim().parse().ok())
          .unwrap_or(0);
      }
      while !signal.load(Ordering::Relaxed) {
        for (index, core) in ["eos", "ei"].iter().enumerate() {
          let Ok(page) = log_page(&paths, core, cursors[index]) else {
            continue;
          };
          let Some(next) = page["next_cursor"].as_u64() else {
            continue;
          };
          let Some(entries) = page["entries"].as_array() else {
            continue;
          };
          let selected: Vec<Value> = entries
            .iter()
            .filter_map(|entry| {
              let mut entry = entry.clone();
              let level = entry["level"].as_str()?;
              entry["level"] = match level {
                "warning" => json!("warn"),
                "critical" => json!("error"),
                "info" | "warn" | "error" => json!(level),
                _ => return None,
              };
              Some(entry)
            })
            .collect();
          if store.append_owner_pool(core, &selected).is_err() {
            continue;
          }
          if next != cursors[index] {
            let cursor = store
              .root()
              .join("owner-pool")
              .join(format!("{core}.cursor"));
            if config::atomic_write(&cursor, next.to_string().as_bytes()).is_ok() {
              cursors[index] = next;
            }
          }
        }
        std::thread::sleep(Duration::from_millis(250));
      }
    });
    Ok(Self {
      stop,
      task: Some(task),
    })
  }
}

impl Drop for OwnerLogPump {
  fn drop(&mut self) {
    self.stop.store(true, Ordering::Relaxed);
    if let Some(task) = self.task.take() {
      let _ = task.join();
    }
  }
}

/// Send a bounded setup request to an already-started owner service.
pub(crate) fn setup(paths: &Paths, core: &str, request: &Value) -> Result<Value> {
  ensure!(matches!(core, "eos" | "ei"), "unknown owner core");
  let op = request["op"]
    .as_str()
    .context("setup request lacks operation")?;
  ensure!(
    matches!(
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
        | "vision_frame"
        | "latest_output"
        | "vision_subscribe"
    ),
    "unsupported owner setup operation"
  );
  let path = socket(paths, core);
  let mut stream = UnixStream::connect(&path)
    .with_context(|| format!("{core} owner service unavailable at {}", path.display()))?;
  stream.set_read_timeout(Some(Duration::from_secs(10)))?;
  stream.set_write_timeout(Some(Duration::from_secs(10)))?;
  let mut body = request.clone();
  body["schema_version"] = json!(1);
  let body = serde_json::to_vec(&body)?;
  ensure!(
    body.len() <= 1024 * 1024,
    "owner setup request exceeds 1 MiB"
  );
  write!(
    stream,
    "POST /v1/{op} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
    body.len()
  )?;
  stream.write_all(&body)?;
  let mut response = Vec::new();
  let response_limit: usize = if matches!(op, "vision_frame" | "latest_output") {
    // The registered frame payload can be 20 MiB before the owner envelope.
    24 * 1024 * 1024
  } else {
    4 * 1024 * 1024
  };
  stream
    .take((response_limit + 8193) as u64)
    .read_to_end(&mut response)?;
  ensure!(
    response.len() <= response_limit + 8192,
    "owner setup response exceeds bound"
  );
  let separator = response
    .windows(4)
    .position(|part| part == b"\r\n\r\n")
    .context("owner setup response lacks HTTP headers")?;
  let (header, body) = response.split_at(separator + 4);
  let header = std::str::from_utf8(header)?;
  ensure!(
    header.starts_with("HTTP/1.1 200 ")
      || header.starts_with("HTTP/1.1 400 ")
      || header.starts_with("HTTP/1.1 409 ")
      || header.starts_with("HTTP/1.1 422 ")
      || header.starts_with("HTTP/1.1 500 ")
      || header.starts_with("HTTP/1.1 501 ")
      || header.starts_with("HTTP/1.1 503 "),
    "owner setup transport rejected request"
  );
  let reply: Value = serde_json::from_slice(body)?;
  Ok(reply)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn vision_reply_keeps_a_registered_twenty_megabyte_frame() {
    let temporary = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temporary.path().join("config"),
      data: temporary.path().join("data"),
      runtime: temporary.path().join("run"),
    };
    fs::create_dir_all(&paths.runtime).unwrap();
    let listener = std::os::unix::net::UnixListener::bind(socket(&paths, "eos")).unwrap();
    let server = std::thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut reader = std::io::BufReader::new(&mut stream);
      let mut content_length = 0;
      loop {
        let mut line = String::new();
        std::io::BufRead::read_line(&mut reader, &mut line).unwrap();
        if line == "\r\n" {
          break;
        }
        if let Some(length) = line.strip_prefix("Content-Length: ") {
          content_length = length.trim().parse::<usize>().unwrap();
        }
      }
      let mut request_body = vec![0; content_length];
      reader.read_exact(&mut request_body).unwrap();
      drop(reader);
      let body = json!({"ok":true,"frame":{"data":"a".repeat(20 * 1024 * 1024)}}).to_string();
      write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
        body.len()
      )
      .unwrap();
      stream.write_all(body.as_bytes()).unwrap();
    });
    let reply = setup(&paths, "eos", &json!({"op":"vision_frame"})).unwrap();
    server.join().unwrap();
    assert_eq!(
      reply["frame"]["data"].as_str().unwrap().len(),
      20 * 1024 * 1024
    );
  }

  #[test]
  fn owner_reattach_requires_retained_run_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temporary.path().join("config"),
      data: temporary.path().join("data"),
      runtime: temporary.path().join("run"),
    };
    fs::create_dir_all(&paths.runtime).unwrap();
    let idle = json!({"run_id":null,"revision":null});
    assert!(identity_matches(&paths, "demo", &idle).unwrap());
    let running = json!({"run_id":"r1","revision":"a".repeat(64)});
    assert!(!identity_matches(&paths, "demo", &running).unwrap());
    fs::write(
      paths.runtime.join("demo.owner-run.json"),
      json!({"run_id":"r1","revision":"a".repeat(64)}).to_string(),
    )
    .unwrap();
    assert!(identity_matches(&paths, "demo", &running).unwrap());
    assert!(!identity_matches(&paths, "demo", &idle).unwrap());
    assert!(
      !identity_matches(
        &paths,
        "demo",
        &json!({"run_id":"r2","revision":"a".repeat(64)})
      )
      .unwrap()
    );
  }
}
