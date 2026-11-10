//! Run-fenced button activation ingress. The browser supplies an event, never a movement target.

use super::*;
use axum::body::Bytes;
use crate::domain::graph::{edge_admission, graph_contract};
use crate::runtime::owners::owner_services;
use std::sync::Mutex;
use std::time::Duration;

const BUTTON_TYPE: &str = "isos.ui.button.trigger@v0.2";
const ACTIVATION_TYPE: &str = "button_activation@v0.2";
const MAX_EVENT_BYTES: usize = 2048;
const MONITOR_TYPE: &str = "arm_monitor_state@v0.2";
const MONITOR_CACHE_AGE: Duration = Duration::from_millis(200);

struct MonitorCache {
  deployment: String,
  run_id: String,
  revision: String,
  loaded_at: Instant,
  observation: Value,
}

// One owner read per refresh interval serves every UI monitor reader. EOS's
// safety sampler and EI's upstream refresh never depend on this cache lock.
static MONITOR_CACHE: Mutex<Option<MonitorCache>> = Mutex::new(None);
const MONITOR_CHAIN: [(&str, &str); 8] = [
  (
    "eos/hardware/mock/arm_observer",
    "eos.hardware.mock.arm_observer@v0.2",
  ),
  (
    "eos/dispatcher/arm/monitor",
    "eos.dispatcher.arm.monitor@v0.2",
  ),
  (
    "eos/skill/demo/arm_status",
    "eos.skill.demo.arm_status@v0.2",
  ),
  (
    "eos/interface/egress/arm_status",
    "eos.interface.egress.arm_status@v0.2",
  ),
  ("ei/io/input/arm_status", "ei.io.input.arm_status@v0.2"),
  (
    "ei/cognition/state/arm_status",
    "ei.cognition.state.arm_status@v0.2",
  ),
  (
    "ei/governance/monitor/arm_status",
    "ei.governance.monitor.arm_status@v0.2",
  ),
  ("isos/ui/monitor/arm", "isos.ui.monitor.arm@v0.2"),
];
const CHAIN: [(&str, &str, &str); 7] = [
  ("ei.io.input.trigger@v0.2", "activation", "activation"),
  (
    "ei.cognition.intent.forwarder@v0.2",
    "activation",
    "activation",
  ),
  ("ei.governance.safety.gate@v0.2", "activation", "activation"),
  (
    "eos.interface.ingress.trigger@v0.2",
    "activation",
    "activation",
  ),
  ("eos.skill.demo.joint_delta@v0.2", "delta", "activation"),
  ("eos.dispatcher.arm.delta@v0.2", "delta", "delta"),
  ("eos.hardware.mock.arm@v0.2", "delta", "delta"),
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Activation {
  event_id: String,
  kind: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventRequest {
  schema_version: String,
  source_node: String,
  output: String,
  data_type: String,
  run_id: String,
  active_revision: String,
  payload: Activation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventResultRequest {
  schema_version: String,
  source_node: String,
  run_id: String,
  active_revision: String,
  event_id: String,
}

fn valid_id(value: &str, max: usize) -> bool {
  !value.is_empty()
    && value.len() <= max
    && value
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_node_path(path: &str) -> bool {
  path.split('/').count() == 4
    && path.split('/').all(|part| {
      !part.is_empty()
        && part.len() <= 48
        && part
          .bytes()
          .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    })
}

fn node<'a>(graph: &'a Value, path: &str) -> Option<&'a Value> {
  let parts: Vec<_> = path.split('/').collect();
  if !valid_node_path(path) {
    return None;
  }
  graph.pointer(&format!(
    "/cores/{}/layers/{}/groups/{}/nodes/{}",
    parts[0], parts[1], parts[2], parts[3]
  ))
}

fn next_node(graph: &Value, source: &str, output: &str, input: &str, kind: &str) -> Option<String> {
  let mut matches = Vec::new();
  for (core_id, core) in graph["cores"].as_object()? {
    for (layer_id, layer) in core["layers"].as_object()? {
      for (group_id, group) in layer["groups"].as_object()? {
        for (node_id, candidate) in group["nodes"].as_object()? {
          let path = format!("{core_id}/{layer_id}/{group_id}/{node_id}");
          let incoming = &candidate["inputs"][input];
          if candidate["node_type"] == kind
            && incoming["node"] == source
            && incoming["output"] == output
          {
            matches.push(path);
          }
        }
      }
    }
  }
  (matches.len() == 1).then(|| matches.remove(0))
}

fn chain_is_declared(graph: &Value, source: &str) -> bool {
  if node(graph, source).is_none_or(|entry| entry["node_type"] != BUTTON_TYPE) {
    return false;
  }
  let mut previous = source.to_owned();
  for (index, (kind, output, input)) in CHAIN.iter().enumerate() {
    let source_output = if index == 0 {
      "activation"
    } else {
      CHAIN[index - 1].1
    };
    let Some(next) = next_node(graph, &previous, source_output, input, kind) else {
      return false;
    };
    previous = next;
    let _ = output;
  }
  true
}

fn monitor_source_node(graph: &Value) -> Option<&'static str> {
  let source = "eos/hardware/mock/arm_observer";
  if let Some(first) = node(graph, source) {
    if first["node_type"] == "eos.hardware.mock.arm_observer@v0.2"
      && first
        .get("inputs")
        .is_none_or(|inputs| inputs.as_object().is_some_and(|map| map.is_empty()))
    {
      return Some(source);
    }
    return None;
  }
  let source = "eos/hardware/mock/arm_output";
  let first = node(graph, source)?;
  let arm = node(graph, "eos/hardware/mock/arm")?;
  (first["node_type"] == "eos.hardware.mock.arm_output@v0.2"
    && arm["node_type"] == "eos.hardware.mock.arm@v0.2"
    && first["inputs"]["observation"]["node"] == "eos/hardware/mock/arm"
    && first["inputs"]["observation"]["output"] == "observation"
    && first["inputs"]
      .as_object()
      .is_some_and(|map| map.len() == 1))
  .then_some("eos/hardware/mock/arm")
}

fn monitor_chain_is_declared(graph: &Value) -> bool {
  let Some(observation_source) = monitor_source_node(graph) else {
    return false;
  };
  let source = if observation_source == "eos/hardware/mock/arm" {
    "eos/hardware/mock/arm_output"
  } else {
    "eos/hardware/mock/arm_observer"
  };
  let mut previous = source;
  for (path, node_type) in MONITOR_CHAIN.into_iter().skip(1) {
    let Some(stage) = node(graph, path) else {
      return false;
    };
    if stage["node_type"] != node_type
      || stage["inputs"]["observation"]["node"] != previous
      || stage["inputs"]["observation"]["output"] != "observation"
      || stage["inputs"]
        .as_object()
        .is_none_or(|inputs| inputs.len() != 1)
    {
      return false;
    }
    previous = path;
  }
  true
}

fn valid_monitor_observation(value: &Value, run_id: &str, revision: &str, graph: &Value) -> bool {
  let expected_names =
    &graph["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm"]["joint_names"];
  let state = value["state"].as_str();
  let positions = &value["joint_positions_rad"];
  let positions_valid = positions.as_array().is_some_and(|joints| {
    joints.len() == 6
      && joints
        .iter()
        .all(|joint| joint.as_f64().is_some_and(f64::is_finite))
  });
  value["run_id"] == run_id
    && value["revision"] == revision
    && monitor_source_node(graph).is_some_and(|source| value["source_node"] == source)
    && value["sequence"].as_u64().is_some()
    && value["sampled_at_monotonic_ns"].as_u64().is_some()
    && value["joint_names"] == *expected_names
    && value["joint_names"]
      .as_array()
      .is_some_and(|names| names.len() == 6)
    && (value["reason_code"].is_null() || value["reason_code"].is_string())
    && match state {
      Some("ready") => positions_valid && value["reason_code"].is_null(),
      Some("degraded" | "stale" | "stopped") => positions.is_null(),
      _ => false,
    }
}

fn monitor_snapshot(paths: Paths, deployment: String) -> ApiResult {
  let status = crate::runtime::manager::manager_status_for_http(&paths, &deployment)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if status["state"] == "stopped" {
    if let Ok(mut cache) = MONITOR_CACHE.lock() {
      *cache = None;
    }
    return ok(json!({"state":"stopped","data_type":MONITOR_TYPE,
      "run_id":null,"active_revision":null,"observation":null}));
  }
  if !matches!(status["state"].as_str(), Some("running" | "degraded"))
    || status["run_id_state"] != "known"
  {
    return Err(fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "run_identity_unknown",
      "active run identity is unavailable",
      Value::Null,
    ));
  }
  let run_id = status["run_id"].as_str().ok_or_else(|| {
    fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "run_identity_unknown",
      "active run identity is unavailable",
      Value::Null,
    )
  })?;
  let revision = status["active_revision"].as_str().ok_or_else(|| {
    fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "run_identity_unknown",
      "active revision is unavailable",
      Value::Null,
    )
  })?;
  let saved = graph_contract::read(&paths.node_graph_path(&deployment))
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if saved.revision != revision {
    return Err(fail(
      StatusCode::CONFLICT,
      "graph_fence_stale",
      "active graph changed",
      Value::Null,
    ));
  }
  if !monitor_chain_is_declared(&saved.value) {
    return Err(fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      "monitor_route_invalid",
      "arm monitor chain is incomplete",
      Value::Null,
    ));
  }
  let observation = {
    let mut cache = MONITOR_CACHE.lock().map_err(|_| {
      fail(
        StatusCode::SERVICE_UNAVAILABLE,
        "monitor_state_invalid",
        "monitor cache is unavailable",
        Value::Null,
      )
    })?;
    if let Some(entry) = cache.as_ref().filter(|entry| {
      entry.deployment == deployment
        && entry.run_id == run_id
        && entry.revision == revision
        && entry.loaded_at.elapsed() < MONITOR_CACHE_AGE
    }) {
      entry.observation.clone()
    } else {
      let reply = owner_services::setup(
        &paths,
        "ei",
        &json!({
          "core":"ei", "op":"monitor", "run_id":run_id, "revision":revision
        }),
      )
      .map_err(|_| {
        fail(
          StatusCode::SERVICE_UNAVAILABLE,
          "owner_unavailable",
          "EI monitor is unavailable",
          Value::Null,
        )
      })?;
      if reply["ok"] != true
        || reply["run_id"] != run_id
        || reply["revision"] != revision
        || !valid_monitor_observation(&reply["observation"], run_id, revision, &saved.value)
      {
        return Err(fail(
          StatusCode::SERVICE_UNAVAILABLE,
          "monitor_state_invalid",
          "EI monitor state is unavailable",
          Value::Null,
        ));
      }
      let observation = reply["observation"].clone();
      *cache = Some(MonitorCache {
        deployment: deployment.clone(),
        run_id: run_id.to_owned(),
        revision: revision.to_owned(),
        loaded_at: Instant::now(),
        observation: observation.clone(),
      });
      observation
    }
  };
  let current = crate::runtime::manager::manager_status_for_http(&paths, &deployment)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if !matches!(current["state"].as_str(), Some("running" | "degraded"))
    || current["run_id"] != run_id
    || current["active_revision"] != revision
  {
    return Err(fail(
      StatusCode::CONFLICT,
      "run_fence_stale",
      "monitor run changed",
      Value::Null,
    ));
  }
  ok(json!({"state":"running","data_type":MONITOR_TYPE,
    "run_id":run_id,"active_revision":revision,"observation":observation}))
}

