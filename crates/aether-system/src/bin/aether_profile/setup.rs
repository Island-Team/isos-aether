//! Bounded owner setup for the EOS-only graph projection.
use std::{
  collections::BTreeMap,
  error::Error,
  io::{Read, Write},
};
use aether_hal::{DataClass, DeliveryMode, NodeKind, NodeTypeRegistry, PortTransport, Profile};
use serde_json::{json, Value};
const MAX_REQUEST_BYTES: u64 = 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
pub fn run(registry: &NodeTypeRegistry) -> Result<(), Box<dyn Error>> {
  let mut input = Vec::new();
  std::io::stdin()
    .take(MAX_REQUEST_BYTES + 1)
    .read_to_end(&mut input)?;
  let reply = if input.len() as u64 > MAX_REQUEST_BYTES {
    failure(
      "request",
      "request_too_large",
      "setup request exceeds 1 MiB",
    )
  } else {
    match serde_json::from_slice::<Value>(&input) {
      Ok(request) => match super::package_registry::selected(&request, registry) {
        Ok(selected) => handle(&request, &selected),
        Err(code) => failure("packages", code, "invalid selected package contribution"),
      },
      Err(_) => failure("request", "invalid_json", "setup request is not JSON"),
    }
  };
  let mut bytes = serde_json::to_vec(&reply)?;
  if bytes.len() > MAX_RESPONSE_BYTES {
    bytes = serde_json::to_vec(&failure(
      "response",
      "response_too_large",
      "setup reply exceeds 4 MiB",
    ))?;
  }
  std::io::stdout().write_all(&bytes)?;
  std::io::stdout().write_all(b"\n")?;
  Ok(())
}
fn failure(path: &str, code: &str, message: &str) -> Value {
  json!({"schema_version":3,"ok":false,"diagnostics":[{"path":path,"code":code,"message":message}],"files":[]})
}
fn options_for_draft(graph: &Value, registry: &NodeTypeRegistry) -> Value {
  let families = registry.catalog_adapter_families();
  let mut setting_options = Vec::new();
  if let Some(layers) = graph["cores"]["eos"]["layers"].as_object() {
    for (layer_id, layer) in layers {
      let Some(groups) = layer["groups"].as_object() else {
        continue;
      };
      for (group_id, group) in groups {
        let Some(nodes) = group["nodes"].as_object() else {
          continue;
        };
        for (node_id, node) in nodes {
          let Some(definition) = node["node_type"].as_str().and_then(|id| registry.get(id)) else {
            continue;
          };
          if definition.kind == NodeKind::Software {
            continue;
          }
          let rows: Vec<_> = families.iter().filter(|family| {
            family.descriptor.device_kinds.contains(&definition.kind)
              && family.package.as_ref().map(|pin| json!(pin)) == node.get("package").cloned()
              && (node["node_type"] != "eos.hardware.mock.arm@v0.2" || family.descriptor.id == "mock-arm")
          }).map(|family| {
            let available = family.compiled || registry.verified_adapter_library_for_family(&family.descriptor.id).is_ok();
            json!({"value":family.descriptor.id,"label":family.descriptor.id,
              "description":family.descriptor.description,"available":available,
              "reason_code":if available {Value::Null} else {json!("adapter_unavailable")},
              "reason":if available {Value::Null} else {json!("Verified EOS adapter library is unavailable")}})
          }).collect();
          setting_options.push(
            json!({"node_path":format!("eos/{layer_id}/{group_id}/{node_id}"),
            "setting":"adapter.type","selected":node["adapter"]["type"],"options":rows}),
          );
        }
      }
    }
  }
  json!({"schema_version":3,"ok":true,"diagnostics":[],"files":[],"catalog":{"setting_options":setting_options}})
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExecutionMode {
  Demo,
  Native,
  Mixed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkerKind {
  Demo,
  Native,
  OwnerBridge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RouteLink {
  pub source: String,
  pub output: String,
  pub target: String,
  pub input: String,
  pub data_type: String,
}

#[derive(Clone, Debug)]
pub(crate) struct RoutePlan {
  pub mode: ExecutionMode,
  pub workers: BTreeMap<String, WorkerKind>,
  pub links: Vec<RouteLink>,
}

fn is_demo_node(node_type: &str) -> bool {
  matches!(
    node_type,
    "eos.skill.demo.joint_delta@v0.2"
      | "eos.skill.demo.arm_status@v0.2"
      | "eos.dispatcher.arm.delta@v0.2"
      | "eos.dispatcher.arm.monitor@v0.2"
      | "eos.hardware.mock.arm@v0.2"
      | "eos.hardware.mock.arm_observer@v0.2"
      | "eos.hardware.mock.arm_output@v0.2"
      | "eos.interface.ingress.trigger@v0.2"
      | "eos.interface.egress.arm_status@v0.2"
  )
}

fn is_direct_only_node(node_type: &str) -> bool {
  matches!(
    node_type,
    "eos.hardware.mock.camera@v0.2"
      | "eos.perception.physical.features@v0.2"
      | "eos.dispatcher.command.binding@v0.2"
      | "eos.safety.device.guard@v0.2"
      | "eos.safety.device.estop@v0.2"
      | "eos.evidence.episode.recorder@v0.2"
      | "eos.dataset.export.manifest@v0.2"
  )
}

fn command_route_allowed(source: Option<&WorkerKind>, target: Option<&WorkerKind>) -> bool {
  source == Some(&WorkerKind::Demo) && target == Some(&WorkerKind::Demo)
}

fn worker_link_supported(source: WorkerKind, target: WorkerKind) -> bool {
  source == target || target != WorkerKind::Demo
}

fn demo_cross_host_output_supported(node_type: &str, output: &str) -> bool {
  node_type == "eos.hardware.mock.arm_observer@v0.2" && output == "observation"
}

pub(crate) fn demo_projection(profile: &Profile) -> Result<Profile, String> {
  let mut demo = profile.clone();
  for group in demo
    .cores
    .values_mut()
    .flat_map(|core| core.layers.values_mut())
    .flat_map(|layer| layer.groups.values_mut())
  {
    group.nodes.retain(|_, node| is_demo_node(&node.node_type));
  }
  demo
    .validate_demo_chain()
    .map_err(|error| format!("unsupported_demo_topology: {error}"))?;
  Ok(demo)
}

pub(crate) fn route_plan(
  profile: &Profile,
  registry: &NodeTypeRegistry,
) -> Result<RoutePlan, String> {
  let nodes: Vec<_> = profile.all_nodes().collect();
  if nodes.is_empty() {
    return Err("empty_eos_graph: no executable EOS nodes".into());
  }
  let demo_count = nodes
    .iter()
    .filter(|node| is_demo_node(&node.node_type))
    .count();
  if demo_count != 0 {
    demo_projection(profile)?;
  }
  let mut workers = BTreeMap::new();
  for node in nodes {
    let Some(definition) = registry.get(&node.node_type) else {
      return Err(format!("unknown_node_type: {}", node.node_type));
    };
    if is_demo_node(&node.node_type) {
      if node.package.is_some() {
        return Err(format!("builtin_demo_package_forbidden: {}", node.path()));
      }
      workers.insert(node.path().to_owned(), WorkerKind::Demo);
      continue;
    }
    if is_direct_only_node(&node.node_type) {
      return Err(format!(
        "isolated_node_graph_runtime_unavailable: {}",
        node.node_type
      ));
    }
    if node.node_type == "eos.interface.egress.vision_flow@v0.2" {
      workers.insert(node.path().to_owned(), WorkerKind::OwnerBridge);
      continue;
    }
    if node.node_type.starts_with("eos.process.") && node.package.is_some() {
      continue;
    }
    if definition.kind != NodeKind::Software {
      registry
        .verified_adapter_library_for_family(&node.adapter.adapter_type)
        .map_err(|error| format!("adapter_library_unavailable: {error}"))?;
      if definition.kind != NodeKind::Camera
        && !node.adapter.settings.contains_key("service_instance")
      {
        return Err(format!(
          "native_actuator_requires_dispatcher: {}",
          node.node_type
        ));
      }
    } else {
      if registry.native_library_for_type(&node.node_type).is_none() {
        return Err(format!(
          "unsupported_eos_node_implementation: {}",
          node.node_type
        ));
      }
      registry
        .verified_native_library_for_type(&node.node_type)
        .map_err(|error| format!("native_library_unavailable: {}: {error}", node.node_type))?;
    }
    workers.insert(node.path().to_owned(), WorkerKind::Native);
  }
  if !workers
    .values()
    .any(|worker| matches!(worker, WorkerKind::Demo | WorkerKind::Native))
  {
    return Err("no_executable_worker: EOS graph has no worker-hosted nodes".into());
  }
  let mode = match (
    demo_count != 0,
    workers.values().any(|kind| *kind != WorkerKind::Demo),
  ) {
    (true, true) => ExecutionMode::Mixed,
    (true, false) => ExecutionMode::Demo,
    (false, _) => ExecutionMode::Native,
  };
  let routes = registry.catalog_port_routes();
  let mut links = Vec::new();
  for target in profile.all_nodes() {
    let target_type = registry.get(&target.node_type).expect("checked node type");
    for (input_name, sources) in &target.inputs {
      let input = target_type
        .inputs
        .iter()
        .find(|port| port.name == *input_name)
        .ok_or_else(|| format!("unknown_input: {}/{}", target.path(), input_name))?;
      for source in sources.values() {
        if target_type.kind == NodeKind::Software
          && target_type
            .implementation
            .as_ref()
            .and_then(|implementation| implementation.abi.as_deref())
            == Some("isos_node_v2")
          && input.delivery_mode != DeliveryMode::Declaration
        {
          return Err(format!(
            "unsupported_v2_software_input: {}/{input_name}",
            target.path()
          ));
        }
        let producer = profile.node(&source.node);
        if producer.is_none() {
          // The EOS projection retains cross-owner ingress admitted by IsOS.
          continue;
        }
        let producer = producer.expect("checked source");
        let source_type = registry
          .get(&producer.node_type)
          .expect("checked source type");
        let output = source_type
          .outputs
          .iter()
          .find(|port| port.name == source.output)
          .ok_or_else(|| format!("unknown_output: {}/{}", source.node, source.output))?;
        let source_worker = workers.get(&source.node);
        let target_worker = workers.get(target.path());
        let command = input.data_class == Some(DataClass::Command)
          || output.data_class == Some(DataClass::Command)
          || matches!(
            input.data_type.id(),
            "base_movement" | "manipulator_movement" | "control_request" | "command_outcome"
          );
        if command && !command_route_allowed(source_worker, target_worker) {
          return Err(format!(
            "command_route_requires_dispatcher: {}/{input_name}",
            target.path()
          ));
        }
        if input.transport == Some(PortTransport::None)
          && input.delivery_mode == DeliveryMode::Declaration
        {
          continue;
        }
        let route_exists = routes.iter().any(|route| {
          route["source_type"] == source_type.id
            && route["output"] == source.output
            && route["target_type"] == target_type.id
            && route["input"] == *input_name
        });
        if !route_exists {
          return Err(format!(
            "unsupported_authored_route: {}/{input_name}",
            target.path()
          ));
        }
        if source_worker.is_none() || target_worker.is_none() {
          return Err(format!(
            "unhosted_authored_route: {}/{input_name}",
            target.path()
          ));
        }
        let source_worker = *source_worker.expect("checked graph host");
        let target_worker = *target_worker.expect("checked graph host");
        if !worker_link_supported(source_worker, target_worker) {
          return Err(format!(
            "unsupported_demo_worker_ingress: {}/{input_name}",
            target.path()
          ));
        }
        if source_worker == WorkerKind::Demo
          && target_worker != WorkerKind::Demo
          && !demo_cross_host_output_supported(&producer.node_type, &source.output)
        {
          return Err(format!(
            "unsupported_demo_worker_output: {}/{input_name}",
            target.path()
          ));
        }
        if source_worker != target_worker
          && (input.transport != Some(PortTransport::InProcess)
            || output.transport != Some(PortTransport::InProcess))
        {
          return Err(format!(
            "unsupported_cross_worker_transport: {}/{input_name}",
            target.path()
          ));
        }
        links.push(RouteLink {
          source: source.node.clone(),
          output: source.output.clone(),
          target: target.path().to_owned(),
          input: input_name.clone(),
          data_type: input.data_type.id().to_owned(),
        });
      }
    }
  }
  Ok(RoutePlan {
    mode,
    workers,
    links,
  })
}

pub(crate) fn execution_mode(
  profile: &Profile,
  registry: &NodeTypeRegistry,
) -> Result<ExecutionMode, String> {
  Ok(route_plan(profile, registry)?.mode)
}
pub(crate) fn handle(request: &Value, registry: &NodeTypeRegistry) -> Value {
  if request["schema_version"] != 3 || request["core"] != "eos" {
    return failure("request", "invalid_envelope", "expected EOS setup schema 3");
  }
  let op = request["op"].as_str().unwrap_or("");
  match op {
    "identity" => {
      return json!({"schema_version":3,"ok":true,"diagnostics":[],"files":[],"identity":{
      "core":"eos","product_version":env!("CARGO_PKG_VERSION"),"graph_version":"0.2.1","node_type_version":"0.2",
      "supported_graph_versions":["0.2.1"],"supported_node_type_versions":["0.2"],"supported_node_types":registry.all().filter(|node|node.core=="eos" && !is_direct_only_node(&node.id)).map(|node|node.id.as_str()).collect::<Vec<_>>(),
      "features":super::PROFILE_FEATURES,"setup_schema_version":3}});
    }
    "catalog" => {
      return json!({"schema_version":3,"ok":true,"diagnostics":[],"files":[],"catalog":{
      "types":registry.all().collect::<Vec<_>>(),"data_types":registry.data_types().collect::<Vec<_>>(),
      "direct_only_types":registry.all().filter(|node| is_direct_only_node(&node.id)).map(|node|node.id.as_str()).collect::<Vec<_>>(),
      "adapter_families":registry.catalog_adapter_families(),"primitive_specs":aether_hal::PRIMITIVE_SPECS.iter().filter(|spec|spec.name=="arm.delta").collect::<Vec<_>>(),
      "port_routes":registry.catalog_port_routes(),"bridge_adapters":registry.catalog_bridge_adapters()}});
    }
    "options" | "validate" | "render" => {}
    _ => return failure("op", "invalid_operation", "unknown setup operation"),
  }
  let revision = request["revision"].as_str().unwrap_or("");
  if revision.is_empty() || revision.len() > 128 || !revision.is_ascii() {
    return failure(
      "revision",
      "invalid_revision",
      "missing or invalid graph revision",
    );
  }
  if !request["endpoints"].is_object() || !request["packages"].is_array() {
    return failure(
      "request",
      "invalid_inputs",
      "expected endpoints object and packages array",
    );
  }
  let graph = &request["graph"];
  if graph["version"] != "0.2.1"
    || graph["cores"]
      .as_object()
      .is_none_or(|cores| cores.len() != 1 || !cores.contains_key("eos"))
  {
    return failure(
      "graph/cores",
      "invalid_graph",
      "expected EOS-only graph 0.2.1 projection",
    );
  }
  if op == "options" {
    return options_for_draft(graph, registry);
  }
  let source = match serde_json::to_string(graph) {
    Ok(source) => source,
    Err(_) => return failure("graph", "invalid_graph", "graph is not JSON"),
  };
  let profile = match Profile::from_json_with_registry(&source, registry) {
    Ok(profile) => profile,
    Err(error) => return failure("graph", "invalid_graph", &error.to_string()),
  };
  if let Err(reason) = execution_mode(&profile, registry) {
    return failure("graph/cores/eos", "unsupported_execution", &reason);
  }
  if op == "validate" {
    return json!({"schema_version":3,"ok":true,"diagnostics":[],"files":[]});
  }
  json!({"schema_version":3,"ok":true,"diagnostics":[],"files":[{"path":"eos/graph.json","content":source}]})
}
#[cfg(test)]
mod tests {
  use super::*;
  use sha2::{Digest, Sha256};
  #[test]
  fn isolated_catalog_types_are_not_demo_workers_or_supported_graph_types() {
    let id = "eos.hardware.mock.camera@v0.2";
    assert!(is_direct_only_node(id));
    assert!(!is_demo_node(id));
    let registry = NodeTypeRegistry::builtin();
    let reply = handle(
      &json!({"schema_version":3,"core":"eos","op":"identity"}),
      &registry,
    );
    let supported = reply["identity"]["supported_node_types"]
      .as_array()
      .unwrap();
    assert!(!supported.iter().any(|value| value == id));
    assert!(supported
      .iter()
      .any(|value| value == "eos.hardware.mock.arm@v0.2"));
  }
  fn demo_profile() -> Profile {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../tests/fixtures/graph/demo-only.nodes.json");
    let mut projection: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let eos = projection["cores"]["eos"].clone();
    projection["cores"] = json!({"eos":eos});
    Profile::from_json_with_registry(&projection.to_string(), &NodeTypeRegistry::builtin()).unwrap()
  }
  fn mixed_camera_profile(
    frame_transport: &str,
  ) -> Result<(Profile, NodeTypeRegistry, tempfile::TempDir), String> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("lib")).unwrap();
    let library = dir.path().join("lib/camera.so");
    std::fs::write(&library, b"route-plan-fixture").unwrap();
    let digest = format!("{:x}", Sha256::digest(std::fs::read(&library).unwrap()));
    let pin = json!({"id":"vendor/camera","version":"1.0.0"});
    let sidecar = dir.path().join("node-types.json");
    std::fs::write(
      &sidecar,
      json!({
        "package":pin,
        "libraries":[{"path":"lib/camera.so","sha256":digest}],
        "adapters":[{"family":"camera-test","library":"lib/camera.so","sha256":digest}],
        "adapter_families":[{"id":"camera-test","device_kinds":["camera"],
          "transport":"unix_socket","description":"Test camera adapter"}],
        "node_types":[{"id":"eos.hardware.camera.device@v0.2","core":"eos",
          "layer":"hardware","group":"camera","kind":"camera","lifecycle":"adapter",
          "description":"Test camera","capabilities":["rgb8"],"settings":[],"inputs":[],
          "outputs":[{"name":"frame","data_type":"image_frame@v0.2","required":false,
            "description":"Observed camera frame","frequency_hz":30.0,"transport":frame_transport,
            "delivery_mode":"event","data_class":"image",
            "contract":{"owner":"eos","id":"image_frame@v0.2","version":"0.2"}}],
          "implementation":{"kind":"native_plugin","abi":"isos_node_v2","library":"lib/camera.so"}}]
      })
      .to_string(),
    )
    .unwrap();
    let registry = NodeTypeRegistry::with_contribution_files(&[sidecar]).unwrap();
    let mut graph = serde_json::to_value(demo_profile()).unwrap();
    graph["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"] = json!({"nodes":{
      "front":{"id":"front","node_type":"eos.hardware.camera.device@v0.2",
        "package":pin,"label":"Front camera","description":"Test source",
        "frame_id":"camera","capabilities":["rgb8"],
        "adapter":{"type":"camera-test","package":pin,"settings":{"service_instance":"front"}},
        "health_timing":{"poll_interval_ms":100,"stale_after_ms":500},
        "observation":{"max_rate_hz":30,"max_payload_bytes":12000000},"movement":null}
    }});
    graph["cores"]["eos"]["layers"]["interface"]["groups"]["egress"]["nodes"]["vision"] = json!({"id":"vision","node_type":"eos.interface.egress.vision_flow@v0.2",
        "label":"Vision","description":"Test bridge",
        "inputs":{"frame":{"node":"eos/hardware/camera/front","output":"frame"}}});
    let profile = Profile::from_json_with_registry(&graph.to_string(), &registry)
      .map_err(|error| error.to_string())?;
    Ok((profile, registry, dir))
  }
  #[test]
  fn mixed_camera_frame_targets_owner_bridge() {
    let (profile, registry, _dir) = mixed_camera_profile("in_process").unwrap();
    let plan = route_plan(&profile, &registry).unwrap();
    assert_eq!(plan.mode, ExecutionMode::Mixed);
    assert_eq!(
      plan.workers["eos/hardware/camera/front"],
      WorkerKind::Native
    );
    assert_eq!(
      plan.workers["eos/interface/egress/vision"],
      WorkerKind::OwnerBridge
    );
    assert!(plan.links.iter().any(|link| {
      link.source == "eos/hardware/camera/front"
        && link.output == "frame"
        && link.target == "eos/interface/egress/vision"
        && link.input == "frame"
        && link.data_type == "image_frame@v0.2"
    }));
  }
  #[test]
  fn mixed_camera_rejects_wrong_frame_transport() {
    let error = mixed_camera_profile("unix_socket").unwrap_err();
    assert!(error.contains("transport"), "{error}");
  }
  #[test]
  fn runtime_admission_keeps_demo_nodes_on_demo_worker() {
    let registry = NodeTypeRegistry::builtin();
    let mut profile = demo_profile();
    assert_eq!(
      execution_mode(&profile, &registry).unwrap(),
      ExecutionMode::Demo
    );
    let plan = route_plan(&profile, &registry).unwrap();
    assert_eq!(plan.workers.len(), 8);
    assert_eq!(plan.links.len(), 6);
    assert!(plan
      .workers
      .values()
      .all(|worker| *worker == WorkerKind::Demo));
    let node = profile
      .cores
      .values_mut()
      .flat_map(|core| core.layers.values_mut())
      .flat_map(|layer| layer.groups.values_mut())
      .flat_map(|group| group.nodes.values_mut())
      .find(|node| node.node_type == "eos.skill.demo.arm_status@v0.2")
      .unwrap();
    node.package =
      Some(serde_json::from_value(json!({"id":"community/extra","version":"0.2.0"})).unwrap());
    assert!(execution_mode(&profile, &registry)
      .unwrap_err()
      .starts_with("builtin_demo_package_forbidden"));
  }
  #[test]
  fn command_route_requires_two_demo_hosts_even_when_both_are_absent() {
    assert!(command_route_allowed(
      Some(&WorkerKind::Demo),
      Some(&WorkerKind::Demo)
    ));
    assert!(!command_route_allowed(None, None));
    assert!(!command_route_allowed(
      Some(&WorkerKind::Native),
      Some(&WorkerKind::Native)
    ));
    assert!(!command_route_allowed(
      Some(&WorkerKind::Demo),
      Some(&WorkerKind::OwnerBridge)
    ));
  }
  #[test]
  fn demo_worker_has_no_generic_cross_host_ingress() {
    assert!(!worker_link_supported(WorkerKind::Native, WorkerKind::Demo));
    assert!(!worker_link_supported(
      WorkerKind::OwnerBridge,
      WorkerKind::Demo
    ));
    assert!(worker_link_supported(WorkerKind::Demo, WorkerKind::Native));
    assert!(worker_link_supported(
      WorkerKind::Native,
      WorkerKind::OwnerBridge
    ));
  }
  #[test]
  fn demo_worker_exposes_only_observer_output_across_hosts() {
    assert!(demo_cross_host_output_supported(
      "eos.hardware.mock.arm_observer@v0.2",
      "observation"
    ));
    assert!(!demo_cross_host_output_supported(
      "eos.hardware.mock.arm_output@v0.2",
      "observation"
    ));
    assert!(!demo_cross_host_output_supported(
      "eos.interface.egress.arm_status@v0.2",
      "observation"
    ));
  }
  #[test]
  fn general_hardware_declaration_cannot_report_executable_readiness() {
    let registry = NodeTypeRegistry::builtin();
    let mut profile = demo_profile();
    for node in profile
      .cores
      .values_mut()
      .flat_map(|core| core.layers.values_mut())
      .flat_map(|layer| layer.groups.values_mut())
      .flat_map(|group| group.nodes.values_mut())
    {
      node.node_type = "eos.hardware.base.device@v0.2".into();
    }
    assert!(execution_mode(&profile, &registry).is_err());
  }
  #[test]
  fn runtime_admission_rejects_partial_demo_and_unimplemented_interface() {
    let registry = NodeTypeRegistry::builtin();
    let mut partial = demo_profile();
    for node in partial
      .cores
      .values_mut()
      .flat_map(|core| core.layers.values_mut())
      .flat_map(|layer| layer.groups.values_mut())
      .flat_map(|group| group.nodes.values_mut())
    {
      if node.node_type == "eos.skill.demo.arm_status@v0.2" {
        node.node_type = "eos.interface.egress.arm_status@v0.2".into();
      }
    }
    assert!(execution_mode(&partial, &registry)
      .unwrap_err()
      .starts_with("unsupported_demo_topology"));
    let mut interface_only = demo_profile();
    for group in interface_only
      .cores
      .values_mut()
      .flat_map(|core| core.layers.values_mut())
      .flat_map(|layer| layer.groups.values_mut())
    {
      group
        .nodes
        .retain(|_, node| node.node_type == "eos.interface.egress.arm_status@v0.2");
    }
    assert!(execution_mode(&interface_only, &registry)
      .unwrap_err()
      .starts_with("unsupported_demo_topology"));
  }
  #[test]
  fn catalog_includes_demo_but_not_package_hardware_types() {
    let reply = handle(
      &json!({"schema_version":3,"core":"eos","op":"catalog"}),
      &NodeTypeRegistry::builtin(),
    );
    assert_eq!(reply["catalog"]["types"].as_array().unwrap().len(), 17);
    assert_eq!(
      reply["catalog"]["direct_only_types"]
        .as_array()
        .unwrap()
        .len(),
      7
    );
    assert!(reply["catalog"]["types"]
      .as_array()
      .unwrap()
      .iter()
      .all(|node| !node["id"]
        .as_str()
        .unwrap_or("")
        .starts_with("eos.hardware.")
        || node["id"]
          .as_str()
          .unwrap_or("")
          .starts_with("eos.hardware.mock.")));
    assert!(reply["catalog"]["data_types"].as_array().unwrap().len() >= 8);
    let arm = reply["catalog"]["types"]
      .as_array()
      .unwrap()
      .iter()
      .find(|node| node["id"] == "eos.hardware.mock.arm@v0.2")
      .unwrap();
    assert_eq!(arm["settings"][0]["default"], 10);
    assert_eq!(arm["settings"][1]["default"], 50);
    assert!(reply["catalog"]["types"]
      .as_array()
      .unwrap()
      .iter()
      .all(|node| node["id"] != "eos.hardware.camera.device@v0.2"));
  }
  #[test]
  fn old_projection_is_rejected() {
    let reply = handle(
      &json!({"schema_version":3,"core":"eos","op":"validate","revision":"a","endpoints":{},"packages":[],
      "graph":{"version":"0.2.0","cores":{"isos":{"layers":{}}}}}),
      &NodeTypeRegistry::builtin(),
    );
    assert_eq!(reply["ok"], false);
    assert_eq!(reply["diagnostics"][0]["code"], "invalid_graph");
  }
  #[test]
  fn incomplete_hardware_draft_still_receives_adapter_choices() {
    let reply = handle(
      &json!({"schema_version":3,"core":"eos","op":"options",
      "revision":"a","endpoints":{},"packages":[],"graph":{"version":"0.2.1",
      "cores":{"eos":{"layers":{"hardware":{"groups":{"mock":{"nodes":{"arm":{
      "id":"arm","node_type":"eos.hardware.mock.arm@v0.2"}}}}}}}}}}),
      &NodeTypeRegistry::builtin(),
    );
    assert_eq!(reply["ok"], true);
    assert_eq!(
      reply["catalog"]["setting_options"][0]["setting"],
      "adapter.type"
    );
    assert_eq!(
      reply["catalog"]["setting_options"][0]["options"][0]["value"],
      "mock-arm"
    );
    assert_eq!(
      reply["catalog"]["setting_options"][0]["selected"],
      Value::Null
    );
  }
}
