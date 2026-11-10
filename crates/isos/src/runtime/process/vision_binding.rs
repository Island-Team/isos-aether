//! IsOS-owned, selected camera evidence service binding outside graph edges.
#![expect(dead_code, reason = "test-only vision binding regression harness")]
use crate::{
  app::config, domain::graph::edge_admission, domain::graph::graph_contract,
  runtime::process::ProcessSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
  collections::BTreeSet,
  fs,
  io::{BufRead, BufReader, Read, Write},
  os::unix::net::UnixStream,
  path::Path,
  time::Duration,
};

pub const TEMPLATE_FILE: &str = "isos/vision-template.json";
const INGRESS: &str = "ei.vision.ingress@v0.1.frame";

#[cfg(test)]
thread_local! {
    static SELECTED_TEMPLATE_DELAY_MS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn test_selected_template_delay(ms: u64) {
  SELECTED_TEMPLATE_DELAY_MS.with(|delay| delay.set(ms));
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
  pub code: &'static str,
  pub stage: &'static str,
  pub path: String,
  pub message: &'static str,
}
impl std::fmt::Display for Diagnostic {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}: {}", self.code, self.path)
  }
}
impl std::error::Error for Diagnostic {}
fn fail(code: &'static str, path: impl Into<String>, message: &'static str) -> Diagnostic {
  Diagnostic {
    code,
    stage: "selection",
    path: path.into(),
    message,
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Template {
  pub schema_version: u32,
  pub graph_revision: String,
  pub projection_revision: String,
  pub source_path: String,
  pub gate_path: String,
  pub ingress_type: String,
  pub render: Render,
  pub target_fps: u32,
  pub endpoint_kind: String,
  pub endpoint_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Render {
  pub max_width_px: u32,
  pub jpeg_quality: u32,
}

fn valid_path(path: &str) -> bool {
  path.split('/').count() == 4
    && path.split('/').all(|part| {
      !part.is_empty()
        && part.len() <= 48
        && part
          .bytes()
          .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    })
}

pub fn selected_gate() -> Result<Option<(String, String)>, Diagnostic> {
  let (gate, fingerprint) = config::vision_gate_selection().map_err(|_| {
    fail(
      "vision_binding_changed",
      "/vision_evidence/gate_path",
      "Read the selected IsOS configuration",
    )
  })?;
  let Some(gate) = gate else { return Ok(None) };
  if !valid_path(&gate)
    || !(gate.starts_with("eos/interface/gate/") || gate.starts_with("isos/eos_interface/gate/"))
  {
    return Err(fail(
      "vision_gate_unknown",
      "/vision_evidence/gate_path",
      "Select a canonical EOS gate",
    ));
  }
  Ok(Some((gate, fingerprint)))
}

pub fn selected_gate_for_graph(graph: &Value) -> Result<Option<(String, String)>, Diagnostic> {
  if let Some(selected) = selected_gate()? {
    return Ok(Some(selected));
  }
  if graph["version"] != "0.2.0" {
    return Ok(None);
  }
  // A camera gate may serve EOS consumers without an EI ingress. Only
  // auto-select the vision handoff when the graph declares that core.
  if !graph
    .pointer("/cores/isos/layers")
    .and_then(Value::as_object)
    .is_some_and(|layers| layers.keys().any(|name| name.starts_with("ei_")))
  {
    return Ok(None);
  }
  let gate_nodes = "/cores/isos/layers/eos_interface/groups/gate/nodes";
  let Some(nodes) = graph.pointer(gate_nodes).and_then(Value::as_object) else {
    return Ok(None);
  };
  let gates: Vec<_> = nodes
    .iter()
    .filter(|(_, node)| node["node_type"] == "isos.eos_interface.camera_gate@v0.2")
    .collect();
  if gates.len() > 1 {
    return Err(fail(
      "vision_source_ambiguous",
      "/vision_evidence/gate_path",
      "Select one camera gate in IsOS configuration",
    ));
  }
  if gates.is_empty() {
    return Ok(None);
  }
  Ok(Some((
    format!("isos/eos_interface/gate/{}", gates[0].0),
    String::new(),
  )))
}

fn node<'a>(graph: &'a Value, path: &str) -> Option<&'a Value> {
  let parts: Vec<_> = path.split('/').collect();
  if parts.len() != 4 {
    return None;
  }
  graph.pointer(&format!(
    "/cores/{}/layers/{}/groups/{}/nodes/{}",
    parts[0], parts[1], parts[2], parts[3]
  ))
}

pub fn select(
  graph: &Value,
  catalog: &Value,
  specs: &[ProcessSpec],
  graph_revision: &str,
  projection_bytes: &[u8],
) -> Result<Option<Template>, Diagnostic> {
  select_with_gate(
    graph,
    catalog,
    specs,
    graph_revision,
    projection_bytes,
    selected_gate_for_graph(graph)?,
  )
}

fn select_with_gate(
  graph: &Value,
  catalog: &Value,
  specs: &[ProcessSpec],
  graph_revision: &str,
  projection_bytes: &[u8],
  selected: Option<(String, String)>,
) -> Result<Option<Template>, Diagnostic> {
  let Some((gate, _source_config_sha256)) = selected else {
    return Ok(None);
  };
  if graph["version"] != "0.2.0" {
    return Err(fail(
      "vision_graph_stale",
      "/version",
      "Vision evidence requires an admitted graph",
    ));
  }
  let gate_node = node(graph, &gate).ok_or_else(|| {
    fail(
      "vision_gate_unknown",
      "/vision_evidence/gate_path",
      "Selected gate is absent",
    )
  })?;
  if gate_node["node_type"] != "isos.eos_interface.camera_gate@v0.2" {
    return Err(fail(
      "vision_gate_unknown",
      "/vision_evidence/gate_path",
      "Selected node is not a camera gate",
    ));
  }
  let source = gate_node
    .pointer("/inputs/frame/node")
    .and_then(Value::as_str)
    .ok_or_else(|| {
      fail(
        "vision_source_missing",
        format!("{gate}/inputs/frame"),
        "Gate has no camera source",
      )
    })?;
  let source_node = node(graph, source).ok_or_else(|| {
    fail(
      "vision_source_missing",
      format!("{gate}/inputs/frame/node"),
      "Camera source is absent",
    )
  })?;
  if source_node["node_type"] != "isos.eos_hardware.camera@v0.2"
    || gate_node["inputs"]["frame"]["output"] != "frame"
  {
    return Err(fail(
      "vision_source_missing",
      format!("{gate}/inputs/frame"),
      "Source must be a registered camera frame",
    ));
  }
  let edges = edge_admission::admit(graph, catalog).map_err(|_| {
    fail(
      "vision_source_missing",
      "/vision_evidence/gate_path",
      "Camera edge admission failed",
    )
  })?;
  let matches = edges
    .iter()
    .filter(|edge| {
      edge.routed
        && edge.source == source
        && edge.target == gate
        && edge.input == "frame"
        && edge.data_type == "camera_frame"
        && edge.transport == "in_process"
    })
    .count();
  if matches != 1 {
    return Err(fail(
      if matches == 0 {
        "vision_source_missing"
      } else {
        "vision_source_ambiguous"
      },
      "/vision_evidence/gate_path",
      "Selection must resolve to one admitted camera edge",
    ));
  }
  let expected = [("eos", "eos"), ("bridge", "eos"), ("ei", "ei")];
  for (id, role) in expected {
    if specs
      .iter()
      .filter(|spec| spec.id == id && spec.settings.role.as_deref() == Some(role))
      .count()
      != 1
    {
      return Err(fail(
        "vision_ingress_unsupported",
        format!("/cores/isos/layers/process/groups/component/nodes/{id}/node_type"),
        "Vision requires selected EOS, bridge and EI children",
      ));
    }
  }
  let ids: BTreeSet<_> = specs.iter().map(|spec| spec.id.as_str()).collect();
  if ids.len() != specs.len() {
    return Err(fail(
      "vision_target_duplicate",
      "/vision_evidence/gate_path",
      "Duplicate selected process target",
    ));
  }
  for (id, dependency) in [("bridge", "eos"), ("ei", "bridge")] {
    let spec = specs.iter().find(|spec| spec.id == id).unwrap();
    if !spec
      .settings
      .depends_on
      .iter()
      .any(|entry| entry.rsplit('/').next() == Some(dependency))
    {
      return Err(fail(
        "vision_ingress_unsupported",
        format!("/cores/isos/layers/process/groups/component/nodes/{id}/settings/depends_on"),
        "Author the required process dependency",
      ));
    }
  }
  let eos = specs.iter().find(|spec| spec.id == "eos").unwrap();
  let bridge = specs.iter().find(|spec| spec.id == "bridge").unwrap();
  let eos_socket = arg(eos, "--socket");
  let bridge_socket = arg(bridge, "--socket");
  if eos_socket.is_none()
    || eos_socket != bridge_socket
    || !eos_socket.is_some_and(|socket| Path::new(socket).is_absolute())
  {
    return Err(fail(
      "vision_endpoint_unavailable",
      "/cores/isos/layers/process/groups/component/nodes/bridge/settings/args",
      "Select one matching private EOS control socket",
    ));
  }
  let listener = arg(bridge, "--listen").ok_or_else(|| {
    fail(
      "vision_endpoint_unavailable",
      "/cores/isos/layers/process/groups/component/nodes/bridge/settings/args",
      "Select one explicit loopback bridge listener",
    )
  })?;
  let address = listener.parse::<std::net::SocketAddrV4>().map_err(|_| {
    fail(
      "vision_endpoint_unavailable",
      "/cores/isos/layers/process/groups/component/nodes/bridge/settings/args",
      "Bridge listener must be 127.0.0.1:<port>",
    )
  })?;
  if *address.ip() != std::net::Ipv4Addr::LOCALHOST || address.port() < 1024 {
    return Err(fail(
      "vision_endpoint_unavailable",
      "/cores/isos/layers/process/groups/component/nodes/bridge/settings/args",
      "Bridge listener must be 127.0.0.1:<port>",
    ));
  }
  if bridge.settings.ready_tcp.as_deref() != Some(listener) || bridge.settings.ready_unix.is_some()
  {
    return Err(fail(
      "vision_endpoint_unavailable",
      "/cores/isos/layers/process/groups/component/nodes/bridge/settings/ready_tcp",
      "Bridge readiness must match the selected public listener",
    ));
  }
  if !graph_revision.bytes().all(|byte| byte.is_ascii_hexdigit()) || graph_revision.len() != 64 {
    return Err(fail(
      "vision_graph_stale",
      "/revision",
      "Graph revision is invalid",
    ));
  }
  Ok(Some(Template {
    schema_version: 1,
    graph_revision: graph_revision.into(),
    projection_revision: hex::encode(Sha256::digest(projection_bytes)),
    source_path: source.into(),
    gate_path: gate,
    ingress_type: INGRESS.into(),
    render: Render {
      max_width_px: 1280,
      jpeg_quality: 80,
    },
    target_fps: 30,
    endpoint_kind: "uds".into(),
    endpoint_name: "eos-vision-owner.sock".into(),
  }))
}

fn arg<'a>(spec: &'a ProcessSpec, flag: &str) -> Option<&'a str> {
  let mut matches = spec.settings.args.windows(2).filter(|pair| pair[0] == flag);
  let value = matches.next()?.get(1)?.as_str();
  (matches.next().is_none() && !value.starts_with("--")).then_some(value)
}