pub(crate) async fn node_monitor(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  let paths = app.paths.clone();
  let deployment = app.deployment.clone();
  tokio::task::spawn_blocking(move || monitor_snapshot(paths, deployment))
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?
}

fn uncertain_owner_reply(reply: &Value) -> bool {
  matches!(
    reply["error"].as_str(),
    Some("outcome_unknown" | "eos_outcome_unknown" | "eos_result_fence_mismatch")
  )
}

fn dispatch(paths: Paths, deployment: String, event: EventRequest) -> ApiResult {
  if event.schema_version != "0.2.0"
    || event.output != "activation"
    || event.data_type != ACTIVATION_TYPE
    || event.payload.kind != "activate"
    || !valid_id(&event.payload.event_id, 64)
    || !valid_id(&event.run_id, 128)
    || event.active_revision.len() != 64
    || !event
      .active_revision
      .bytes()
      .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    || !valid_node_path(&event.source_node)
  {
    return Err(fail(
      StatusCode::BAD_REQUEST,
      "node_event_invalid",
      "button event is invalid",
      Value::Null,
    ));
  }
  let status = crate::runtime::manager::manager_status_for_http(&paths, &deployment)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if status["state"] != "running"
    || status["run_id_state"] != "known"
    || status["run_id"] != event.run_id
    || status["active_revision"] != event.active_revision
  {
    return Err(fail(
      StatusCode::CONFLICT,
      "run_fence_stale",
      "button event run is stale",
      Value::Null,
    ));
  }
  let bytes = fs::read(paths.node_graph_path(&deployment))
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if bytes.len() > graph_contract::MAX_GRAPH_BYTES
    || hex::encode(Sha256::digest(&bytes)) != event.active_revision
  {
    return Err(fail(
      StatusCode::CONFLICT,
      "graph_fence_stale",
      "active graph changed",
      Value::Null,
    ));
  }
  let graph: Value = serde_json::from_slice(&bytes)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  let catalog = process::catalog_for_graph(&paths, &graph)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  edge_admission::admit(&graph, &catalog).map_err(|issue| {
    fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      "node_edges_invalid",
      issue.message.clone(),
      Value::Null,
    )
  })?;
  if !chain_is_declared(&graph, &event.source_node) {
    return Err(fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      "node_route_invalid",
      "button chain is incomplete",
      Value::Null,
    ));
  }
  let reply = owner_services::setup(
    &paths,
    "ei",
    &json!({
      "core":"ei", "op":"activate", "run_id":event.run_id,
      "revision":event.active_revision, "source_node":event.source_node,
      "event_id":event.payload.event_id, "kind":"activate"
    }),
  )
  .map_err(|_| {
    fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "owner_outcome_unknown",
      "button outcome is unknown",
      Value::Null,
    )
  })?;
  if uncertain_owner_reply(&reply) {
    return Err(fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "owner_outcome_unknown",
      "button outcome is unknown",
      Value::Null,
    ));
  }
  if reply["ok"] != true
    || reply["run_id"] != event.run_id
    || reply["revision"] != event.active_revision
    || reply["event_id"] != event.payload.event_id
    || !matches!(
      reply["status"].as_str(),
      Some("accepted" | "running" | "succeeded")
    )
    || reply["command_id"].as_str().is_none_or(str::is_empty)
  {
    return Err(fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      "owner_event_rejected",
      "EI rejected button event",
      Value::Null,
    ));
  }
  ok(reply)
}

