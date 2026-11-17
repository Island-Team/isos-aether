//! IsOS-owned, run-fenced package service children. Legacy process pins use their old path.
use super::*;
use crate::domain::{
  launch,
  packages::{self, VerifiedPackage},
};
use crate::runtime::logs::{LogContext, LogStore};
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{
  collections::BTreeMap,
  io::{Read, Seek, SeekFrom},
  os::unix::{
    fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    net::UnixStream,
    process::CommandExt,
  },
  path::PathBuf,
  process::{Command, Stdio},
  thread,
};

#[derive(Clone, Serialize, Deserialize)]
struct Child {
  id: String,
  version: String,
  instance: String,
  pid: u32,
  start_ticks: u64,
  socket: PathBuf,
  generation: String,
  config_digest: String,
  #[serde(default)]
  devices: Vec<String>,
  #[serde(default)]
  stdout: PathBuf,
  #[serde(default)]
  stderr: PathBuf,
  #[serde(default)]
  stdout_offset: u64,
  #[serde(default)]
  stderr_offset: u64,
}
#[derive(Serialize, Deserialize)]
struct Receipt {
  run_id: String,
  revision: String,
  children: Vec<Child>,
}

fn path(paths: &config::Paths, name: &str) -> PathBuf {
  paths.runtime.join(format!("{name}.package-services.json"))
}
fn socket_root() -> Result<PathBuf> {
  let root = PathBuf::from(format!("/tmp/isos-package-{}", unsafe { libc::geteuid() }));
  let mut builder = fs::DirBuilder::new();
  builder.mode(0o700);
  match builder.create(&root) {
    Ok(()) => (),
    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
    Err(error) => return Err(error.into()),
  }
  let metadata = fs::symlink_metadata(&root)?;
  ensure!(
    metadata.file_type().is_dir()
      && metadata.uid() == unsafe { libc::geteuid() }
      && metadata.permissions().mode() & 0o7777 == 0o700,
    "unsafe package service socket directory"
  );
  Ok(root)
}
fn short_socket(paths: &config::Paths, name: &str, run_id: &str, key: &str) -> Result<PathBuf> {
  let root = socket_root()?;
  let mut digest = Sha256::new();
  for part in [
    paths.runtime.as_os_str().as_encoded_bytes(),
    name.as_bytes(),
    run_id.as_bytes(),
    key.as_bytes(),
  ] {
    digest.update((part.len() as u64).to_le_bytes());
    digest.update(part);
  }
  let hash = hex::encode(digest.finalize());
  let socket = root.join(format!("{}.sock", &hash[..32]));
  ensure!(
    socket.as_os_str().as_encoded_bytes().len() + 5 < 108,
    "package service socket path is too long"
  );
  Ok(socket)
}
fn read(paths: &config::Paths, name: &str) -> Result<Option<Receipt>> {
  let path = path(paths, name);
  let meta = match fs::symlink_metadata(&path) {
    Ok(meta) => meta,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
    Err(error) => return Err(error.into()),
  };
  ensure!(
    meta.file_type().is_file()
      && meta.uid() == unsafe { libc::geteuid() }
      && meta.nlink() == 1
      && meta.permissions().mode() & 0o7777 == 0o600
      && meta.len() <= 65536,
    "unsafe package service receipt"
  );
  let file = fs::OpenOptions::new()
    .read(true)
    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
    .open(path)?;
  let opened = file.metadata()?;
  ensure!(
    meta.ino() == opened.ino() && meta.dev() == opened.dev(),
    "package service receipt changed while opening"
  );
  Ok(Some(serde_json::from_reader(file)?))
}
fn ticks(pid: u32) -> Result<(char, u64)> {
  let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
  let fields: Vec<_> = stat
    .rsplit_once(") ")
    .context("invalid child stat")?
    .1
    .split_whitespace()
    .collect();
  Ok((
    fields[0].chars().next().context("empty process state")?,
    fields[19].parse()?,
  ))
}
fn alive(child: &Child) -> bool {
  ticks(child.pid).is_ok_and(|(state, start)| state != 'Z' && start == child.start_ticks)
}
fn clear_dead_stale_receipt(
  paths: &config::Paths,
  name: &str,
  run_id: &str,
  revision: &str,
) -> Result<()> {
  let Some(mut receipt) = read(paths, name)? else {
    return Ok(());
  };
  ensure!(
    receipt.run_id != run_id || receipt.revision != revision,
    "package service receipt already exists"
  );
  ensure!(
    receipt.children.iter().all(|child| !alive(child)),
    "package service receipt belongs to a live previous run"
  );
  drain(paths, name, &mut receipt)?;
  fs::remove_file(path(paths, name))?;
  Ok(())
}
fn probe_health(child: &Child) -> Result<()> {
  ensure!(
    alive(child) && !child.devices.is_empty(),
    "package service child is unavailable"
  );
  for (index, device) in child.devices.iter().enumerate() {
    let mut stream = UnixStream::connect(&child.socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request_id = format!("isos-health-{index}");
    let request = serde_json::to_vec(
      &json!({"version":"2.0.0","id":request_id,"node":device,"op":"health","generation_id":child.generation}),
    )?;
    ensure!(
      request.len() <= 65536,
      "package service health request is too large"
    );
    let mut header = [0u8; 8];
    header[..4].copy_from_slice(&(request.len() as u32).to_le_bytes());
    use std::io::Write;
    stream.write_all(&header)?;
    stream.write_all(&request)?;
    stream.read_exact(&mut header)?;
    let metadata_size = u32::from_le_bytes(header[..4].try_into()?) as usize;
    let payload_size = u32::from_le_bytes(header[4..].try_into()?) as usize;
    ensure!(
      (1..=65536).contains(&metadata_size) && payload_size == 0,
      "invalid service health frame"
    );
    let mut metadata = vec![0u8; metadata_size];
    stream.read_exact(&mut metadata)?;
    let reply: Value = serde_json::from_slice(&metadata)?;
    ensure!(
      reply["version"] == "2.0.0"
        && reply["id"] == request_id
        && reply["ok"] == true
        && reply["accepted"] == true
        && reply["generation_id"] == child.generation
        && reply["features"]
          .as_array()
          .is_some_and(|features| features.iter().any(|feature| feature == "strict-socket-v1")),
      "package service health rejected generation or device"
    );
  }
  Ok(())
}
fn wait_for_exit(child: &Child, deadline: Instant) -> Result<bool> {
  loop {
    if ticks(child.pid).is_ok_and(|(_, start)| start != child.start_ticks) {
      return Ok(false);
    }
    let mut status = 0;
    let waited = unsafe { libc::waitpid(child.pid as i32, &mut status, libc::WNOHANG) };
    if waited == child.pid as i32 {
      return Ok(true);
    }
    if waited < 0 {
      let error = std::io::Error::last_os_error();
      if error.raw_os_error() == Some(libc::ECHILD) {
        return Ok(!alive(child));
      }
      if error.raw_os_error() != Some(libc::EINTR) {
        return Err(error.into());
      }
    }
    if Instant::now() >= deadline {
      return Ok(false);
    }
    thread::sleep(Duration::from_millis(20));
  }
}
fn capture_file(path: &std::path::Path) -> Result<fs::File> {
  Ok(
    fs::OpenOptions::new()
      .write(true)
      .create_new(true)
      .mode(0o600)
      .open(path)?,
  )
}
fn drain_one(
  logs: &LogStore,
  run_id: &str,
  child: &Child,
  stream: &str,
  path: &std::path::Path,
  offset: &mut u64,
) -> Result<()> {
  if path.as_os_str().is_empty() {
    return Ok(());
  }
  let metadata = fs::symlink_metadata(path)?;
  ensure!(
    metadata.file_type().is_file()
      && metadata.permissions().mode() & 0o7777 == 0o600
      && metadata.len() >= *offset,
    "unsafe package service capture"
  );
  let mut file = fs::OpenOptions::new()
    .read(true)
    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
    .open(path)?;
  file.seek(SeekFrom::Start(*offset))?;
  let mut bytes = Vec::new();
  file.take(65_536).read_to_end(&mut bytes)?;
  *offset += bytes.len() as u64;
  for line in bytes.split(|byte| *byte == b'\n') {
    if line.is_empty() {
      continue;
    }
    let text = String::from_utf8_lossy(&line[..line.len().min(4096)]);
    let message: String = text
      .chars()
      .map(|ch| if ch.is_control() { ' ' } else { ch })
      .collect();
    logs.append_event_context("package","info","package_service_output",&message,LogContext {
      run_id:Some(run_id),correlation_id:None,node_path:None,
      fields:json!({"package":child.id,"version":child.version,"instance":child.instance,"stream":stream,"generation":child.generation}),
    })?;
  }
  Ok(())
}
fn drain(paths: &config::Paths, name: &str, receipt: &mut Receipt) -> Result<()> {
  let logs = LogStore::new(paths, name)?;
  for child in &mut receipt.children {
    let snapshot = child.clone();
    drain_one(
      &logs,
      &receipt.run_id,
      &snapshot,
      "stdout",
      &child.stdout,
      &mut child.stdout_offset,
    )?;
    drain_one(
      &logs,
      &receipt.run_id,
      &snapshot,
      "stderr",
      &child.stderr,
      &mut child.stderr_offset,
    )?;
  }
  config::atomic_write(&path(paths, name), &serde_json::to_vec(receipt)?)?;
  Ok(())
}
fn selections(graph: &Value) -> Result<BTreeMap<(String, String, String), Vec<String>>> {
  let mut result: BTreeMap<(String, String, String), Vec<String>> = BTreeMap::new();
  if let Some(layers) = graph
    .pointer("/cores/eos/layers")
    .and_then(Value::as_object)
  {
    for (layer, layer_value) in layers {
      if let Some(groups) = layer_value.get("groups").and_then(Value::as_object) {
        for (group, group_value) in groups {
          if let Some(nodes) = group_value.get("nodes").and_then(Value::as_object) {
            for (node_id, node) in nodes {
              let Some(instance) = node
                .pointer("/adapter/settings/service_instance")
                .and_then(Value::as_str)
              else {
                continue;
              };
              let id = node
                .pointer("/adapter/package/id")
                .and_then(Value::as_str)
                .context("service node lacks exact package id")?;
              let version = node
                .pointer("/adapter/package/version")
                .and_then(Value::as_str)
                .context("service node lacks exact package version")?;
              result
                .entry((id.into(), version.into(), instance.into()))
                .or_default()
                .push(format!("eos/{layer}/{group}/{node_id}"));
            }
          }
        }
      }
    }
  }
  Ok(result)
}
fn config_value(value: &toml::Value, key: &str) -> Result<String> {
  let field = value
    .get(key)
    .with_context(|| format!("package config field {key} is required"))?;
  match field {
    toml::Value::String(text) => Ok(text.clone()),
    toml::Value::Integer(number) => Ok(number.to_string()),
    toml::Value::Boolean(flag) => Ok(flag.to_string()),
    _ => bail!("unsupported runtime config value"),
  }
}
fn environment_value(
  template: &str,
  host: &toml::Value,
  section: &toml::Value,
  generation: &str,
  selected_settings: &str,
) -> Result<String> {
  let mut rendered = String::new();
  let mut rest = template;
  while let Some(start) = rest.find("${") {
    rendered.push_str(&rest[..start]);
    let after = &rest[start + 2..];
    let end = after
      .find('}')
      .context("unclosed runtime environment template")?;
    let token = &after[..end];
    let value = if token == "generation_id" {
      generation.to_owned()
    } else if token == "selected_nodes.settings_json" {
      selected_settings.to_owned()
    } else if let Some(field) = token.strip_prefix("config.host.") {
      config_value(host, field)?
    } else if let Some(field) = token.strip_prefix("config.service.") {
      config_value(section, field)?
    } else {
      bail!("unsupported runtime environment template")
    };
    rendered.push_str(&value);
    rest = &after[end + 1..];
  }
  rendered.push_str(rest);
  ensure!(
    rendered.len() <= 4096 && !rendered.contains('\0'),
    "rendered runtime environment is invalid"
  );
  Ok(rendered)
}
fn child_for<'a>(
  packages: &'a [VerifiedPackage],
  id: &str,
  version: &str,
) -> Result<&'a VerifiedPackage> {
  packages
    .iter()
    .find(|package| package.release.id == id && package.release.version == version)
    .context("selected package service is not verified")
}
fn start_inner(
  paths: &config::Paths,
  name: &str,
  graph: &Value,
  run_id: &str,
  revision: &str,
  packages: &[VerifiedPackage],
) -> Result<Value> {
  clear_dead_stale_receipt(paths, name, run_id, revision)?;
  ensure!(
    read(paths, name)?.is_none(),
    "package service receipt already exists"
  );
  let selected = selections(graph)?;
  let mut receipt = Receipt {
    run_id: run_id.into(),
    revision: revision.into(),
    children: Vec::new(),
  };
  let mut handoff = serde_json::Map::new();
  if selected.is_empty() {
    return Ok(Value::Object(handoff));
  }
  config::atomic_write(&path(paths, name), &serde_json::to_vec(&receipt)?)?;
  for ((id, version, instance), node_paths) in selected {
    let package = child_for(packages, &id, &version)?;
    let service = package
      .release
      .node_contributions
      .as_ref()
      .context("package has no contributions")?
      .runtime_services
      .iter()
      .find(|service| service.config_service_type == instance)
      .context("selected service is undeclared")?;
    let mut devices = Vec::new();
    let mut selected_settings = serde_json::Map::new();
    for node_path in &node_paths {
      let node = graph
        .pointer(&format!(
          "/cores/eos/layers/{}/groups/{}/nodes/{}",
          node_path.split('/').nth(1).unwrap(),
          node_path.split('/').nth(2).unwrap(),
          node_path.split('/').nth(3).unwrap()
        ))
        .context("service node is absent")?;
      let node_type = node["node_type"]
        .as_str()
        .context("service node has no type")?;
      service.check_selected_node_settings(node)?;
      ensure!(
        service
          .node_types
          .iter()
          .any(|declared| declared == node_type),
        "service does not provide selected node type"
      );
      let device = graph
        .pointer(&format!(
          "/cores/eos/layers/{}/groups/{}/nodes/{}/adapter/settings/device_id",
          node_path.split('/').nth(1).unwrap(),
          node_path.split('/').nth(2).unwrap(),
          node_path.split('/').nth(3).unwrap()
        ))
        .and_then(Value::as_str)
        .context("service node lacks device ID")?;
      ensure!(
        !device.is_empty() && device.len() <= 128,
        "invalid service device ID"
      );
      if node_type == "eos.hardware.camera.device@v0.2" {
        ensure!(
          packages::camera_source_declared(package, node_type, &instance, device),
          "camera source is absent from the verified package service"
        );
      }
      devices.push(device.to_owned());
      if let Some(fields) = service.required_node_settings.get(node_type) {
        let mut settings = serde_json::Map::new();
        for field in fields {
          settings.insert(field.clone(), node["settings"][field].clone());
        }
        ensure!(
          selected_settings
            .insert(device.to_owned(), Value::Object(settings))
            .is_none(),
          "duplicate selected service device ID"
        );
      }
    }
    devices.sort();
    devices.dedup();
    let selected_settings = serde_json::to_string(&selected_settings)?;
    let saved = packages::package_config::selected(
      paths,
      name,
      &id,
      &version,
      std::slice::from_ref(&instance),
    )?;
    packages::package_config::external_inputs(service, &saved, &instance)?;
    let source = &saved.values;
    let host = source.get("host").context("package config lacks host")?;
    let section = source
      .get("services")
      .and_then(|v| v.get(&instance))
      .context("selected service config missing")?;
    let entrypoint = packages::checked_file(&package.root, &service.entrypoint, true)?;
    let key = format!("{id}@{version}/{instance}");
    let hash = hex::encode(Sha256::digest(key.as_bytes()));
    let socket = short_socket(paths, name, run_id, &key)?;
    ensure!(
      !socket.exists() && !socket.with_extension("sock.stop").exists(),
      "package service socket already exists"
    );
    let generation = hex::encode(Sha256::digest(
      format!("{run_id}:{revision}:{key}").as_bytes(),
    ));
    let mut args = Vec::new();
    for arg in &service.args {
      if arg == "${socket}" {
        args.push(socket.to_string_lossy().into_owned());
        continue;
      }
      if let Some(resource) = service
        .resources
        .iter()
        .find(|resource| arg == &format!("${{resource.{}.path}}", resource.setting))
      {
        let selected = config_value(section, &resource.setting)?;
        args.push(
          launch::service_resource_path(&package.root, &package.release, resource, &selected)?
            .to_string_lossy()
            .into_owned(),
        );
        continue;
      }
      ensure!(!arg.contains("${"), "unresolved package service argument");
      args.push(arg.clone());
    }
    let mut command = Command::new(entrypoint);
    let mut environment = Vec::new();
    for variable in &service.environment {
      environment.push((
        &variable.name,
        environment_value(
          &variable.value,
          host,
          section,
          &generation,
          &selected_settings,
        )?,
      ));
    }
    let stdout = paths.runtime.join(format!(
      "{name}.pkg-{}-{}.stdout",
      &hash[..16],
      &generation[..16]
    ));
    let stderr = paths.runtime.join(format!(
      "{name}.pkg-{}-{}.stderr",
      &hash[..16],
      &generation[..16]
    ));
    let stdout_file = capture_file(&stdout)?;
    let stderr_file = match capture_file(&stderr) {
      Ok(file) => file,
      Err(error) => {
        let _ = fs::remove_file(&stdout);
        return Err(error);
      }
    };
    command
      .args(args)
      .stdin(Stdio::null())
      .stdout(Stdio::from(stdout_file))
      .stderr(Stdio::from(stderr_file))
      .env("ISOS_RUN_ID", run_id)
      .env("ISOS_GRAPH_REVISION", revision)
      .env("ISOS_PACKAGE_GENERATION", &generation);
    for (name, value) in environment {
      command.env(name, value);
    }
    unsafe {
      command.pre_exec(|| {
        if libc::setsid() < 0 {
          Err(std::io::Error::last_os_error())
        } else {
          Ok(())
        }
      });
    }
    let mut process = match command.spawn() {
      Ok(child) => child,
      Err(error) => {
        let _ = fs::remove_file(&stdout);
        let _ = fs::remove_file(&stderr);
        return Err(error.into());
      }
    };
    let start_ticks = match ticks(process.id()) {
      Ok((_, ticks)) => ticks,
      Err(error) => {
        let _ = process.kill();
        let _ = process.wait();
        let _ = fs::remove_file(&stdout);
        let _ = fs::remove_file(&stderr);
        return Err(error);
      }
    };
    let child = Child {
      id: id.clone(),
      version: version.clone(),
      instance: instance.clone(),
      pid: process.id(),
      start_ticks,
      socket: socket.clone(),
      generation: generation.clone(),
      config_digest: saved.revision.clone(),
      devices,
      stdout: stdout.clone(),
      stderr: stderr.clone(),
      stdout_offset: 0,
      stderr_offset: 0,
    };
    receipt.children.push(child.clone());
    if let Err(error) = config::atomic_write(&path(paths, name), &serde_json::to_vec(&receipt)?) {
      let _ = process.kill();
      let _ = process.wait();
      let _ = fs::remove_file(&stdout);
      let _ = fs::remove_file(&stderr);
      return Err(error);
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
      if !alive(&child) {
        bail!("package service exited before readiness")
      }
      if probe_health(&child).is_ok() {
        break;
      }
      if Instant::now() >= deadline {
        bail!("package service generation-bound health timed out")
      }
      thread::sleep(Duration::from_millis(50));
    }
    for node_path in node_paths {
      handoff.insert(node_path,json!({"package":{"id":id,"version":version},"service_instance":instance,"socket_path":socket,"service_generation":generation,"config_digest":saved.revision}));
    }
  }
  Ok(Value::Object(handoff))
}
pub(crate) fn start_candidate_services(
  paths: &config::Paths,
  name: &str,
  graph: &Value,
  run_id: &str,
  revision: &str,
  packages: &[VerifiedPackage],
) -> Result<Value> {
  match start_inner(paths, name, graph, run_id, revision, packages) {
    Ok(handoff) => Ok(handoff),
    Err(error) => {
      let cleanup = match read(paths, name) {
        Ok(Some(receipt)) if receipt.run_id == run_id && receipt.revision == revision => {
          stop_candidate_services(paths, name, run_id, revision)
        }
        Ok(_) => Ok(()),
        Err(read_error) => Err(read_error),
      };
      match cleanup {
        Ok(()) => Err(error),
        Err(stop_error) => Err(error.context(format!(
          "package service cleanup is unconfirmed: {stop_error:#}"
        ))),
      }
    }
  }
}
pub(crate) fn stop_candidate_services(
  paths: &config::Paths,
  name: &str,
  run_id: &str,
  revision: &str,
) -> Result<()> {
  let Some(mut receipt) = read(paths, name)? else {
    return Ok(());
  };
  ensure!(
    receipt.run_id == run_id && receipt.revision == revision,
    "package service receipt has stale run fence"
  );
  let mut failed = false;
  for child in receipt.children.iter().rev() {
    let was_alive = alive(child);
    if was_alive && unsafe { libc::kill(-(child.pid as i32), libc::SIGTERM) } < 0 {
      failed = true;
    }
    let exited = wait_for_exit(child, Instant::now() + Duration::from_secs(3)).unwrap_or(false);
    let reaped = if exited {
      true
    } else {
      if alive(child) && unsafe { libc::kill(-(child.pid as i32), libc::SIGKILL) } < 0 {
        failed = true;
      }
      wait_for_exit(child, Instant::now() + Duration::from_secs(2)).unwrap_or(false)
    };
    failed |= !was_alive || !reaped;
    let _ = fs::remove_file(&child.socket);
    let _ = fs::remove_file(format!("{}.stop", child.socket.display()));
  }
  drain(paths, name, &mut receipt)?;
  if failed {
    bail!("package service child exit is unconfirmed")
  }
  for child in &receipt.children {
    let _ = fs::remove_file(&child.stdout);
    let _ = fs::remove_file(&child.stderr);
  }
  fs::remove_file(path(paths, name))?;
  Ok(())
}
pub(crate) fn status(
  paths: &config::Paths,
  name: &str,
  run_id: &str,
  revision: &str,
) -> Result<Value> {
  let Some(mut receipt) = read(paths, name)? else {
    return Ok(json!({"ready":true,"children":[]}));
  };
  ensure!(
    receipt.run_id == run_id && receipt.revision == revision,
    "package service status has stale run fence"
  );
  drain(paths, name, &mut receipt)?;
  let children: Vec<_> = receipt.children.iter().map(|child| {
    let ready = probe_health(child).is_ok();
    json!({"id":format!("{}@{}/{}",child.id,child.version,child.instance),"core":"isos","state":if ready {"running"} else {"unavailable"},"ready":ready,"pid":child.pid,"service_generation":child.generation})
  }).collect();
  Ok(json!({"ready":children.iter().all(|child|child["ready"]==true),"children":children}))
}