pub fn bridge_ready(spec: &ProcessSpec, owner_socket: &Path) -> bool {
  use std::net::{SocketAddr, TcpStream};
  let Some(listener) = arg(spec, "--listen") else {
    return false;
  };
  if spec.settings.ready_tcp.as_deref() != Some(listener) {
    return false;
  }
  let Ok(address) = listener.parse::<SocketAddr>() else {
    return false;
  };
  if !address.ip().is_loopback() {
    return false;
  }
  TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok()
    && std::os::unix::net::UnixStream::connect(owner_socket).is_ok()
}

pub fn projection(template: &Template) -> Value {
  json!({"gate_path":template.gate_path,"source_path":template.source_path,
        "projection_revision":template.projection_revision,"render":template.render,
        "target_fps":template.target_fps})
}

pub fn binding(
  template: &Template,
  run_id: &str,
  grant_root: &Path,
  endpoint_root: &Path,
) -> Value {
  let mut value = serde_json::to_value(template).expect("serializable template");
  let object = value.as_object_mut().expect("template object");
  object.remove("endpoint_kind");
  object.remove("endpoint_name");
  object.insert("deployment_run_id".into(), json!(run_id));
  object.insert(
    "eos_endpoint".into(),
    json!({"transport":"uds","path":endpoint_root.join("eos-vision-owner.sock")}),
  );
  object.insert(
    "grant_file".into(),
    json!(grant_root.join("ei-vision.grant")),
  );
  value
}