fn dispatch_result(paths: Paths, deployment: String, request: EventResultRequest) -> ApiResult {
  if request.schema_version != "0.2.0"
    || !valid_node_path(&request.source_node)
    || !valid_id(&request.event_id, 64)
    || !valid_id(&request.run_id, 128)
    || request.active_revision.len() != 64
    || !request
      .active_revision
      .bytes()
      .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
  {
    return Err(fail(
      StatusCode::BAD_REQUEST,
      "node_result_invalid",
      "button result request is invalid",
      Value::Null,
    ));
  }
  let status = crate::runtime::manager::manager_status_for_http(&paths, &deployment)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if !matches!(status["state"].as_str(), Some("running" | "degraded"))
    || status["run_id_state"] != "known"
    || status["run_id"] != request.run_id
    || status["active_revision"] != request.active_revision
  {
    return Err(fail(
      StatusCode::CONFLICT,
      "run_fence_stale",
      "button result run is stale",
      Value::Null,
    ));
  }
  let saved = graph_contract::read(&paths.node_graph_path(&deployment))
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if saved.revision != request.active_revision {
    return Err(fail(
      StatusCode::CONFLICT,
      "graph_fence_stale",
      "active graph changed",
      Value::Null,
    ));
  }
  if !chain_is_declared(&saved.value, &request.source_node) {
    return Err(fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      "node_route_invalid",
      "button chain is incomplete",
      Value::Null,
    ));
  }
  let reply = owner_services::setup(
    &paths,
    "ei",
    &json!({"core":"ei","op":"result","run_id":request.run_id,
      "revision":request.active_revision,"event_id":request.event_id}),
  )
  .map_err(|_| {
    fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "owner_result_unavailable",
      "button result is unavailable",
      Value::Null,
    )
  })?;
  if reply["ok"] != true
    || reply["run_id"] != request.run_id
    || reply["revision"] != request.active_revision
    || reply["event_id"] != request.event_id
    || reply["command_id"].as_str().is_none_or(str::is_empty)
    || !matches!(
      reply["status"].as_str(),
      Some("accepted" | "running" | "succeeded" | "failed" | "cancelled" | "outcome_unknown")
    )
    || (reply["status"] == "succeeded"
      && !reply["joint_positions_rad"]
        .as_array()
        .is_some_and(|joints| {
          joints.len() == 6
            && joints
              .iter()
              .all(|joint| joint.as_f64().is_some_and(f64::is_finite))
        }))
  {
    return Err(fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "owner_result_invalid",
      "EI button result is unavailable",
      Value::Null,
    ));
  }
  let current = crate::runtime::manager::manager_status_for_http(&paths, &deployment)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if current["run_id"] != request.run_id || current["active_revision"] != request.active_revision {
    return Err(fail(
      StatusCode::CONFLICT,
      "run_fence_stale",
      "button result run changed",
      Value::Null,
    ));
  }
  ok(reply)
}

