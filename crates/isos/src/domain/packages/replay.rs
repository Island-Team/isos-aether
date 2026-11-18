//! Isolated candidate replay through the real EOS owner boundary.

use super::{Release, digest, verify, verified_adapter_library};
use crate::{
  app::config::{self, Paths},
  domain::{
    graph::{single_core, graph_contract},
    launch,
  },
  runtime::process::core_bins,
};

pub(super) fn qualification_name(graph: &Value) -> Result<String> {
  let revision = graph_contract::revision(&serde_json::to_vec_pretty(graph)?);
  Ok(format!("pkg_{}", &revision[..12]))
}
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
  collections::BTreeSet,
  fs,
  io::{Read, Write},
  net::Shutdown,
  os::unix::net::UnixStream,
  path::{Path, PathBuf},
  process::{Child, Command, Stdio},
  thread,
  time::{Duration, Instant},
};

/// Every selected service must be declared by the exact package release.
pub(super) fn selected_service_instances(
  graphs: &[PathBuf],
  release: &Release,
) -> Result<Vec<String>> {
  let mut selected = BTreeSet::new();
  let contributions = release
    .node_contributions
    .as_ref()
    .context("release has no node contributions")?;
  let declared = &contributions.runtime_services;
  for graph in graphs {
    let graph: Value = crate::runtime::process::normalize_source(&fs::read_to_string(graph)?)?;
    let groups = graph
      .pointer("/cores/eos/layers/hardware/groups")
      .and_then(Value::as_object)
      .context("candidate graph has no EOS hardware")?;
    let mut found = false;
    let mut devices = BTreeSet::new();
    for entry in groups.values() {
      let Some(nodes) = entry.get("nodes").and_then(Value::as_object) else {
        continue;
      };
      for node in nodes.values() {
        if node["adapter"]["package"]["id"] != release.id
          || node["adapter"]["package"]["version"] != release.version
        {
          continue;
        }
        found = true;
        let instance = node
          .pointer("/adapter/settings/service_instance")
          .and_then(Value::as_str)
          .context("package hardware node lacks service_instance")?;
        let node_type = node["node_type"]
          .as_str()
          .context("hardware node lacks type")?;
        let device_id = node
          .pointer("/adapter/settings/device_id")
          .and_then(Value::as_str)
          .context("package hardware node lacks device_id")?;
        ensure!(
          declared
            .iter()
            .any(|service| service.config_service_type == instance
              && service.node_types.iter().any(|ty| ty == node_type)),
          "undeclared service or hardware node type {instance}/{node_type}"
        );
        selected.insert(instance.to_owned());
        ensure!(
          devices.insert((instance.to_owned(), device_id.to_owned())),
          "duplicate package device {instance}/{device_id}"
        );
      }
    }
    ensure!(found, "candidate graph selects no checked package hardware");
    let instances = devices
      .iter()
      .map(|(instance, _)| instance.as_str())
      .collect::<BTreeSet<_>>();
    ensure!(
      instances.len() == 1,
      "each candidate graph must select one runtime service"
    );
    let instance = instances
      .iter()
      .next()
      .context("candidate graph has no service")?;
    let preset = contributions
      .node_sets
      .iter()
      .find(|set| set.id == **instance)
      .context("runtime service has no matching node preset")?;
    let expected = preset
      .nodes
      .iter()
      .map(|node| {
        node
          .profile
          .pointer("/adapter/settings/device_id")
          .and_then(Value::as_str)
          .map(|device| ((*instance).to_owned(), device.to_owned()))
          .context("preset node has no device ID")
      })
      .collect::<Result<BTreeSet<_>>>()?;
    ensure!(
      devices == expected,
      "candidate graph must select every device in {instance} preset exactly once"
    );
  }
  ensure!(
    !selected.is_empty(),
    "candidate graphs select no runtime services"
  );
  Ok(selected.into_iter().collect())
}

struct Owner {
  child: Child,
  socket: PathBuf,
}
impl Drop for Owner {
  fn drop(&mut self) {
    let _ = self.child.kill();
    let _ = self.child.wait();
    let _ = fs::remove_file(&self.socket);
  }
}

