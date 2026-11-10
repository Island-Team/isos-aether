//! Idle owner setup services. Runtime workers remain behind the admitted Run path.

use crate::{app::config::Paths, runtime::process::core_bins};
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
        Some("idle" | "ready" | "running" | "exited")
      ),
    "owner health identity mismatch"
  );
  Ok(value)
}

fn probe(path: &Path, core: &str) -> Result<()> {
  probe_state(path, core).map(|_| ())
}

fn launch(paths: &Paths, deployment: &str, core: &'static str) -> Result<OwnerService> {
  let binary = executable(core)?;
  let socket = socket(paths, core);
  let metadata = fs::symlink_metadata(&socket);
  if let Ok(metadata) = metadata {
    ensure!(
      metadata.file_type().is_socket(),
      "owner socket path is not a socket: {}",
      socket.display()
    );
    if probe(&socket, core).is_ok() {
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
      issues: Vec::new(),
    };
    for core in ["eos", "ei"] {
      match launch(paths, deployment, core) {
        Ok(service) => services.active.push(service),
        Err(error) => {
          tracing::warn!(event_code = "owner_service_unavailable", core, "{error:#}");
          services.issues.push((core, error.to_string()));
        }
      }
    }
    services
  }

  pub(crate) fn poll(&mut self) {
    let mut index = 0;
    while index < self.active.len() {
      let service = &mut self.active[index];
      let failure = match service.child.as_mut() {
        None => match probe(&service.socket, service.core) {
          Ok(()) => None,
          Err(error) => Some(format!("owner probe failed: {error}")),
        },
        Some(child) => match child.try_wait() {
          Ok(Some(status)) => Some(format!("exited: {status}")),
          Err(error) => Some(format!("poll failed: {error}")),
          Ok(None) => None,
        },
      };
      if let Some(reason) = failure {
        let mut service = self.active.remove(index);
        if let Some(child) = service.child.as_mut() {
          let _ = child.wait();
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
        Ok(value) => json!({"core":core,"state":value["state"]}),
        Err(error) => json!({"core":core,"state":"unavailable","reason":error.to_string()}),
      }
    })
    .collect::<Vec<_>>();
  json!({"owners": owners})
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
  stream
    .take(4 * 1024 * 1024 + 8193)
    .read_to_end(&mut response)?;
  ensure!(
    response.len() <= 4 * 1024 * 1024 + 8192,
    "owner setup response exceeds 4 MiB"
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