pub(crate) async fn node_event_result(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  bytes: Bytes,
) -> ApiResult {
  authorized(&app, &headers)?;
  if bytes.len() > MAX_EVENT_BYTES {
    return Err(fail(
      StatusCode::PAYLOAD_TOO_LARGE,
      "node_result_oversized",
      "button result request exceeds 2048 bytes",
      Value::Null,
    ));
  }
  let request: EventResultRequest = serde_json::from_slice(&bytes).map_err(|_| {
    fail(
      StatusCode::BAD_REQUEST,
      "node_result_invalid",
      "button result request is invalid",
      Value::Null,
    )
  })?;
  let paths = app.paths.clone();
  let deployment = app.deployment.clone();
  tokio::task::spawn_blocking(move || dispatch_result(paths, deployment, request))
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?
}

pub(crate) async fn node_event(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  bytes: Bytes,
) -> ApiResult {
  authorized(&app, &headers)?;
  if bytes.len() > MAX_EVENT_BYTES {
    return Err(fail(
      StatusCode::PAYLOAD_TOO_LARGE,
      "node_event_oversized",
      "button event exceeds 2048 bytes",
      Value::Null,
    ));
  }
  let request: EventRequest = serde_json::from_slice(&bytes).map_err(|_| {
    fail(
      StatusCode::BAD_REQUEST,
      "node_event_invalid",
      "button event is invalid",
      Value::Null,
    )
  })?;
  let paths = app.paths.clone();
  let deployment = app.deployment.clone();
  tokio::task::spawn_blocking(move || dispatch(paths, deployment, request))
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?
}

