//! Bounded, on-demand setup adapters for the saved node graph.
use crate::{app::config::Paths, domain::graph::graph_contract, domain::packages};
#[cfg(test)]
use crate::app::config;
use anyhow::{Context, Result, bail, ensure};
use crate::interfaces::setup::SetupReply;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
  collections::BTreeSet,
  fs,
  io::{Read, Write},
  os::unix::process::CommandExt,
  path::{Component, Path, PathBuf},
  process::{Command, Stdio},
  sync::mpsc,
  thread,
  time::{Duration, Instant},
};

const MAX_REQUEST: usize = 1024 * 1024;
const MAX_REPLY: usize = 4 * 1024 * 1024;
const MAX_FILES: usize = 32;
const MAX_FILE: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
const SETUP_SCHEMA_VERSION: u32 = 2;
pub(crate) const EI_NORTHBOUND_PORT: u16 = 16060;

fn revision(bytes: &[u8]) -> String {
  hex::encode(Sha256::digest(bytes))
}

fn selected_adapter(paths: &Paths, role: &str) -> Result<PathBuf> {
  match role {
    "eos" => crate::runtime::process::core_bins::resolve("aether-profile"),
    "ei" => {
      let _ = paths;
      let directory = PathBuf::from(
        std::env::var_os("ISOS_EI_BIN_DIR")
          .context("configure paths.ei_bin_dir before enabling EI")?,
      );
      ensure!(directory.is_absolute(), "ISOS_EI_BIN_DIR must be absolute");
      let path = directory.join("ei-config");
      ensure!(
        path.is_file(),
        "EI setup adapter missing at {}",
        path.display()
      );
      Ok(path)
    }
    _ => bail!("unknown setup core {role}"),
  }
}

pub fn adapter_path(paths: &Paths, role: &str) -> Result<PathBuf> {
  selected_adapter(paths, role)
}

fn package_refs(paths: &Paths, graph: &Value) -> Result<Vec<Value>> {
  packages::verify_graph_packages(paths, graph)?
    .into_iter()
    .map(|package| {
      Ok(json!({"id":package.release.id,"version":package.release.version,"path":package.store}))
    })
    .collect()
}

fn command_arg(graph: &Value, node: &str, flag: &str) -> Result<Option<String>> {
  let pointer = format!("/cores/isos/layers/process/groups/component/nodes/{node}/settings");
  let Some(settings) = graph.pointer(&pointer) else {
    return Ok(None);
  };
  let args = settings["args"]
    .as_array()
    .with_context(|| format!("{node} process requires settings.args; save the migrated graph"))?;
  ensure!(
    args.iter().all(Value::is_string),
    "{node} settings.args must contain strings"
  );
  let mut values = args.iter().filter_map(Value::as_str);
  let mut selected = None;
  while let Some(arg) = values.next() {
    if arg == flag {
      ensure!(selected.is_none(), "{node} has duplicate {flag}");
      let value = values
        .next()
        .with_context(|| format!("{node} {flag} needs a value"))?;
      ensure!(
        !value.is_empty() && !value.starts_with("--"),
        "{node} {flag} needs a value"
      );
      selected = Some(value.to_owned());
    }
  }
  selected
    .map(Some)
    .with_context(|| format!("{node} process requires {flag}"))
}

fn listener(value: Option<String>, default: u16) -> Result<Value> {
  let Some(value) = value else {
    return Ok(json!({"host":"127.0.0.1","port":default}));
  };
  let (host, port) = value
    .rsplit_once(':')
    .with_context(|| format!("listener {value} must be 127.0.0.1:<port>"))?;
  ensure!(host == "127.0.0.1", "listener {value} must bind 127.0.0.1");
  let port = port
    .parse::<u16>()
    .with_context(|| format!("listener {value} has invalid port"))?;
  ensure!(port >= 1024, "listener {value} port must be 1024–65535");
  Ok(json!({"host":host,"port":port}))
}

fn eos_gateway_port(graph: &Value) -> Result<u16> {
  if graph["cores"].get("eos").is_some() {
    return Ok(16200);
  }
  let pointer = "/cores/isos/layers/process/groups/component/nodes/eos/settings/args";
  let args = graph
    .pointer(pointer)
    .and_then(Value::as_array)
    .context("EOS process requires settings.args")?;
  ensure!(
    args.iter().all(Value::is_string),
    "EOS arguments must be strings"
  );
  let mut port = None;
  let mut index = 0;
  while index < args.len() {
    if args[index] == "--grpc-port" {
      ensure!(port.is_none(), "EOS --grpc-port must occur once");
      let raw = args
        .get(index + 1)
        .and_then(Value::as_str)
        .context("EOS --grpc-port needs a value")?;
      let parsed = raw
        .parse::<u16>()
        .context("EOS --grpc-port must be a TCP port")?;
      ensure!(parsed >= 1024, "EOS --grpc-port must be 1024–65535");
      port = Some(parsed);
      index += 1;
    }
    index += 1;
  }
  Ok(port.unwrap_or(16200))
}