fn exchange(socket: &Path, op: &str, request: &Value) -> Result<Value> {
  let bytes = serde_json::to_vec(request)?;
  ensure!(
    bytes.len() <= 1024 * 1024,
    "owner replay request exceeds 1 MiB"
  );
  let mut stream = UnixStream::connect(socket)?;
  stream.set_read_timeout(Some(Duration::from_secs(5)))?;
  stream.set_write_timeout(Some(Duration::from_secs(5)))?;
  write!(
    stream,
    "POST /v1/{op} HTTP/1.1\r\nHost: owner\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
    bytes.len()
  )?;
  stream.write_all(&bytes)?;
  stream.shutdown(Shutdown::Write)?;
  let mut reply = Vec::new();
  stream.take(20 * 1024 * 1024 + 1).read_to_end(&mut reply)?;
  ensure!(
    reply.len() <= 20 * 1024 * 1024,
    "owner replay reply exceeds 20 MiB"
  );
  let split = reply
    .windows(4)
    .position(|part| part == b"\r\n\r\n")
    .context("owner replay reply has no HTTP body")?
    + 4;
  let head = std::str::from_utf8(&reply[..split])?;
  ensure!(
    ["200", "409", "422", "503"]
      .iter()
      .any(|status| head.starts_with(&format!("HTTP/1.1 {status} "))),
    "owner replay {op} returned unexpected HTTP response"
  );
  Ok(serde_json::from_slice(&reply[split..])?)
}

fn selected_process(graph: &Value, release: &Release) -> Result<(String, String, Value)> {
  let groups = graph
    .pointer("/cores/eos/layers/process/groups")
    .and_then(Value::as_object)
    .context("candidate graph has no EOS package process")?;
  let mut selected = None;
  for (group, entry) in groups {
    let Some(nodes) = entry.get("nodes").and_then(Value::as_object) else {
      continue;
    };
    for (id, node) in nodes {
      if node["package"]["id"] == release.id && node["package"]["version"] == release.version {
        ensure!(
          selected.is_none(),
          "candidate graph selects multiple package processes"
        );
        let ty = node["node_type"]
          .as_str()
          .context("package process type missing")?;
        selected = Some((
          format!("eos/process/{group}/{id}"),
          ty.to_owned(),
          node["settings"].clone(),
        ));
      }
    }
  }
  selected.context("candidate graph does not select checked package process")
}

fn observer_nodes(eos: &Value, release: &Release) -> Result<Vec<String>> {
  let mut nodes = Vec::new();
  if let Some(groups) = eos
    .pointer("/cores/eos/layers/hardware/groups")
    .and_then(Value::as_object)
  {
    for (group, entry) in groups {
      if let Some(instances) = entry.get("nodes").and_then(Value::as_object) {
        for (id, node) in instances {
          if node["adapter"]["package"]["id"] == release.id
            && node["adapter"]["package"]["version"] == release.version
          {
            ensure!(
              node["node_type"] == "eos.hardware.camera.device@v0.2",
              "managed replay does not yet exercise package hardware node eos/hardware/{group}/{id}"
            );
            nodes.push(format!("eos/hardware/{group}/{id}"));
          }
        }
      }
    }
  }
  Ok(nodes)
}