#[cfg(test)]
mod tests {
  use super::*;

  fn graph() -> Value {
    let mut graph = json!({"cores":{}});
    let nodes = [
      ("isos", "ui", "button", "trigger", BUTTON_TYPE, None),
      (
        "ei",
        "io",
        "input",
        "trigger",
        CHAIN[0].0,
        Some("isos/ui/button/trigger"),
      ),
      (
        "ei",
        "cognition",
        "intent",
        "forwarder",
        CHAIN[1].0,
        Some("ei/io/input/trigger"),
      ),
      (
        "ei",
        "governance",
        "safety",
        "gate",
        CHAIN[2].0,
        Some("ei/cognition/intent/forwarder"),
      ),
      (
        "eos",
        "interface",
        "ingress",
        "trigger",
        CHAIN[3].0,
        Some("ei/governance/safety/gate"),
      ),
      (
        "eos",
        "skill",
        "demo",
        "joint_delta",
        CHAIN[4].0,
        Some("eos/interface/ingress/trigger"),
      ),
      (
        "eos",
        "dispatcher",
        "arm",
        "delta",
        CHAIN[5].0,
        Some("eos/skill/demo/joint_delta"),
      ),
      (
        "eos",
        "hardware",
        "mock",
        "arm",
        CHAIN[6].0,
        Some("eos/dispatcher/arm/delta"),
      ),
    ];
    for (index, (core, layer, group, id, kind, source)) in nodes.into_iter().enumerate() {
      let input_name = if index >= 6 { "delta" } else { "activation" };
      let output_name = if index >= 6 { "delta" } else { "activation" };
      graph["cores"][core]["layers"][layer]["groups"][group]["nodes"][id] = json!({"node_type":kind,"inputs":source.map(|source| json!({input_name:{"node":source,"output":output_name}})).unwrap_or_else(|| json!({}))});
    }
    graph
  }