pub(crate) fn endpoints(paths: &Paths, name: &str, graph: &Value) -> Result<Value> {
  let api_port = crate::app::server::local_port()?;
  let eos_socket = command_arg(graph, "eos", "--socket")?
    .map(PathBuf::from)
    .unwrap_or_else(|| paths.owner_runtime("eos").join(format!("{name}-eos.sock")));
  ensure!(
    eos_socket.is_absolute(),
    "EOS --socket must be an absolute path"
  );
  let eos_control = listener(command_arg(graph, "bridge", "--listen")?, 16020)?;
  let eos_catalog = listener(command_arg(graph, "config", "--listen")?, 16021)?;
  let gateway_port = eos_gateway_port(graph)?;
  let ports = [
    api_port,
    eos_control["port"].as_u64().unwrap() as u16,
    eos_catalog["port"].as_u64().unwrap() as u16,
    EI_NORTHBOUND_PORT,
    gateway_port,
  ];
  ensure!(
    ports.into_iter().collect::<BTreeSet<_>>().len() == ports.len(),
    "managed endpoint ports must be unique"
  );
  Ok(json!({
      "isos_api":{"url":format!("http://127.0.0.1:{api_port}")},
      "eos_control":eos_control,
      "eos_catalog":eos_catalog,
      "eos_gateway":{"transport":"tcp","host":"127.0.0.1","port":gateway_port},
      "ei_northbound":listener(None,EI_NORTHBOUND_PORT)?,
      "eos_socket":{"path":eos_socket}
  }))
}

fn request(
  paths: &Paths,
  name: &str,
  role: &str,
  op: &str,
  graph: &Value,
  graph_revision: &str,
) -> Result<Value> {
  request_with_packages(
    paths,
    name,
    role,
    op,
    graph,
    graph_revision,
    package_refs(paths, graph)?,
  )
}

fn request_with_packages(
  paths: &Paths,
  name: &str,
  role: &str,
  op: &str,
  graph: &Value,
  graph_revision: &str,
  packages: Vec<Value>,
) -> Result<Value> {
  ensure!(matches!(role, "eos" | "ei"), "unknown setup core");
  ensure!(
    matches!(op, "catalog" | "options" | "validate" | "render"),
    "unknown setup operation"
  );
  let owner_graph = crate::domain::graph::single_core::owner_projection(graph, role)?;
  let request = json!({"schema_version":SETUP_SCHEMA_VERSION,"op":op,"core":role,"revision":graph_revision,
        "graph":owner_graph,"endpoints":endpoints(paths,name,graph)?,"packages":packages});
  ensure!(
    serde_json::to_vec(&request)?.len() <= MAX_REQUEST,
    "setup request exceeds 1 MiB"
  );
  Ok(request)
}

#[cfg(test)]
fn invoke(program: &Path, request: &Value) -> Result<Value> {
  invoke_with_env(program, request, None)
}