/// Replays the candidate's selected process and camera interfaces against
/// the exact isolated installation and writes the full owner exchange.
pub(super) fn managed_replay(
  paths: &Paths,
  release: &Release,
  graph_path: &Path,
  output: &Path,
) -> Result<()> {
  if release.schema_version == "0.2.1" {
    return managed_replay_v2(paths, release, graph_path, output);
  }
  let graph: Value = crate::runtime::process::normalize_source(&fs::read_to_string(graph_path)?)?;
  let eos = single_core::owner_projection(&graph, "eos")?;
  let (node_path, node_type, settings) = selected_process(&graph, release)?;
  let package = launch::PackageIdentity {
    id: release.id.clone(),
    version: release.version.clone(),
  };
  let resolved = launch::resolve(paths, &package, &node_type, &node_path, &settings)?
    .context("selected package process has no verified launch")?;
  let verified = verify(paths, &release.id, &release.version)?;
  let sidecar = verified
    .sidecar
    .context("checked package has no node type sidecar")?;
  let observers = observer_nodes(&eos, release)?;
  ensure!(
    !observers.is_empty(),
    "candidate has no package camera observer to exercise"
  );
  let root = output.join("managed-replay");
  fs::create_dir(&root)?;
  let logs = root.join("logs");
  fs::create_dir(&logs)?;
  let handoff = root.join("package-process.json");
  config::atomic_write(&handoff, &resolved.launch_bytes)?;
  let socket_guard = tempfile::Builder::new().prefix("isos-replay-").tempdir()?;
  let owner_socket = socket_guard.path().join("eos-owner.sock");
  let runtime = core_bins::resolve("aether-runtime")?;
  let profile = core_bins::resolve("aether-profile")?;
  let mut command = Command::new(profile);
  command
    .args(["owner", "--socket"])
    .arg(&owner_socket)
    .arg("--node-types")
    .arg(&sidecar)
    .arg("--package-process")
    .arg(&handoff)
    .env("EOS_LOG_DIR", &logs)
    .envs(resolved.environment)
    .stdin(Stdio::null())
    .stdout(Stdio::from(fs::File::create(
      root.join("owner.stdout.log"),
    )?))
    .stderr(Stdio::from(fs::File::create(
      root.join("owner.stderr.log"),
    )?));
  for adapter in &release.adapters {
    let (library, sha) =
      verified_adapter_library(paths, &release.id, &release.version, &adapter.family)?;
    command.arg("--adapter-library").arg(format!(
      "{}={}@{}",
      adapter.family,
      library.display(),
      sha
    ));
  }
  let mut owner = Owner {
    child: command.spawn().context("launch EOS replay owner")?,
    socket: owner_socket,
  };
  let deadline = Instant::now() + Duration::from_secs(5);
  while !owner.socket.exists() {
    ensure!(
      Instant::now() < deadline,
      "EOS replay owner did not become ready"
    );
    ensure!(
      owner.child.try_wait()?.is_none(),
      "EOS replay owner exited during setup"
    );
    thread::sleep(Duration::from_millis(20));
  }
  let revision = digest(&serde_json::to_vec(&graph)?);
  let run_id = "pkg_check_1";
  let packages = json!([{"id":release.id,"version":release.version,
    "path":verified.store,"payload_root":verified.root}]);
  let run = exchange(
    &owner.socket,
    "run",
    &json!({"schema_version":1,"core":"eos","op":"run",
    "run_id":run_id,"revision":revision,"graph":eos,"packages":packages,
    "launch":{"sha256":digest(&fs::read(runtime)?),"args":[],"env":{}}}),
  )?;
  config::atomic_write(&root.join("run.json"), &serde_json::to_vec_pretty(&run)?)?;
  if (run["ok"] != true || run["state"] != "ready")
    && let Ok(stop) = exchange(
      &owner.socket,
      "stop",
      &json!({"schema_version":1,"core":"eos",
      "op":"stop","run_id":run_id,"revision":revision}),
    )
  {
    let _ = config::atomic_write(
      &root.join("stop-after-failed-run.json"),
      &serde_json::to_vec_pretty(&stop)?,
    );
  }
  ensure!(
    run["ok"] == true && run["state"] == "ready",
    "EOS replay Run failed: {run}"
  );
  let replay = (|| -> Result<()> {
    let status = exchange(
      &owner.socket,
      "status",
      &json!({"schema_version":1,"core":"eos","op":"status","run_id":run_id,"revision":revision}),
    )?;
    config::atomic_write(
      &root.join("status.json"),
      &serde_json::to_vec_pretty(&status)?,
    )?;
    ensure!(
      status["ok"] == true && status["state"] == "ready",
      "EOS replay readiness failed: {status}"
    );
    let stale = exchange(
      &owner.socket,
      "observe",
      &json!({"schema_version":1,"core":"eos",
      "op":"observe","run_id":run_id,"revision":"0".repeat(64),"node":observers[0]}),
    )?;
    config::atomic_write(
      &root.join("stale-observe.json"),
      &serde_json::to_vec_pretty(&stale)?,
    )?;
    ensure!(
      stale["ok"] == false,
      "EOS replay accepted stale observation identity"
    );
    let direct = exchange(
      &owner.socket,
      "input",
      &json!({"schema_version":1,"core":"eos",
      "op":"input","run_id":run_id,"revision":revision,"node":observers[0],
      "source_node":"isos/ui/direct","source_port":"command","port":"command",
      "payload":{"op":"move"}}),
    )?;
    config::atomic_write(
      &root.join("rejected-direct-control.json"),
      &serde_json::to_vec_pretty(&direct)?,
    )?;
    ensure!(
      direct["ok"] == false,
      "EOS replay admitted undeclared direct control route"
    );
    for (index, node) in observers.iter().enumerate() {
      let observed = exchange(
        &owner.socket,
        "observe",
        &json!({"schema_version":1,"core":"eos","op":"observe","run_id":run_id,"revision":revision,"node":node}),
      )?;
      config::atomic_write(
        &root.join(format!("observe-{index}.json")),
        &serde_json::to_vec_pretty(&observed)?,
      )?;
      ensure!(
        observed["ok"] == true
          && observed["outputs"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "EOS replay observation failed for {node}: {observed}"
      );
    }
    Ok(())
  })();
  let stopped = exchange(
    &owner.socket,
    "stop",
    &json!({"schema_version":1,"core":"eos","op":"stop","run_id":run_id,"revision":revision}),
  )?;
  config::atomic_write(
    &root.join("stop.json"),
    &serde_json::to_vec_pretty(&stopped)?,
  )?;
  ensure!(
    stopped["ok"] == true && stopped["state"] == "stopped",
    "EOS replay Stop unconfirmed: {stopped}"
  );
  let socket_removed = !socket_guard.path().join("eos-package.sock").exists();
  config::atomic_write(
    &root.join("socket-cleanup.json"),
    &serde_json::to_vec_pretty(&json!({"package_socket_removed":socket_removed}))?,
  )?;
  ensure!(socket_removed, "EOS replay package socket survived Stop");
  let log = fs::read(logs.join("eos.jsonl")).context("EOS replay log missing")?;
  ensure!(
    log
      .windows(run_id.len())
      .any(|part| part == run_id.as_bytes()),
    "EOS replay log lacks run identity"
  );
  replay?;
  Ok(())
}

fn run_log(run_id: &str, mut page_at: impl FnMut(u64) -> Result<Value>) -> Result<Vec<u8>> {
  let mut log = Vec::new();
  let mut cursor = 0;
  for _ in 0..256 {
    let page = page_at(cursor)?;
    for entry in page["entries"]
      .as_array()
      .context("EOS manager log entries missing")?
    {
      if entry["run_id"] == run_id {
        serde_json::to_writer(&mut log, entry)?;
        log.push(b'\n');
      }
    }
    let next = page["next_cursor"]
      .as_u64()
      .context("EOS manager log cursor missing")?;
    if next == cursor {
      return Ok(log);
    }
    ensure!(next > cursor, "EOS manager log cursor regressed");
    cursor = next;
  }
  anyhow::bail!("EOS manager log exceeds page limit")
}

fn managed_replay_v2(
  paths: &Paths,
  release: &Release,
  graph_path: &Path,
  output: &Path,
) -> Result<()> {
  let graph: Value = crate::runtime::process::normalize_source(&fs::read_to_string(graph_path)?)?;
  let eos = single_core::owner_projection(&graph, "eos")?;
  let verified = verify(paths, &release.id, &release.version)?;
  let root = output.join("managed-replay");
  let revision = graph_contract::revision(&serde_json::to_vec_pretty(&graph)?);
  let name = qualification_name(&graph)?;
  let applied = graph_contract::read(&paths.node_graph_path(&name))?;
  ensure!(
    applied.revision == revision && applied.value == graph,
    "candidate graph was not Applied exactly"
  );
  crate::runtime::owners::core_setup::verify_launch_inputs(paths, &name)?;
  let _owners = crate::runtime::owners::owner_services::OwnerServices::start(paths, &name);
  let owner_socket = crate::runtime::owners::owner_services::socket(paths, "eos");
  let run = crate::runtime::manager::owner_deployment::run(paths, &name, Some(&revision), false)
    .context("IsOS manager Run failed")?;
  config::atomic_write(&root.join("run.json"), &serde_json::to_vec_pretty(&run)?)?;
  let run_id = run["run_id"]
    .as_str()
    .context("manager Run has no run ID")?
    .to_owned();
  let receipt: Value = serde_json::from_slice(&fs::read(
    paths.runtime.join(format!("{name}.package-services.json")),
  )?)?;
  let sockets = receipt["children"]
    .as_array()
    .context("manager package service receipt has no children")?
    .iter()
    .map(|child| {
      child["socket"]
        .as_str()
        .map(str::to_owned)
        .context("service socket missing")
    })
    .collect::<Result<Vec<_>>>()?;
  let replay = (|| -> Result<()> {
    let status = exchange(
      &owner_socket,
      "status",
      &json!({"schema_version":1,"core":"eos","op":"status","run_id":run_id,"revision":revision}),
    )?;
    config::atomic_write(
      &root.join("status.json"),
      &serde_json::to_vec_pretty(&status)?,
    )?;
    ensure!(
      status["ok"] == true && status["state"] == "ready",
      "EOS replay readiness failed: {status}"
    );
    let mut observed_count = 0;
    for (group, entry) in eos
      .pointer("/cores/eos/layers/hardware/groups")
      .and_then(Value::as_object)
      .context("EOS hardware missing")?
    {
      for (id, node) in entry["nodes"]
        .as_object()
        .context("EOS hardware group has no nodes")?
      {
        if node["adapter"]["package"]["id"] != release.id
          || node["adapter"]["package"]["version"] != release.version
        {
          continue;
        }
        if node.get("observation").is_none() {
          continue;
        }
        let path = format!("eos/hardware/{group}/{id}");
        let observed = exchange(
          &owner_socket,
          "observe",
          &json!({"schema_version":1,"core":"eos","op":"observe","run_id":run_id,"revision":revision,"node":path}),
        )?;
        config::atomic_write(
          &root.join(format!("observe-{observed_count}.json")),
          &serde_json::to_vec_pretty(&observed)?,
        )?;
        ensure!(
          observed["ok"] == true
            && observed["outputs"]
              .as_array()
              .is_some_and(|items| !items.is_empty()),
          "EOS observation failed for {path}: {observed}"
        );
        observed_count += 1;
      }
    }
    ensure!(
      observed_count > 0,
      "candidate has no observable package hardware"
    );
    let mut command_count = 0;
    for (group, entry) in eos
      .pointer("/cores/eos/layers/hardware/groups")
      .and_then(Value::as_object)
      .context("EOS hardware missing")?
    {
      for (id, node) in entry["nodes"]
        .as_object()
        .context("EOS hardware group has no nodes")?
      {
        if node["adapter"]["package"]["id"] != release.id
          || node["adapter"]["package"]["version"] != release.version
        {
          continue;
        }
        let Some(node_type) = node["node_type"].as_str() else {
          continue;
        };
        let cases = command_cases(node_type, node);
        for (port, payload) in cases {
          let path = format!("eos/hardware/{group}/{id}");
          let command_id = format!("pkg-check-{command_count}");
          let envelope = json!({"schema_version":1,"core":"eos","op":"command","run_id":run_id,"revision":revision,
            "node":path,"port":port,"command_id":command_id,"payload":payload});
          let reply = exchange(&owner_socket, "command", &envelope)?;
          config::atomic_write(
            &root.join(format!("command-{command_count}.json")),
            &serde_json::to_vec_pretty(&reply)?,
          )?;
          ensure!(
            reply["ok"] == true && reply["outcome"] == "completed",
            "EOS command failed for {path}/{port}: {reply}"
          );
          let duplicate = exchange(&owner_socket, "command", &envelope)?;
          config::atomic_write(
            &root.join(format!("duplicate-{command_count}.json")),
            &serde_json::to_vec_pretty(&duplicate)?,
          )?;
          ensure!(
            duplicate["ok"] == true && duplicate["outcome"] == "completed",
            "EOS did not return the stored result for {path}/{port}"
          );
          let mut reused = envelope.clone();
          reused["payload"] = distinct_valid_payload(&payload)?;
          let reuse_rejected = exchange(&owner_socket, "command", &reused)?;
          config::atomic_write(
            &root.join(format!("reused-id-{command_count}.json")),
            &serde_json::to_vec_pretty(&reuse_rejected)?,
          )?;
          ensure!(
            reuse_rejected["ok"] == false && reuse_rejected["outcome"] == "rejected",
            "EOS reused command ID with changed payload for {path}/{port}"
          );
          let mut invalid = envelope.clone();
          invalid["command_id"] = json!(format!("pkg-check-invalid-{command_count}"));
          invalid["payload"] = json!({"invalid_axis":999});
          let rejected = exchange(&owner_socket, "command", &invalid)?;
          config::atomic_write(
            &root.join(format!("rejected-{command_count}.json")),
            &serde_json::to_vec_pretty(&rejected)?,
          )?;
          ensure!(
            rejected["ok"] == false,
            "EOS accepted invalid command for {path}/{port}"
          );
          command_count += 1;
        }
      }
    }
    ensure!(command_count > 0, "candidate has no actuator commands");
    Ok(())
  })();
  let stop = crate::runtime::manager::owner_deployment::stop(paths, &name);
  if let Ok(stop) = &stop {
    config::atomic_write(&root.join("stop.json"), &serde_json::to_vec_pretty(stop)?)?;
  }
  replay?;
  let stop = stop?;
  ensure!(
    stop["terminal"]["outcome"] == "confirmed",
    "IsOS manager Stop unconfirmed: {stop}"
  );
  for socket in &sockets {
    ensure!(
      !Path::new(socket).exists(),
      "package service socket survived Stop: {socket}"
    );
  }
  config::atomic_write(
    &root.join("socket-cleanup.json"),
    &serde_json::to_vec_pretty(&json!({"removed":true,"services":sockets.len()}))?,
  )?;
  // The owner is persistent across graph replays. Its log directory belongs to
  // the first deployment that started it, so read through the owner endpoint.
  let eos_log = run_log(run_id.as_str(), |cursor| {
    crate::runtime::owners::owner_services::log_page(paths, "eos", cursor)
      .context("EOS manager replay log unavailable")
  })?;
  config::atomic_write(&root.join("eos.jsonl"), &eos_log)?;
  ensure!(
    eos_log
      .windows(run_id.len())
      .any(|part| part == run_id.as_bytes()),
    "EOS replay log lacks run identity"
  );
  let isos_log_path = paths
    .data
    .join("log/deployments")
    .join(&name)
    .join("isos.jsonl");
  if isos_log_path.is_file() {
    let isos_log = fs::read(&isos_log_path)?;
    ensure!(
      isos_log
        .windows(run_id.len())
        .any(|part| part == run_id.as_bytes()),
      "package service log lacks run identity"
    );
    config::atomic_write(&root.join("isos.jsonl"), &isos_log)?;
  }
  child_failure_probe(paths, release, &graph, &revision, &verified, &root, &name)?;
  Ok(())
}

fn child_failure_probe(
  paths: &Paths,
  release: &Release,
  graph: &Value,
  revision: &str,
  verified: &super::VerifiedPackage,
  root: &Path,
  source_name: &str,
) -> Result<()> {
  let name = format!("fail_{}", &revision[..12]);
  let run_id = "pkg_check_failure";
  let source = super::package_config::show(paths, source_name, &release.id, &release.version)?.toml;
  let initial = super::package_config::show(paths, &name, &release.id, &release.version)?;
  super::package_config::save(
    paths,
    &name,
    &release.id,
    &release.version,
    &initial.revision,
    &source,
  )?;
  crate::runtime::manager::package_services::start_candidate_services(
    paths,
    &name,
    graph,
    run_id,
    revision,
    std::slice::from_ref(verified),
  )?;
  let ready = crate::runtime::manager::package_services::status(paths, &name, run_id, revision)?;
  ensure!(
    ready["ready"] == true,
    "failure probe child was not initially ready"
  );
  let receipt_path = paths.runtime.join(format!("{name}.package-services.json"));
  let receipt: Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
  ensure!(
    receipt["run_id"] == run_id && receipt["revision"] == revision,
    "failure probe receipt fence changed"
  );
  let child = receipt["children"]
    .as_array()
    .and_then(|children| children.first())
    .context("failure probe has no child")?;
  let pid = child["pid"].as_u64().context("failure probe has no PID")?;
  let ticks = child["start_ticks"]
    .as_u64()
    .context("failure probe has no process identity")?;
  let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
  let actual: u64 = stat
    .rsplit_once(") ")
    .context("invalid child process identity")?
    .1
    .split_whitespace()
    .nth(19)
    .context("missing child start ticks")?
    .parse()?;
  ensure!(actual == ticks, "failure probe process identity changed");
  ensure!(
    unsafe { libc::kill(pid as i32, libc::SIGKILL) } == 0,
    "failure probe could not terminate owned child"
  );
  let deadline = Instant::now() + Duration::from_secs(3);
  let status = loop {
    let status = crate::runtime::manager::package_services::status(paths, &name, run_id, revision)?;
    if status["ready"] == false {
      break status;
    }
    ensure!(
      Instant::now() < deadline,
      "package child failure was not detected"
    );
    thread::sleep(Duration::from_millis(30));
  };
  config::atomic_write(
    &root.join("child-failure.json"),
    &serde_json::to_vec_pretty(&status)?,
  )?;
  let stop = crate::runtime::manager::package_services::stop_candidate_services(
    paths, &name, run_id, revision,
  );
  ensure!(
    stop.is_err(),
    "failed package child incorrectly reported confirmed Stop"
  );
  config::atomic_write(
    &root.join("child-failure-stop.json"),
    &serde_json::to_vec_pretty(
      &json!({"stop_confirmed":false,"error":format!("{:#}",stop.unwrap_err())}),
    )?,
  )?;
  Ok(())
}

fn command_cases(node_type: &str, node: &Value) -> Vec<(&'static str, Value)> {
  match node_type {
    "eos.hardware.base.device@v0.2" => vec![(
      "command",
      json!({"frame_id":node["frame_id"],"vx_m_s":0.0,"vy_m_s":0.0,"yaw_rate_rad_s":0.0,"lease_ms":100}),
    )],
    "eos.hardware.manipulator.device@v0.2" => vec![(
      "command",
      json!({"joint_names":node["joint_names"],"position_rad":[0.0,0.0,0.0,0.0,0.0,0.0]}),
    )],
    "eos.hardware.gripper.binary@v0.2" => vec![
      ("command", json!({"action":"open"})),
      ("command", json!({"action":"close"})),
    ],
    "eos.hardware.base.normalized_axes@v0.2" => vec![(
      "command",
      json!({"axes":{"x":0.0,"y":0.0,"yaw":0.0},"lease_ms":100}),
    )],
    "eos.hardware.manipulator.normalized_cartesian@v0.2" => vec![
      (
        "command",
        json!({"axes":{"dx":0.0,"dy":0.0,"dz":0.0,"droll":0.0,"dpitch":0.0,"dyaw":0.0},"lease_ms":100}),
      ),
      ("mode", json!({"mode":"0"})),
      ("mode", json!({"mode":"1"})),
    ],
    "eos.hardware.torso.normalized_axes@v0.2" => {
      vec![("command", json!({"axes":{"lift":0.0},"lease_ms":100}))]
    }
    _ => Vec::new(),
  }
}

fn distinct_valid_payload(original: &Value) -> Result<Value> {
  let mut changed = original.clone();
  if changed.get("vx_m_s").is_some() {
    changed["vx_m_s"] = json!(0.1);
  } else if changed.get("position_rad").is_some() {
    changed["position_rad"][0] = json!(0.1);
  } else if let Some(action) = changed["action"].as_str() {
    changed["action"] = json!(if action == "open" { "close" } else { "open" });
  } else if let Some(mode) = changed["mode"].as_str() {
    changed["mode"] = json!(if mode == "0" { "1" } else { "0" });
  } else if let Some(axes) = changed["axes"].as_object_mut() {
    let key = axes
      .keys()
      .next()
      .cloned()
      .context("axes payload is empty")?;
    axes.insert(key, json!(0.1));
  } else {
    anyhow::bail!("no valid distinct payload for checked command")
  }
  ensure!(
    changed != *original,
    "command replay payload did not change"
  );
  Ok(changed)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn persistent_owner_log_selects_later_run_across_pages() {
    let log = run_log("run-2", |cursor| {
      Ok(match cursor {
        0 => json!({"next_cursor": 100, "entries": [
          {"run_id":"run-1","message":"EOS run ready"}
        ]}),
        100 => json!({"next_cursor": 200, "entries": [
          {"run_id":"run-2","message":"EOS run ready"}
        ]}),
        200 => json!({"next_cursor": 200, "entries": []}),
        _ => unreachable!(),
      })
    })
    .unwrap();
    let text = String::from_utf8(log).unwrap();
    assert!(text.contains("run-2"));
    assert!(!text.contains("run-1"));
  }

  #[test]
  fn command_id_reuse_probe_changes_a_valid_field() {
    let base =
      json!({"frame_id":"base-0","vx_m_s":0.0,"vy_m_s":0.0,"yaw_rate_rad_s":0.0,"lease_ms":100});
    assert_eq!(distinct_valid_payload(&base).unwrap()["vx_m_s"], 0.1);
    let axes = json!({"axes":{"x":0.0,"y":0.0,"yaw":0.0},"lease_ms":100});
    assert_eq!(distinct_valid_payload(&axes).unwrap()["axes"]["x"], 0.1);
    let mode = json!({"mode":"0"});
    assert_eq!(distinct_valid_payload(&mode).unwrap()["mode"], "1");
  }

  #[test]
  fn owner_replay_accepts_observation_reply_over_one_mib() {
    use std::os::unix::net::UnixListener;
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("owner.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut request = Vec::new();
      stream.read_to_end(&mut request).unwrap();
      let body = json!({"ok":true,"payload":"a".repeat(2 * 1024 * 1024)}).to_string();
      write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
      )
      .unwrap();
    });
    let reply = exchange(&socket, "observe", &json!({"op":"observe"})).unwrap();
    assert_eq!(reply["payload"].as_str().unwrap().len(), 2 * 1024 * 1024);
    server.join().unwrap();
  }

  fn release() -> Release {
    serde_json::from_value(json!({
      "contract":"isos.package@v1","schema_version":"0.2.0","id":"vendor/device",
      "kind":"component","version":"0.2.0","source_commit":"a".repeat(40),
      "node_contributions":{},
      "assets":[{"os":"linux","arch":"x86_64","url":"https://github.com/vendor/device/releases/download/v0.2.0/bundle.tar.gz","size_bytes":1,"sha256":"b".repeat(64)}]
    })).unwrap()
  }

  #[test]
  fn v2_candidate_requires_exact_service_preset_devices() {
    let mut release = release();
    release.schema_version = "0.2.1".into();
    release.version = "0.2.1".into();
    release.node_contributions = Some(serde_json::from_value(json!({
      "runtime_services":[{"id":"native","entrypoint":"bin/service","config_service_type":"native","node_types":["eos.hardware.camera.device@v0.2"]}],
      "node_sets":[{"id":"native","parameters":{},"nodes":[
        {"ref":"front","core":"eos","layer":"hardware","group":"camera","profile":{"adapter":{"settings":{"device_id":"cam-0"}}}},
        {"ref":"rear","core":"eos","layer":"hardware","group":"camera","profile":{"adapter":{"settings":{"device_id":"cam-1"}}}}
      ]}]
    })).unwrap());
    let directory = tempfile::tempdir().unwrap();
    let graph = directory.path().join("candidate.json");
    let node = |device| {
      json!({"node_type":"eos.hardware.camera.device@v0.2",
      "adapter":{"package":{"id":"vendor/device","version":"0.2.1"},"settings":{"service_instance":"native","device_id":device}}})
    };
    let mut value = json!({"version":"0.2.1","cores":{"eos":{"layers":{"hardware":{"groups":{"camera":{"nodes":{"front":node("cam-0")}}}}}}}});
    fs::write(&graph, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(selected_service_instances(std::slice::from_ref(&graph), &release).is_err());
    value["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"]["nodes"]["rear"] =
      node("cam-1");
    fs::write(&graph, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(
      selected_service_instances(std::slice::from_ref(&graph), &release).unwrap(),
      ["native"]
    );
    value["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"]["nodes"]["rear"]["adapter"]["settings"]
      ["device_id"] = json!("cam-0");
    fs::write(&graph, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(selected_service_instances(std::slice::from_ref(&graph), &release).is_err());
  }

  #[test]
  fn replay_selects_only_checked_package_process_and_camera() {
    let package = json!({"id":"vendor/device","version":"0.2.0"});
    let graph = json!({"cores":{"eos":{"layers":{
      "process":{"groups":{"sim":{"nodes":{"main":{"node_type":"eos.process.sim.main@v0.2","package":package,"settings":{"scene":"one"}}}}}},
      "hardware":{"groups":{"camera":{"nodes":{"front":{"node_type":"eos.hardware.camera.device@v0.2","adapter":{"package":package}}}}}}
    }}}});
    let selected = selected_process(&graph, &release()).unwrap();
    assert_eq!(selected.0, "eos/process/sim/main");
    assert_eq!(selected.2["scene"], "one");
    assert_eq!(
      observer_nodes(&graph, &release()).unwrap(),
      ["eos/hardware/camera/front"]
    );
    let mut unsupported = graph;
    unsupported["cores"]["eos"]["layers"]["hardware"]["groups"]["base"]["nodes"]["drive"] =
      json!({"node_type":"eos.hardware.base.device@v0.2","adapter":{"package":package}});
    assert!(observer_nodes(&unsupported, &release()).is_err());
  }
}