  #[test]
  fn button_route_requires_all_eight_nodes() {
    let mut graph = graph();
    assert!(chain_is_declared(&graph, "isos/ui/button/trigger"));
    graph["cores"]["eos"]["layers"]["skill"]["groups"]["demo"]["nodes"]["joint_delta"]["inputs"] =
      json!({});
    assert!(!chain_is_declared(&graph, "isos/ui/button/trigger"));
  }

  #[test]
  fn button_route_rejects_ambiguous_stage() {
    let mut graph = graph();
    graph["cores"]["ei"]["layers"]["io"]["groups"]["input"]["nodes"]["second"] = json!({"node_type":CHAIN[0].0,"inputs":{"activation":{"node":"isos/ui/button/trigger","output":"activation"}}});
    assert!(!chain_is_declared(&graph, "isos/ui/button/trigger"));
  }

  #[test]
  fn button_route_accepts_another_declared_button_instance() {
    let mut graph = graph();
    graph["cores"]["isos"]["layers"]["ui"]["groups"]["button"]["nodes"]["second"] =
      json!({"node_type":BUTTON_TYPE});
    graph["cores"]["ei"]["layers"]["io"]["groups"]["input"]["nodes"]["trigger"]["inputs"]["activation"]
      ["node"] = json!("isos/ui/button/second");
    assert!(chain_is_declared(&graph, "isos/ui/button/second"));
    assert!(!chain_is_declared(&graph, "isos/ui/button/trigger"));
  }

  #[test]
  fn ambiguous_owner_outcome_is_not_a_definite_rejection() {
    assert!(uncertain_owner_reply(&json!({"error":"outcome_unknown"})));
    assert!(uncertain_owner_reply(
      &json!({"error":"eos_outcome_unknown"})
    ));
    assert!(!uncertain_owner_reply(
      &json!({"error":"activation_path_invalid"})
    ));
  }

  #[test]
  fn event_id_is_bounded_ascii() {
    assert!(valid_id("click-1", 64));
    assert!(!valid_id("", 64));
    assert!(!valid_id("click/1", 64));
    assert!(!valid_id("点", 64));
    assert!(!valid_id(&"a".repeat(65), 64));
  }