fn invoke_with_env(
  program: &Path,
  request: &Value,
  environment: Option<&std::collections::BTreeMap<String, String>>,
) -> Result<Value> {
  let identity = request["op"] == "identity";
  let reply_limit = if identity { 64 * 1024 } else { MAX_REPLY };
  let timeout = if identity {
    Duration::from_secs(5)
  } else {
    TIMEOUT
  };
  let mut command = Command::new(program);
  command
    .args(["setup", "--json"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  if let Some(environment) = environment {
    command.envs(environment);
  }
  command.env_remove(crate::runtime::process::skill_selection::ENV);
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
    .with_context(|| format!("start setup adapter {}", program.display()))?;
  let group = child.id() as i32;
  let input = serde_json::to_vec(request)?;
  let writer = child.stdin.take().unwrap();
  let (write_tx, write_rx) = mpsc::channel();
  thread::spawn(move || {
    let mut writer = writer;
    let _ = write_tx.send(writer.write_all(&input));
  });
  let stdout = child.stdout.take().unwrap();
  let (out_tx, out_rx) = mpsc::channel();
  thread::spawn(move || {
    let mut data = Vec::new();
    let result = stdout
      .take((reply_limit + 1) as u64)
      .read_to_end(&mut data)
      .map(|_| data);
    let _ = out_tx.send(result);
  });
  let stderr = child.stderr.take().unwrap();
  let (err_tx, err_rx) = mpsc::channel();
  thread::spawn(move || {
    let mut data = Vec::new();
    let result = stderr.take(8193).read_to_end(&mut data).map(|_| data);
    let _ = err_tx.send(result);
  });
  let started = Instant::now();
  let mut status = None;
  let mut written = None;
  let mut output = None;
  let mut stderr = None;
  loop {
    if status.is_none() {
      status = child.try_wait()?;
    }
    if written.is_none() {
      written = write_rx.try_recv().ok();
    }
    if output.is_none() {
      output = out_rx.try_recv().ok();
    }
    if stderr.is_none() {
      stderr = err_rx.try_recv().ok();
    }
    if status.is_some() && written.is_some() && output.is_some() && stderr.is_some() {
      break;
    }
    if started.elapsed() > timeout {
      unsafe { libc::kill(-group, libc::SIGKILL) };
      let _ = child.kill();
      let _ = child.wait();
      bail!("setup adapter timed out");
    }
    thread::sleep(Duration::from_millis(10));
  }
  written.unwrap()?;
  let output = output.unwrap()?;
  let _stderr = stderr.unwrap()?;
  ensure!(
    output.len() <= reply_limit,
    "setup adapter response exceeds limit"
  );
  let status = status.unwrap();
  if !status.success() && status.code() != Some(2) {
    bail!("setup adapter failed; inspect the selected owner's private diagnostics");
  }
  let reply: Value = crate::runtime::process::reader_features::parse_unique_json(&output)
    .context("parse setup adapter response")?;
  validate_reply(request, &reply)?;
  ensure!(
    status.success() || reply["ok"] == false,
    "setup adapter exited with rejection status but reported success"
  );
  Ok(reply)
}

fn validate_reply(request: &Value, reply: &Value) -> Result<()> {
  let typed: SetupReply =
    serde_json::from_value(reply.clone()).context("setup adapter response violates schema 2")?;
  ensure!(
    typed.schema_version == SETUP_SCHEMA_VERSION,
    "setup adapter schema mismatch"
  );
  if !typed.ok {
    ensure!(
      typed.identity.is_none() && typed.catalog.is_none() && typed.files.is_empty(),
      "rejected setup reply must not publish data"
    );
  } else {
    match request["op"].as_str() {
      Some("identity") => ensure!(
        typed.identity.as_ref().is_some_and(Value::is_object)
          && typed.catalog.is_none()
          && typed.files.is_empty(),
        "setup identity reply has invalid fields"
      ),
      Some("catalog" | "options") => ensure!(
        typed.catalog.as_ref().is_some_and(Value::is_object)
          && typed.identity.is_none()
          && typed.files.is_empty(),
        "setup catalog reply has invalid fields"
      ),
      Some("validate") => ensure!(
        typed.identity.is_none() && typed.catalog.is_none() && typed.files.is_empty(),
        "setup validate reply has invalid fields"
      ),
      Some("render") => ensure!(
        typed.identity.is_none() && typed.catalog.is_none(),
        "setup render reply has invalid fields"
      ),
      _ => bail!("unsupported setup operation"),
    }
  }
  ensure!(
    typed
      .diagnostics
      .iter()
      .all(|item| !item.path.is_empty() && !item.code.is_empty() && !item.message.is_empty()),
    "setup adapter diagnostic fields must be nonempty"
  );
  ensure!(
    typed
      .files
      .iter()
      .all(|item| !item.path.is_empty() && item.content.len() <= MAX_FILE),
    "setup adapter file is invalid or exceeds 1 MiB"
  );
  ensure!(
    typed.ok || !typed.diagnostics.is_empty(),
    "setup adapter rejected request without diagnostics"
  );
  Ok(())
}

fn invoke_managed(
  paths: &Paths,
  role: &str,
  request: &Value,
  environment: Option<&std::collections::BTreeMap<String, String>>,
) -> Result<Value> {
  let reply = if paths.socket().exists() {
    let reply = super::owner_services::setup(paths, role, request)?;
    validate_reply(request, &reply)?;
    reply
  } else {
    invoke_with_env(&selected_adapter(paths, role)?, request, environment)?
  };
  if reply["ok"] == false {
    for diagnostic in reply["diagnostics"]
      .as_array()
      .into_iter()
      .flatten()
      .take(32)
    {
      tracing::warn!(
        event_code = "setup_owner_diagnostic",
        owner = role,
        path = diagnostic["path"].as_str().unwrap_or(""),
        code = diagnostic["code"].as_str().unwrap_or(""),
        "Selected owner rejected node graph"
      );
    }
  }
  Ok(reply)
}

fn safe_file(role: &str, entry: &Value) -> Result<(PathBuf, Vec<u8>)> {
  let path = entry["path"].as_str().context("setup file has no path")?;
  ensure!(path.len() <= 256, "setup file path too long");
  let relative = Path::new(path);
  ensure!(
    !relative.is_absolute() && !path.contains('\\'),
    "setup file path must be relative"
  );
  ensure!(
    relative
      .components()
      .all(|part| matches!(part, Component::Normal(_))),
    "setup file path is unsafe"
  );
  ensure!(
    relative
      .components()
      .next()
      .is_some_and(|part| part.as_os_str() == role),
    "setup file must stay under its core directory"
  );
  let content = entry["content"]
    .as_str()
    .context("setup file has no string content")?
    .as_bytes()
    .to_vec();
  ensure!(content.len() <= MAX_FILE, "setup file exceeds 1 MiB");
  Ok((relative.to_path_buf(), content))
}

pub fn call(
  paths: &Paths,
  name: &str,
  role: &str,
  op: &str,
  graph: &Value,
  graph_revision: &str,
) -> Result<Value> {
  let request = request(paths, name, role, op, graph, graph_revision)?;
  let environment = if role == "ei" {
    Some(crate::domain::graph::generated_v2::ei_launch_env(graph)?)
  } else {
    None
  };
  invoke_managed(paths, role, &request, environment.as_ref())
}

/// Preview draft selector options against the selected adapter binary. A
/// persistent owner may still be running an older admitted generation; draft
/// editing does not alter that owner. Apply revalidates with the owner.
pub fn preview_options(
  paths: &Paths,
  name: &str,
  role: &str,
  graph: &Value,
  graph_revision: &str,
) -> Result<Value> {
  let request = request(paths, name, role, "options", graph, graph_revision)?;
  let environment = if role == "ei" {
    Some(crate::domain::graph::generated_v2::ei_launch_env(graph)?)
  } else {
    None
  };
  invoke_with_env(
    &selected_adapter(paths, role)?,
    &request,
    environment.as_ref(),
  )
}

/// Read draft types from the same selected adapter used for option previews.
pub fn preview_catalog(
  paths: &Paths,
  name: &str,
  role: &str,
  graph: &Value,
  graph_revision: &str,
) -> Result<Value> {
  let request = request(paths, name, role, "catalog", graph, graph_revision)?;
  let environment = if role == "ei" {
    Some(crate::domain::graph::generated_v2::ei_launch_env(graph)?)
  } else {
    None
  };
  invoke_with_env(
    &selected_adapter(paths, role)?,
    &request,
    environment.as_ref(),
  )
}

/// The selected catalog already verified package bytes for this request.
/// Pass those exact verified stores to the owner without hashing them again.
pub fn catalog_with_verified_packages(
  paths: &Paths,
  name: &str,
  role: &str,
  graph: &Value,
  graph_revision: &str,
  verified: &[crate::domain::packages::VerifiedPackage],
) -> Result<Value> {
  // This entry point is only for a saved 0.1.0 selected catalog. The caller
  // supplies the exact verified package stores; type admission follows after
  // the owner catalog is merged. Global discovery could admit an unrelated
  // identity or reject a contributed type before its sidecar is loaded.
  ensure!(
    graph_contract::uses_revisioned_generation(graph),
    "selected catalog requires a revisioned graph"
  );
  let packages = verified.iter().map(|package| {
        json!({"id":package.release.id,"version":package.release.version,"path":package.store})
    }).collect();
  let request = request_with_packages(
    paths,
    name,
    role,
    "catalog",
    graph,
    graph_revision,
    packages,
  )?;
  let environment = if role == "ei" {
    Some(crate::domain::graph::generated_v2::ei_launch_env(graph)?)
  } else {
    None
  };
  invoke_managed(paths, role, &request, environment.as_ref())
}

pub fn identity(paths: &Paths, role: &str) -> Result<Value> {
  identity_with_env(paths, role, None)
}

pub fn identity_for_graph(paths: &Paths, role: &str, graph: &Value) -> Result<Value> {
  ensure!(matches!(role, "eos" | "ei"), "unknown setup core {role}");
  let environment = if role == "ei" {
    Some(crate::domain::graph::generated_v2::ei_launch_env(graph)?)
  } else {
    None
  };
  let reply = invoke_managed(
    paths,
    role,
    &json!({"schema_version":SETUP_SCHEMA_VERSION,"op":"identity","core":role}),
    environment.as_ref(),
  )?;
  ensure!(reply["ok"] == true, "setup identity rejected");
  let identity = reply
    .get("identity")
    .cloned()
    .context("identity reply missing identity")?;
  validate_identity(role, &identity)?;
  Ok(identity)
}

fn validate_identity(role: &str, identity: &Value) -> Result<()> {
  let support = crate::domain::version::bundled()?;
  ensure!(
    identity["core"] == role
      && identity["graph_version"]
        .as_str()
        .is_some_and(|value| !value.is_empty())
      && identity["node_type_version"]
        .as_str()
        .is_some_and(|value| !value.is_empty())
      && identity["setup_schema_version"] == SETUP_SCHEMA_VERSION,
    "{role} setup identity is incompatible with IsOS",
  );
  ensure!(
    identity["product_version"] == support.product_version,
    "{role} product version mismatch: expected {}, received {}",
    support.product_version,
    identity["product_version"]
  );
  Ok(())
}

fn identity_with_env(
  paths: &Paths,
  role: &str,
  environment: Option<&std::collections::BTreeMap<String, String>>,
) -> Result<Value> {
  ensure!(matches!(role, "eos" | "ei"), "unknown setup core {role}");
  let reply = invoke_managed(
    paths,
    role,
    &json!({"schema_version":SETUP_SCHEMA_VERSION,"op":"identity","core":role}),
    environment,
  )?;
  ensure!(reply["ok"] == true, "{role} identity rejected");
  let identity = reply
    .get("identity")
    .context("identity reply missing identity")?;
  validate_identity(role, identity)?;
  Ok(identity.clone())
}

pub fn prepare_run(paths: &Paths, name: &str, expected_revision: &str) -> Result<()> {
  let graph_path = paths.node_graph_path(name);
  let saved =
    graph_contract::read(&graph_path).with_context(|| format!("read {}", graph_path.display()))?;
  ensure!(
    saved.revision == expected_revision,
    "node graph revision changed during setup"
  );
  let graph = saved.value;
  ensure!(
    !graph
      .pointer("/cores/isos/layers")
      .and_then(Value::as_object)
      .is_some_and(serde_json::Map::is_empty),
    "saved graph is empty; add nodes and Apply before Run"
  );
  let proposal = crate::domain::graph::graph_apply::Proposal {
    graph: graph.clone(),
    bytes: Vec::new(),
    revision: expected_revision.to_owned(),
  };
  let prepared = crate::domain::graph::graph_apply::prepare(paths, name, &proposal)
    .map_err(anyhow::Error::new)?;
  verify_rendered_snapshot(paths, name, &graph, expected_revision, &prepared.files)?;
  verify_launch_inputs(paths, name)?;
  Ok(())
}

/// Revalidate a previously applied graph against the currently configured cores.
/// A fresh draft has no launch inputs and remains unapplied.
pub fn refresh_after_init(paths: &Paths, name: &str) -> Result<Option<String>> {
  let graph_path = paths.node_graph_path(name);
  let saved =
    graph_contract::read(&graph_path).with_context(|| format!("read {}", graph_path.display()))?;
  if !snapshot_root(paths, name, &saved.revision).exists()
    && !paths.generated(name).join("selection.json").exists()
    && !paths
      .generated(name)
      .join("v2/required")
      .join(format!("{}.json", saved.revision))
      .exists()
  {
    return Ok(None);
  }
  verify_launch_inputs(paths, name).map_err(|error| {
            anyhow::anyhow!(
                "generated inputs are stale or invalid: {error:#}; restore the selected inputs or Apply the same saved revision again; `isos init` keeps the generation immutable"
            )
        })?;
  Ok(None)
}

pub fn render_graph(
  paths: &Paths,
  name: &str,
  graph: &Value,
  expected_revision: &str,
) -> Result<Vec<(PathBuf, Vec<u8>)>> {
  let ei_process = ei_owner_selected(graph);
  let eos_selected = graph["cores"].get("eos").is_some();
  let roles = ["eos", "ei"]
    .into_iter()
    .filter(|role| *role == "eos" && eos_selected || *role == "ei" && ei_process);
  let mut files = Vec::new();
  let mut seen_files = BTreeSet::new();
  for role in roles {
    let reply = call(paths, name, role, "render", graph, expected_revision).map_err(|error| {
      if graph_contract::uses_revisioned_generation(graph) {
        anyhow::Error::new(crate::domain::graph::generated_v2::owner_rejected(role))
      } else {
        error
      }
    })?;
    if graph_contract::uses_revisioned_generation(graph) && reply["ok"] != true {
      return Err(crate::domain::graph::generated_v2::owner_rejected(role).into());
    }
    ensure!(
      reply["ok"] == true,
      "{role} setup rejected graph: {}",
      reply["diagnostics"]
    );
    let entries = reply["files"].as_array().unwrap();
    ensure!(
      entries.len() <= MAX_FILES,
      "setup adapter returned too many files"
    );
    for entry in entries {
      let directory = role;
      let file = safe_file(directory, entry).map_err(|error| {
        if graph_contract::uses_revisioned_generation(graph) {
          anyhow::Error::new(crate::domain::graph::generated_v2::unsafe_render())
        } else {
          error
        }
      })?;
      ensure!(
        seen_files.insert(file.0.clone()),
        "duplicate setup file path"
      );
      files.push(file);
    }
  }
  let mut inputs = Vec::new();
  if eos_selected {
    for binary in ["aether-profile", "aether-runtime"] {
      let path = crate::runtime::process::core_bins::resolve(binary)?;
      inputs.push(
        json!({"core":"eos","binary":binary,"path":path,"sha256":revision(&fs::read(&path)?)}),
      );
    }
  }
  if ei_process {
    for binary in ["ei-config", "ei"] {
      let path = selected_adapter(paths, "ei")?.with_file_name(binary);
      inputs.push(
        json!({"core":"ei","binary":binary,"path":path,"sha256":revision(&fs::read(&path)?)}),
      );
    }
  }
  files.push((
    PathBuf::from("launch-inputs.json"),
    serde_json::to_vec_pretty(&inputs)?,
  ));
  Ok(files)
}

fn ei_owner_selected(graph: &Value) -> bool {
  graph["cores"].get("ei").is_some()
}

pub fn verify_launch_inputs(paths: &Paths, name: &str) -> Result<()> {
  let graph_path = paths.node_graph_path(name);
  let saved = graph_contract::read(&graph_path)?;
  let file = if graph_contract::uses_revisioned_generation(&saved.value) {
    verify_revisioned(paths, name, &saved.value, &saved.revision)?;
    crate::domain::graph::generated_v2::selected_root(paths, name, &saved.value, &saved.revision)?
      .join("core-setup/launch-inputs.json")
  } else {
    paths.generated(name).join("core-setup/launch-inputs.json")
  };
  let inputs: Value = serde_json::from_slice(&fs::read(&file).with_context(|| {
    format!(
      "saved launch inputs are missing at {}; save the graph",
      file.display()
    )
  })?)?;
  let remedy = if graph_contract::uses_revisioned_generation(&saved.value) {
    "restore the pinned binary or create a new graph revision and Apply"
  } else {
    "run `isos init`, then restart"
  };
  for input in inputs.as_array().context("invalid saved launch inputs")? {
    let path = Path::new(
      input["path"]
        .as_str()
        .context("launch input path is missing")?,
    );
    ensure!(path.is_absolute(), "launch input path must be absolute");
    let configured = match input["core"].as_str() {
      Some("eos") => crate::runtime::process::core_bins::resolve(
        input["binary"]
          .as_str()
          .context("launch input binary is missing")?,
      )?,
      Some("ei") => selected_adapter(paths, "ei")?.with_file_name(
        input["binary"]
          .as_str()
          .context("EI launch input binary missing")?,
      ),
      _ => bail!("unknown launch input core"),
    };
    ensure!(
      path == configured,
      "configured path for {} changed since graph Apply; {remedy}",
      input["binary"]
    );
    let current = revision(&fs::read(path).with_context(|| {
      format!(
        "configured file {} is missing; save the graph after rebuilding",
        path.display()
      )
    })?);
    ensure!(
      input["sha256"] == current,
      "configured file {} changed since graph Apply; {remedy}",
      path.display()
    );
  }
  Ok(())
}

pub fn snapshot_root(paths: &Paths, name: &str, graph_revision: &str) -> PathBuf {
  paths.generated(name).join(graph_revision)
}

pub fn write_revisioned(
  paths: &Paths,
  name: &str,
  graph: &Value,
  graph_revision: &str,
  files: Vec<(PathBuf, Vec<u8>)>,
) -> Result<PathBuf> {
  crate::domain::graph::generated_v2::publish(paths, name, graph, graph_revision, &files)
    .map_err(anyhow::Error::new)
}

pub fn verify_revisioned(
  paths: &Paths,
  name: &str,
  graph: &Value,
  graph_revision: &str,
) -> Result<()> {
  if crate::domain::graph::generated_v2::selected(paths, name, graph, graph_revision, None)?
    .is_some()
  {
    return Ok(());
  }
  let root = snapshot_root(paths, name, graph_revision);
  let manifest: Value = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
  ensure!(
    manifest["schema_version"] == 1
      && manifest["source_revision"] == graph_revision
      && manifest["graph_sha256"] == graph_revision,
    "generated manifest differs from saved graph revision"
  );
  let entries = manifest["files"]
    .as_array()
    .context("generated manifest lacks files")?;
  for entry in entries {
    let relative = Path::new(
      entry["path"]
        .as_str()
        .context("generated file path missing")?,
    );
    ensure!(
      relative
        .components()
        .all(|part| matches!(part, Component::Normal(_))),
      "generated manifest has unsafe file path"
    );
    let file = root.join(relative);
    ensure!(
      fs::symlink_metadata(&file)?.file_type().is_file(),
      "generated file is not regular: {}",
      file.display()
    );
    ensure!(
      entry["sha256"] == revision(&fs::read(&file)?),
      "generated file digest changed: {}",
      file.display()
    );
  }
  for required in ["eos/graph.json", "core-setup/launch-inputs.json"] {
    ensure!(
      entries.iter().any(|entry| entry["path"] == required),
      "generated manifest lacks {required}"
    );
  }
  let packages = package_refs(paths, graph)?;
  let recorded = manifest["packages"]
    .as_array()
    .context("generated manifest lacks packages")?;
  ensure!(
    packages.len() == recorded.len(),
    "generated package set changed"
  );
  for (package, saved) in packages.iter().zip(recorded) {
    let receipt = Path::new(package["path"].as_str().unwrap()).join("receipt.json");
    ensure!(
      saved["id"] == package["id"]
        && saved["version"] == package["version"]
        && saved["sha256"] == revision(&fs::read(receipt)?),
      "generated package receipt changed"
    );
  }
  Ok(())
}

pub fn verify_rendered_snapshot(
  paths: &Paths,
  name: &str,
  graph: &Value,
  graph_revision: &str,
  expected: &[(PathBuf, Vec<u8>)],
) -> Result<()> {
  if crate::domain::graph::generated_v2::selected(
    paths,
    name,
    graph,
    graph_revision,
    Some(expected),
  )?
  .is_some()
  {
    return Ok(());
  }
  verify_revisioned(paths, name, graph, graph_revision)?;
  let root = snapshot_root(paths, name, graph_revision);
  let manifest: Value = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
  let entries = manifest["files"]
    .as_array()
    .context("generated manifest lacks files")?;
  ensure!(
    entries.len() == expected.len(),
    "generated file set differs from owner render"
  );
  for (relative, content) in expected {
    let relative = if relative == Path::new("launch-inputs.json") {
      Path::new("core-setup/launch-inputs.json")
    } else {
      relative.as_path()
    };
    let path = relative.to_string_lossy();
    ensure!(
      entries.iter().any(|entry| entry["path"] == path.as_ref()),
      "generated manifest lacks owner-rendered file {}",
      relative.display()
    );
    ensure!(
      fs::read(root.join(relative))? == *content,
      "generated file differs from owner render: {}",
      relative.display()
    );
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::PermissionsExt;

  #[test]
  fn draft_options_use_selected_adapter_with_manager_socket_present() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    paths.prepare().unwrap();
    fs::write(paths.socket(), b"stale manager socket placeholder").unwrap();
    let graph: Value = serde_json::from_slice(
      &fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/graph-pool/demo.nodes.json"),
      )
      .unwrap(),
    )
    .unwrap();
    let revision =
      crate::domain::graph::graph_contract::revision(&serde_json::to_vec(&graph).unwrap());
    let reply = preview_options(&paths, "default", "eos", &graph, &revision).unwrap();
    assert_eq!(reply["ok"], true);
    assert!(reply["catalog"]["setting_options"].is_array());
  }

  #[test]
  fn pooled_demo_options_use_one_selected_adapter_generation() {
    if std::env::var_os("ISOS_EI_BIN_DIR").is_none() {
      return;
    }
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    paths.prepare().unwrap();
    fs::write(paths.socket(), b"stale manager socket placeholder").unwrap();
    let graph: Value = serde_json::from_slice(
      &fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/graph-pool/demo.nodes.json"),
      )
      .unwrap(),
    )
    .unwrap();
    let options = crate::domain::graph::node_options::for_graph(&paths, &graph).unwrap();
    assert!(options["setting_options"].is_array());
  }

  #[test]
  fn setup_identity_requires_exact_supported_owner_version() {
    let mut identity = json!({
      "core": "eos",
      "graph_version": "0.2.0",
      "node_type_version": "0.2",
      "setup_schema_version": SETUP_SCHEMA_VERSION,
      "product_version": "0.2.1"
    });
    assert!(validate_identity("eos", &identity).is_ok());
    identity["product_version"] = json!("0.2.0");
    assert!(
      validate_identity("eos", &identity)
        .unwrap_err()
        .to_string()
        .contains("expected 0.2.1")
    );
    identity["product_version"] = json!("0.2.1");
    assert!(validate_identity("ei", &identity).is_err());
  }

  #[test]
  fn owner_projection_keeps_only_its_core_layers() {
    let graph = json!({"version":"0.2.0","robot":{"id":"r","name":"R"},"cores":{
      "isos":{"layers":{"ui":{"groups":{}}}},
      "ei":{"layers":{"io":{"groups":{}},"cognition":{"groups":{}},"governance":{"groups":{}}}},
      "eos":{"layers":{"interface":{"groups":{}},"dispatcher":{"groups":{}},"hardware":{"groups":{}}}}
    }});
    let ei = crate::domain::graph::single_core::owner_projection(&graph, "ei").unwrap();
    let eos = crate::domain::graph::single_core::owner_projection(&graph, "eos").unwrap();
    assert!(ei.pointer("/cores/ei/layers/io").is_some());
    assert!(ei.pointer("/cores/eos").is_none());
    assert!(eos.pointer("/cores/eos/layers/hardware").is_some());
    assert!(eos.pointer("/cores/ei").is_none());
  }

  #[test]
  fn health_only_three_core_graph_selects_ei_and_default_owner_endpoints() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let graph = json!({"version":"0.2.0","robot":{"id":"r","name":"R"},"cores":{
      "isos":{"layers":{"ui":{"groups":{}}}},
      "ei":{"layers":{"io":{"groups":{}}}},
      "eos":{"layers":{"hardware":{"groups":{}}}}
    }});
    assert!(ei_owner_selected(&graph));
    let resolved = endpoints(&paths, "default", &graph).unwrap();
    assert_eq!(
      resolved["eos_gateway"],
      json!({"transport":"tcp","host":"127.0.0.1","port":16200})
    );
    assert_eq!(resolved["ei_northbound"]["port"], EI_NORTHBOUND_PORT);
    let mut absent = graph.clone();
    absent["cores"].as_object_mut().unwrap().remove("ei");
    assert!(!ei_owner_selected(&absent));
  }

  fn paths(root: &Path) -> Paths {
    Paths {
      config: root.join("config"),
      data: root.join("data"),
      runtime: root.join("run"),
    }
  }

  #[test]
  fn rejects_escaping_adapter_paths() {
    for path in ["../eos/x", "/tmp/x", "ei/x", "eos/../x", "eos\\x"] {
      assert!(
        safe_file("eos", &json!({"path":path,"content":"x"})).is_err(),
        "{path}"
      );
    }
    assert_eq!(
      safe_file("eos", &json!({"path":"eos/graph.json","content":"{}"}))
        .unwrap()
        .1,
      b"{}"
    );
  }

  #[test]
  fn adapter_reply_is_bounded_and_versioned() {
    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("adapter");
    fs::write(&script, "#!/bin/sh\ncat >/dev/null\nprintf '{\"schema_version\":2,\"ok\":true,\"diagnostics\":[],\"files\":[]}'\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let reply = invoke(&script, &json!({"schema_version":2,"op":"render"})).unwrap();
    assert_eq!(reply["ok"], true);
    fs::write(&script, "#!/bin/sh\ncat >/dev/null\nprintf '{\"schema_version\":1,\"ok\":true,\"diagnostics\":[],\"files\":[]}'\n").unwrap();
    assert!(invoke(&script, &json!({"schema_version":2,"op":"render"})).is_err());
    fs::write(&script, "#!/bin/sh\ncat >/dev/null\nprintf '{\"schema_version\":2,\"ok\":false,\"diagnostics\":[],\"files\":[]}'\n").unwrap();
    assert!(invoke(&script, &json!({"schema_version":2,"op":"render"})).is_err());
    fs::write(&script, "#!/bin/sh\ncat >/dev/null\nprintf '{\"schema_version\":2,\"ok\":true,\"diagnostics\":[],\"files\":[{\"path\":\"eos/graph.json\"}]}'\n").unwrap();
    assert!(invoke(&script, &json!({"schema_version":2,"op":"render"})).is_err());
    fs::write(&script, "#!/bin/sh\ncat >/dev/null\nprintf '{\"schema_version\":2,\"ok\":true,\"diagnostics\":[],\"files\":[],\"unexpected\":true}'\n").unwrap();
    assert!(invoke(&script, &json!({"schema_version":2,"op":"render"})).is_err());
    fs::write(&script, "#!/bin/sh\ncat >/dev/null\nprintf '{\"schema_version\":2,\"ok\":true,\"diagnostics\":[],\"files\":[]}'\n").unwrap();
    assert!(invoke(&script, &json!({"schema_version":2,"op":"identity"})).is_err());
    fs::write(&script, "#!/bin/sh\ncat >/dev/null\nprintf '{\"schema_version\":2,\"ok\":false,\"diagnostics\":[{\"path\":\"/graph/cores/isos/layers/ei_io/groups/link/nodes/gateway/node_type\",\"code\":\"unsupported_node_type_version\",\"message\":\"unsupported type\"}],\"files\":[]}'\nexit 2\n").unwrap();
    let rejected = invoke(&script, &json!({"schema_version":2,"op":"render"})).unwrap();
    assert_eq!(rejected["ok"], false);
    assert_eq!(
      rejected["diagnostics"][0]["code"],
      "unsupported_node_type_version"
    );
    fs::write(&script, "#!/bin/sh\ncat >/dev/null\nprintf '{\"schema_version\":2,\"ok\":true,\"diagnostics\":[],\"files\":[]}'\nexit 2\n").unwrap();
    assert!(invoke(&script, &json!({"schema_version":2,"op":"render"})).is_err());
  }

  #[test]
  fn setup_adapter_does_not_receive_skill_selection_handoff() {
    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("adapter");
    fs::write(&script, "#!/bin/sh\ncat >/dev/null\nif [ -n \"${EOS_SKILL_SELECTION_FILE+x}\" ]; then exit 77; fi\nprintf '{\"schema_version\":2,\"ok\":true,\"diagnostics\":[],\"files\":[]}'\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let mut environment = std::collections::BTreeMap::new();
    environment.insert(
      crate::runtime::process::skill_selection::ENV.to_string(),
      "/tmp/forgery".to_string(),
    );
    assert_eq!(
      invoke_with_env(
        &script,
        &json!({"schema_version":2,"op":"render"}),
        Some(&environment)
      )
      .unwrap()["ok"],
      true
    );
  }

  #[test]
  fn explicit_endpoints_reject_missing_malformed_and_duplicate_flags() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let mut graph = json!({"cores":{"isos":{"layers":{"process":{"groups":{"component":{"nodes":{
        "eos":{"settings":{"args":["--socket", "/tmp/eos.sock"]}},
        "bridge":{"settings":{"args":["--listen", "127.0.0.1:16020"]}}
    }}}}}}}});
    let resolved = endpoints(&paths, "default", &graph).unwrap();
    assert_eq!(resolved["eos_control"]["port"], 16020);
    assert_eq!(
      resolved["eos_gateway"],
      json!({"transport":"tcp","host":"127.0.0.1","port":16200})
    );
    assert_eq!(resolved["ei_northbound"]["port"], EI_NORTHBOUND_PORT);
    let eos_args = &mut graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]
      ["eos"]["settings"]["args"];
    *eos_args = json!(["--socket", "/tmp/eos.sock", "--grpc-port", "16210"]);
    assert_eq!(
      endpoints(&paths, "default", &graph).unwrap()["eos_gateway"]["port"],
      16210
    );
    for invalid in [
      json!(["--socket", "/tmp/eos.sock", "--grpc-port"]),
      json!(["--socket", "/tmp/eos.sock", "--grpc-port", "invalid"]),
      json!(["--socket", "/tmp/eos.sock", "--grpc-port", "0"]),
      json!(["--socket", "/tmp/eos.sock", "--grpc-port", "16020"]),
      json!([
        "--socket",
        "/tmp/eos.sock",
        "--grpc-port",
        "16210",
        "--grpc-port",
        "16211"
      ]),
    ] {
      graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["eos"]["settings"]
        ["args"] = invalid;
      assert!(endpoints(&paths, "default", &graph).is_err());
    }
    graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["eos"]["settings"]
      ["args"] = json!(["--socket", "/tmp/eos.sock"]);
    graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["bridge"]["settings"]
      ["args"] = json!(["--listen", "bad"]);
    assert!(endpoints(&paths, "default", &graph).is_err());
    graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["bridge"]["settings"]
      ["args"] = json!(["--listen", "127.0.0.1:16020", "--listen", "127.0.0.1:16021"]);
    assert!(endpoints(&paths, "default", &graph).is_err());
    graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["bridge"]["settings"]
      ["args"] = json!([]);
    assert!(endpoints(&paths, "default", &graph).is_err());
  }

  #[test]
  fn changed_graph_revision_aborts_without_output() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    paths.prepare().unwrap();
    config::atomic_write(
      &paths.node_graph_path("default"),
      br#"{"version":"0.2.0","robot":{},"cores":{"isos":{"layers":{}}}}"#,
    )
    .unwrap();
    assert!(prepare_run(&paths, "default", "stale").is_err());
    assert!(!paths.generated("default").exists());
    let revision =
      crate::domain::graph::starter::revision(&paths.node_graph_path("default")).unwrap();
    let error = prepare_run(&paths, "default", &revision).unwrap_err();
    assert!(
      error.to_string().contains("saved graph is empty"),
      "{error}"
    );
    assert!(!paths.generated("default").exists());
  }

  #[test]
  fn paired_generated_file_and_manifest_tamper_cannot_replace_owner_render() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    paths.prepare().unwrap();
    let graph = json!({"version":"0.1.0","robot":{},"cores":{}});
    let graph_bytes = serde_json::to_vec_pretty(&graph).unwrap();
    let graph_revision = revision(&graph_bytes);
    config::atomic_write(&paths.node_graph_path("default"), &graph_bytes).unwrap();
    let expected = vec![
      (
        PathBuf::from("eos/graph.json"),
        b"original projection".to_vec(),
      ),
      (PathBuf::from("launch-inputs.json"), b"[]".to_vec()),
    ];
    let root =
      write_revisioned(&paths, "default", &graph, &graph_revision, expected.clone()).unwrap();
    verify_rendered_snapshot(&paths, "default", &graph, &graph_revision, &expected).unwrap();
    fs::write(root.join("eos/graph.json"), b"changed projection").unwrap();
    let manifest_path = root.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    let entry = manifest["files"]
      .as_array_mut()
      .unwrap()
      .iter_mut()
      .find(|entry| entry["path"] == "eos/graph.json")
      .unwrap();
    entry["sha256"] = json!(revision(b"changed projection"));
    fs::write(
      &manifest_path,
      serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    assert!(
      verify_revisioned(&paths, "default", &graph, &graph_revision)
        .unwrap_err()
        .to_string()
        .contains("generated_render_mismatch")
    );
    assert!(
      verify_rendered_snapshot(&paths, "default", &graph, &graph_revision, &expected)
        .unwrap_err()
        .to_string()
        .contains("generated_render_mismatch")
    );
  }
}