#[cfg(test)]
mod tests {
  use super::*;
  fn receipt_paths() -> (tempfile::TempDir, config::Paths) {
    let temp = tempfile::tempdir().unwrap();
    let paths = config::Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    fs::create_dir_all(&paths.runtime).unwrap();
    (temp, paths)
  }
  #[test]
  fn dead_previous_run_receipt_is_recovered() {
    let (_temp, paths) = receipt_paths();
    let receipt = Receipt {
      run_id: "old".into(),
      revision: "rev".into(),
      children: vec![Child {
        id: "vendor/sim".into(),
        version: "0.2.1".into(),
        instance: "native".into(),
        pid: std::process::id(),
        start_ticks: 0,
        socket: PathBuf::new(),
        generation: "old".into(),
        config_digest: String::new(),
        devices: vec![],
        stdout: PathBuf::new(),
        stderr: PathBuf::new(),
        stdout_offset: 0,
        stderr_offset: 0,
      }],
    };
    config::atomic_write(
      &path(&paths, "default"),
      &serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    clear_dead_stale_receipt(&paths, "default", "new", "rev").unwrap();
    assert!(read(&paths, "default").unwrap().is_none());
  }
  #[test]
  fn live_previous_run_receipt_is_preserved() {
    let (_temp, paths) = receipt_paths();
    let child = Child {
      id: "vendor/sim".into(),
      version: "0.2.1".into(),
      instance: "native".into(),
      pid: std::process::id(),
      start_ticks: ticks(std::process::id()).unwrap().1,
      socket: PathBuf::new(),
      generation: "old".into(),
      config_digest: String::new(),
      devices: vec![],
      stdout: PathBuf::new(),
      stderr: PathBuf::new(),
      stdout_offset: 0,
      stderr_offset: 0,
    };
    let receipt = Receipt {
      run_id: "old".into(),
      revision: "rev".into(),
      children: vec![child],
    };
    config::atomic_write(
      &path(&paths, "default"),
      &serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    assert!(clear_dead_stale_receipt(&paths, "default", "new", "rev").is_err());
    assert!(read(&paths, "default").unwrap().is_some());
  }
  #[test]
  fn current_run_receipt_is_not_reclaimed() {
    let (_temp, paths) = receipt_paths();
    let receipt = Receipt {
      run_id: "same".into(),
      revision: "rev".into(),
      children: vec![],
    };
    config::atomic_write(
      &path(&paths, "default"),
      &serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    assert!(clear_dead_stale_receipt(&paths, "default", "same", "rev").is_err());
    assert!(read(&paths, "default").unwrap().is_some());
  }
  #[test]
  fn long_deployment_root_uses_short_private_socket() {
    let temp = tempfile::tempdir().unwrap();
    let long = temp.path().join("a".repeat(90)).join("b".repeat(90));
    let paths = config::Paths {
      config: long.join("config"),
      data: long.join("data"),
      runtime: long.join("run"),
    };
    let socket = short_socket(&paths, "deployment", "run-1", "vendor/sim@0.2.1/native").unwrap();
    assert!(socket.as_os_str().as_encoded_bytes().len() + 5 < 108);
    assert!(socket.starts_with(socket_root().unwrap()));
    assert_ne!(
      socket,
      short_socket(&paths, "deployment", "run-2", "vendor/sim@0.2.1/native").unwrap()
    );
    assert_eq!(
      fs::symlink_metadata(socket.parent().unwrap())
        .unwrap()
        .permissions()
        .mode()
        & 0o7777,
      0o700
    );
  }
  #[test]
  fn health_requires_matching_generation_and_selected_device() {
    use std::os::unix::net::UnixListener;
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("health.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let child = Child {
      id: "vendor/sim".into(),
      version: "0.2.1".into(),
      instance: "native".into(),
      pid: std::process::id(),
      start_ticks: ticks(std::process::id()).unwrap().1,
      socket: socket.clone(),
      generation: "g-one".into(),
      config_digest: String::new(),
      devices: vec!["cam-0".into()],
      stdout: PathBuf::new(),
      stderr: PathBuf::new(),
      stdout_offset: 0,
      stderr_offset: 0,
    };
    let server = thread::spawn(move || {
      for generation in ["stale", "g-one"] {
        let (mut stream, _) = listener.accept().unwrap();
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let size = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
        assert_eq!(u32::from_le_bytes(header[4..].try_into().unwrap()), 0);
        let mut body = vec![0u8; size];
        stream.read_exact(&mut body).unwrap();
        let request: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(request["node"], "cam-0");
        assert_eq!(request["generation_id"], "g-one");
        let reply = serde_json::to_vec(
          &json!({"version":"2.0.0","id":request["id"],"ok":true,"accepted":true,
          "features":["strict-socket-v1"],"generation_id":generation}),
        )
        .unwrap();
        header[..4].copy_from_slice(&(reply.len() as u32).to_le_bytes());
        header[4..].copy_from_slice(&0u32.to_le_bytes());
        use std::io::Write;
        stream.write_all(&header).unwrap();
        stream.write_all(&reply).unwrap();
      }
    });
    assert!(probe_health(&child).is_err());
    probe_health(&child).unwrap();
    server.join().unwrap();
  }
  #[test]
  fn wait_never_reaps_a_reused_process_identity() {
    let child = Child {
      id: "vendor/sim".into(),
      version: "0.2.1".into(),
      instance: "native".into(),
      pid: std::process::id(),
      start_ticks: ticks(std::process::id()).unwrap().1 + 1,
      socket: PathBuf::new(),
      generation: String::new(),
      config_digest: String::new(),
      devices: Vec::new(),
      stdout: PathBuf::new(),
      stderr: PathBuf::new(),
      stdout_offset: 0,
      stderr_offset: 0,
    };
    assert!(!wait_for_exit(&child, Instant::now()).unwrap());
  }
  #[test]
  fn wait_reaps_child_after_sigkill() {
    let mut process = Command::new("/bin/sleep").arg("30").spawn().unwrap();
    let child = Child {
      id: "vendor/sim".into(),
      version: "0.2.1".into(),
      instance: "native".into(),
      pid: process.id(),
      start_ticks: ticks(process.id()).unwrap().1,
      socket: PathBuf::new(),
      generation: String::new(),
      config_digest: String::new(),
      devices: Vec::new(),
      stdout: PathBuf::new(),
      stderr: PathBuf::new(),
      stdout_offset: 0,
      stderr_offset: 0,
    };
    assert_eq!(unsafe { libc::kill(process.id() as i32, libc::SIGKILL) }, 0);
    assert!(wait_for_exit(&child, Instant::now() + Duration::from_secs(2)).unwrap());
    assert!(!alive(&child));
    let _ = process.wait();
  }
  #[test]
  fn service_selection_deduplicates_nodes_by_exact_pin_and_instance() {
    let graph = json!({"cores":{"eos":{"layers":{"hardware":{"groups":{"camera":{"nodes":{
      "front":{"node_type":"eos.hardware.camera.device@v0.2","adapter":{"package":{"id":"vendor/sim","version":"0.2.1"},"settings":{"service_instance":"native"}}},
      "rear":{"node_type":"eos.hardware.camera.device@v0.2","adapter":{"package":{"id":"vendor/sim","version":"0.2.1"},"settings":{"service_instance":"native"}}}
    }}}}}}}});
    let selected = selections(&graph).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(
      selected[&(
        String::from("vendor/sim"),
        String::from("0.2.1"),
        String::from("native")
      )]
        .len(),
      2
    );
  }
  #[test]
  fn environment_template_rejects_unknown_tokens() {
    let host: toml::Value = toml::from_str("root = '/opt/sim'").unwrap();
    let service: toml::Value = toml::from_str("seed = 42").unwrap();
    assert_eq!(
      environment_value("${config.host.root}/lib", &host, &service, "g1", "{}").unwrap(),
      "/opt/sim/lib"
    );
    assert_eq!(
      environment_value("${config.service.seed}", &host, &service, "g1", "{}").unwrap(),
      "42"
    );
    assert!(environment_value("${HOME}", &host, &service, "g1", "{}").is_err());
    assert_eq!(
      environment_value(
        "${selected_nodes.settings_json}",
        &host,
        &service,
        "g1",
        "{\"cam-0\":{\"camera_width\":1280}}"
      )
      .unwrap(),
      "{\"cam-0\":{\"camera_width\":1280}}"
    );
  }
  #[test]
  fn stop_reaps_owned_service_and_rejects_stale_run() {
    let temp = tempfile::tempdir().unwrap();
    let paths = config::Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let mut command = Command::new("/bin/sleep");
    command.arg("30");
    unsafe {
      command.pre_exec(|| {
        if libc::setsid() < 0 {
          Err(std::io::Error::last_os_error())
        } else {
          Ok(())
        }
      });
    }
    let mut process = command.spawn().unwrap();
    let start_ticks = ticks(process.id()).unwrap().1;
    let socket = paths.runtime.join("service.sock");
    fs::write(&socket, b"").unwrap();
    let receipt = Receipt {
      run_id: "run-one".into(),
      revision: "a".repeat(64),
      children: vec![Child {
        id: "vendor/sim".into(),
        version: "0.2.1".into(),
        instance: "native".into(),
        pid: process.id(),
        start_ticks,
        socket: socket.clone(),
        generation: "g".into(),
        config_digest: "b".repeat(64),
        devices: vec!["cam-0".into()],
        stdout: PathBuf::new(),
        stderr: PathBuf::new(),
        stdout_offset: 0,
        stderr_offset: 0,
      }],
    };
    config::atomic_write(
      &path(&paths, "default"),
      &serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    assert!(stop_candidate_services(&paths, "default", "old", &"a".repeat(64)).is_err());
    assert!(alive(&receipt.children[0]));
    stop_candidate_services(&paths, "default", "run-one", &"a".repeat(64)).unwrap();
    let _ = process.wait();
    assert!(!socket.exists());
    assert!(!path(&paths, "default").exists());
  }
  #[test]
  fn capture_is_bounded_sanitized_and_run_fenced() {
    let temp = tempfile::tempdir().unwrap();
    let paths = config::Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let file = paths.runtime.join("capture.stdout");
    fs::write(&file, b"ready\x1b[31m\n").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    let logs = LogStore::new(&paths, "default").unwrap();
    let child = Child {
      id: "vendor/sim".into(),
      version: "0.2.1".into(),
      instance: "native".into(),
      pid: 0,
      start_ticks: 0,
      socket: PathBuf::new(),
      generation: "g1".into(),
      config_digest: String::new(),
      devices: vec!["cam-0".into()],
      stdout: file.clone(),
      stderr: PathBuf::new(),
      stdout_offset: 0,
      stderr_offset: 0,
    };
    let mut offset = 0;
    drain_one(&logs, "run-one", &child, "stdout", &file, &mut offset).unwrap();
    assert_eq!(offset, 11);
    let lines = fs::read_to_string(paths.data.join("log/deployments/default/isos.jsonl")).unwrap();
    assert!(lines.contains("\"run_id\":\"run-one\""));
    assert!(lines.contains("package_service_output"));
    assert!(!lines.contains("\\u001b"));
  }
}