  #[test]
  fn monitor_route_requires_independent_source_and_every_return_stage() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    let mut graph = crate::domain::graph::starter::draft(&paths, "demo").unwrap();
    assert!(monitor_chain_is_declared(&graph));
    graph["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm_observer"]["inputs"] =
      json!({"observation":{"node":"eos/hardware/mock/arm","output":"observation"}});
    assert!(!monitor_chain_is_declared(&graph));
    graph["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm_observer"]
      .as_object_mut()
      .unwrap()
      .remove("inputs");
    graph["cores"]["ei"]["layers"]["cognition"]["groups"]["state"]["nodes"]["arm_status"]["inputs"] =
      json!({});
    assert!(!monitor_chain_is_declared(&graph));
  }

  #[test]
  fn monitor_observation_rejects_invalid_or_stale_joint_data() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    let graph = crate::domain::graph::starter::draft(&paths, "demo").unwrap();
    let mut sample = json!({
      "run_id":"run-1", "revision":"a".repeat(64),
      "source_node":"eos/hardware/mock/arm_observer", "sequence":1,
      "sampled_at_monotonic_ns":1234,
      "joint_names":["joint_1","joint_2","joint_3","joint_4","joint_5","joint_6"],
      "joint_positions_rad":[0.0,0.0,0.0,0.0,0.0,0.0],
      "state":"ready", "reason_code":null
    });
    assert!(valid_monitor_observation(
      &sample,
      "run-1",
      &"a".repeat(64),
      &graph
    ));
    sample["source_node"] = json!("eos/hardware/mock/arm");
    assert!(!valid_monitor_observation(
      &sample,
      "run-1",
      &"a".repeat(64),
      &graph
    ));
    sample["source_node"] = json!("eos/hardware/mock/arm_observer");
    sample["joint_positions_rad"] = Value::Null;
    assert!(!valid_monitor_observation(
      &sample,
      "run-1",
      &"a".repeat(64),
      &graph
    ));
    sample["state"] = json!("stale");
    assert!(valid_monitor_observation(
      &sample,
      "run-1",
      &"a".repeat(64),
      &graph
    ));
    sample["run_id"] = json!("old-run");
    assert!(!valid_monitor_observation(
      &sample,
      "run-1",
      &"a".repeat(64),
      &graph
    ));
  }

  #[test]
  fn legacy_monitor_source_still_accepts_arm_observation() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    let mut graph = crate::domain::graph::starter::draft(&paths, "demo").unwrap();
    let mock = graph["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]
      .as_object_mut()
      .unwrap();
    mock.remove("arm_observer");
    mock.insert(
      "arm_output".into(),
      json!({
        "id":"arm_output", "node_type":"eos.hardware.mock.arm_output@v0.2",
        "inputs":{"observation":{"node":"eos/hardware/mock/arm","output":"observation"}}
      }),
    );
    graph["cores"]["eos"]["layers"]["dispatcher"]["groups"]["arm"]["nodes"]["monitor"]["inputs"]
      ["observation"]["node"] = json!("eos/hardware/mock/arm_output");
    assert!(monitor_chain_is_declared(&graph));
    let mut sample = json!({
      "run_id":"run-1", "revision":"a".repeat(64),
      "source_node":"eos/hardware/mock/arm", "sequence":1,
      "sampled_at_monotonic_ns":1234,
      "joint_names":["joint_1","joint_2","joint_3","joint_4","joint_5","joint_6"],
      "joint_positions_rad":[0.0,0.0,0.0,0.0,0.0,0.0],
      "state":"ready", "reason_code":null
    });
    assert!(valid_monitor_observation(
      &sample,
      "run-1",
      &"a".repeat(64),
      &graph
    ));
    sample["source_node"] = json!("eos/hardware/mock/arm_observer");
    assert!(!valid_monitor_observation(
      &sample,
      "run-1",
      &"a".repeat(64),
      &graph
    ));
  }
}