pub fn selected_template(
  paths: &config::Paths,
  deployment: &str,
  revision: &str,
) -> Result<Option<Template>, Diagnostic> {
  #[cfg(test)]
  SELECTED_TEMPLATE_DELAY_MS.with(|delay| {
    if delay.get() > 0 {
      std::thread::sleep(Duration::from_millis(delay.get()));
    }
  });
  let saved = graph_contract::read(&paths.node_graph_path(deployment)).map_err(|_| {
    fail(
      "vision_graph_stale",
      "/revision",
      "Saved graph is unavailable",
    )
  })?;
  if saved.revision != revision {
    return Err(fail(
      "vision_graph_stale",
      "/revision",
      "Saved graph changed",
    ));
  }
  let root =
    crate::domain::graph::generated_v2::selected_v2_root(paths, deployment, &saved.value, revision)
      .map_err(|_| {
        fail(
          "vision_binding_changed",
          "/vision_evidence/gate_path",
          "Selected generation changed",
        )
      })?;
  let path = root.join(TEMPLATE_FILE);
  let selected = selected_gate_for_graph(&saved.value)?;
  if selected.is_none() {
    if path.exists() {
      return Err(fail(
        "vision_binding_changed",
        "/vision_evidence/gate_path",
        "Unselected binding is present",
      ));
    }
    return Ok(None);
  }
  let bytes = fs::read(&path).map_err(|_| {
    fail(
      "vision_binding_changed",
      "/vision_evidence/gate_path",
      "Selected template is absent",
    )
  })?;
  let template: Template = serde_json::from_slice(&bytes).map_err(|_| {
    fail(
      "vision_binding_changed",
      "/vision_evidence/gate_path",
      "Selected template is invalid",
    )
  })?;
  let (gate, _) = selected.expect("selected gate");
  if template.graph_revision != revision
    || template.gate_path != gate
    || template.schema_version != 1
    || template.ingress_type != INGRESS
    || template.endpoint_kind != "uds"
    || template.endpoint_name != "eos-vision-owner.sock"
    || template.render
      != (Render {
        max_width_px: 1280,
        jpeg_quality: 80,
      })
    || template.target_fps != 30
  {
    return Err(fail(
      "vision_binding_changed",
      "/vision_evidence/gate_path",
      "Selected binding changed",
    ));
  }
  Ok(Some(template))
}

pub fn attest_eos(spec: &ProcessSpec, template: &Template, run_id: &str) -> bool {
  let socket = arg(spec, "--socket").map(Path::new);
  let Some(socket) = socket else { return false };
  let Ok(mut stream) = UnixStream::connect(socket) else {
    return false;
  };
  if stream
    .set_read_timeout(Some(Duration::from_millis(500)))
    .is_err()
    || stream
      .set_write_timeout(Some(Duration::from_millis(500)))
      .is_err()
  {
    return false;
  }
  let request = json!({"op":"vision_attest","gate_path":template.gate_path}).to_string() + "\n";
  if stream.write_all(request.as_bytes()).is_err() {
    return false;
  }
  let mut bytes = Vec::new();
  let Ok(count) = BufReader::new(stream)
    .take(4097)
    .read_until(b'\n', &mut bytes)
  else {
    return false;
  };
  if count == 0 || count > 4096 || bytes.last() != Some(&b'\n') {
    return false;
  }
  let Ok(value) = serde_json::from_slice::<Value>(&bytes[..count - 1]) else {
    return false;
  };
  value["ok"] == true
    && value["ready"] == true
    && value["run_id"] == run_id
    && value["active_revision"] == template.graph_revision
    && value["projection_revision"] == template.projection_revision
    && value["source_path"] == template.source_path
    && value["gate_path"] == template.gate_path
    && value["runtime_stream_id"]
      .as_str()
      .is_some_and(|id| !id.is_empty())
    && value["gate_generation"].as_u64().is_some()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn health_only_graph_has_no_vision_binding() {
    let graph = json!({"version":"0.2.0","cores":{
      "isos":{"layers":{}},"ei":{"layers":{}},"eos":{"layers":{}}
    }});
    let selected = select(&graph, &json!({"types":[]}), &[], &"0".repeat(64), b"").unwrap();
    assert!(selected.is_none());
  }
}
