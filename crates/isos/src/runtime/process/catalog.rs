//! Selected node and adapter catalogs from the verified graph and package pins.

use super::*;
use std::time::Instant;

pub fn catalog_at(paths: &Paths) -> Result<Value> {
  catalog_with_sidecars(packages::node_type_sidecars(paths)?)
}

/// Built-in EOS registry without installed package contributions. Option
/// projection resolves package selectors and selected EI stages from the draft.
pub fn catalog_base() -> Result<Value> {
  catalog_with_sidecars(Vec::new())
}

/// Resolve only the package process pins used by an editable draft. A
/// missing or changed pin is omitted so node-options can show a repair row.
/// Installed releases outside this graph cannot contribute selector metadata.
pub fn catalog_for_option_graph(paths: &Paths, graph: &Value) -> Result<Value> {
  ensure!(
    graph["version"] == "0.2.1",
    "node graph must be version 0.2.1"
  );
  let mut sidecars = BTreeSet::new();
  let mut seen_pins = BTreeSet::new();
  let mut verification_ms = 0_u64;
  let mut verification_count = 0_u64;
  for node in graph["cores"]
    .as_object()
    .into_iter()
    .flat_map(|cores| cores.values())
    .flat_map(|core| {
      core["layers"]
        .as_object()
        .into_iter()
        .flat_map(|layers| layers.values())
    })
    .flat_map(|layer| {
      layer["groups"]
        .as_object()
        .into_iter()
        .flat_map(|groups| groups.values())
    })
    .flat_map(|group| {
      group["nodes"]
        .as_object()
        .into_iter()
        .flat_map(|nodes| nodes.values())
    })
  {
    for package in [node.get("package"), node.pointer("/adapter/package")]
      .into_iter()
      .flatten()
    {
      if let (Some(id), Some(version)) = (package["id"].as_str(), package["version"].as_str()) {
        // Every exact pin is still verified once per request, even when many nodes use it.
        if !seen_pins.insert((id.to_owned(), version.to_owned())) {
          continue;
        }
        let started = Instant::now();
        let verified = packages::verify(paths, id, version);
        verification_ms += started.elapsed().as_millis() as u64;
        verification_count += 1;
        if let Ok(verified) = verified
          && let Some(sidecar) = verified.sidecar
        {
          sidecars.insert(sidecar);
        }
      }
    }
  }
  tracing::debug!(
    event_code = "node_options_stage",
    stage = "package_verify",
    elapsed_ms = verification_ms,
    count = verification_count
  );
  let catalog = match sidecars.is_empty() {
    false => catalog_with_sidecars(sidecars.into_iter().collect())?,
    true => catalog_base()?,
  };
  with_ei_stage_catalog(
    paths,
    graph,
    catalog,
    graph_contract::has_unified_ei_layers(graph),
    None,
    true,
  )
}

pub fn catalog_for_graph(paths: &Paths, graph: &Value) -> Result<Value> {
  let verified = packages::verify_graph_packages(paths, graph)?;
  // EOS currently keys its runtime registry by type ID. Two selected releases
  // with the same contributed EOS ID cannot both be loaded into one owner.
  let mut eos_types = BTreeMap::new();
  for package in &verified {
    if let Some(contributions) = &package.release.node_contributions {
      for node_type in &contributions.node_types {
        if node_type["core"] != "eos" {
          continue;
        }
        let id = node_type["id"]
          .as_str()
          .context("EOS contribution lacks type ID")?;
        let pin = format!("{}@{}", package.release.id, package.release.version);
        if let Some(previous) = eos_types.insert(id, pin.clone()) {
          ensure!(
            previous == pin,
            "selected EOS node type {id} conflicts between {previous} and {pin}; select one package release for this type ID"
          );
        }
      }
    }
  }
  let sidecars = verified
    .iter()
    .filter_map(|package| package.sidecar.clone())
    .collect();
  let catalog = catalog_with_sidecars(sidecars)?;
  with_ei_stage_catalog(
    paths,
    graph,
    catalog,
    graph_contract::has_unified_ei_layers(graph),
    Some(&verified),
    false,
  )
}

fn with_ei_stage_catalog(
  paths: &Paths,
  graph: &Value,
  mut catalog: Value,
  include: bool,
  verified: Option<&[packages::VerifiedPackage]>,
  preview: bool,
) -> Result<Value> {
  if graph_contract::uses_revisioned_generation(graph) && include {
    let bytes = serde_json::to_vec(graph)?;
    let revision = graph_contract::revision(&bytes);
    let reply = if preview {
      crate::runtime::owners::core_setup::preview_catalog(paths, "default", "ei", graph, &revision)?
    } else if let Some(verified) = verified {
      crate::runtime::owners::core_setup::catalog_with_verified_packages(
        paths, "default", "ei", graph, &revision, verified,
      )?
    } else {
      crate::runtime::owners::core_setup::call(paths, "default", "ei", "catalog", graph, &revision)?
    };
    merge_ei_stage_catalog(&mut catalog, &reply)?;
    finalize_owner_routes(&mut catalog)?;
  }
  Ok(catalog)
}

fn finalize_owner_routes(catalog: &mut Value) -> Result<()> {
  let eos = catalog["port_routes"]
    .as_array()
    .cloned()
    .unwrap_or_default();
  let ei = catalog["ei_port_routes"]
    .as_array()
    .cloned()
    .unwrap_or_default();
  let types = catalog["types"]
    .as_array()
    .context("aggregate catalog lacks types")?;
  let core = |id: &Value| -> Option<&str> {
    types
      .iter()
      .find(|node| node["id"] == *id)
      .and_then(|node| node["core"].as_str())
  };
  let mut accepted = Vec::new();
  for route in eos.iter().chain(&ei) {
    let source = core(&route["source_type"]);
    let target = core(&route["target_type"]);
    let (Some(source), Some(target)) = (source, target) else {
      continue;
    };
    let claimed = |owner: &str| match owner {
      "eos" => eos.contains(route),
      "ei" => ei.contains(route),
      "isos" => {
        (source != "isos" || route["source_type"] == "isos.ui.button.trigger@v0.2")
          && (target != "isos"
            || matches!(
              route["target_type"].as_str(),
              Some("isos.ui.monitor.arm@v0.2" | "isos.ui.vision.monitor@v0.2")
            ))
      }
      _ => false,
    };
    if claimed(source) && claimed(target) && !accepted.contains(route) {
      accepted.push(route.clone());
    }
  }
  catalog["port_routes"] = Value::Array(accepted);
  let bridges = catalog["ei_bridge_adapters"]
    .as_array()
    .cloned()
    .unwrap_or_default();
  let target = catalog["bridge_adapters"]
    .as_array_mut()
    .context("aggregate catalog lacks bridges")?;
  for bridge in bridges {
    ensure!(
      bridge["core"] == "ei",
      "EI owner claimed another core's bridge"
    );
    if !target.contains(&bridge) {
      target.push(bridge);
    }
  }
  Ok(())
}

pub(crate) fn merge_ei_stage_catalog(catalog: &mut Value, reply: &Value) -> Result<()> {
  ensure!(reply["ok"] == true, "EI owner catalog rejected graph");
  let stages = reply["catalog"]["stage_types"]
    .as_array()
    .context("EI owner catalog lacks stage_types")?;
  let mut actual = BTreeSet::new();
  for stage in stages {
    let id = stage["id"].as_str().context("EI wrapper lacks id")?;
    ensure!(
      id.starts_with("ei.") && id.ends_with("@v0.2") && actual.insert(id),
      "EI owner catalog has unsupported or duplicate wrapper {id}"
    );
    let layer = stage["layer"].as_str().context("EI wrapper lacks layer")?;
    let group = stage["group"].as_str().context("EI wrapper lacks group")?;
    ensure!(
      stage["core"] == "ei" && id.starts_with(&format!("ei.{layer}.{group}.")),
      "EI wrapper {id} has invalid placement"
    );
    let mut normalized = stage.clone();
    normalized["settings"] = Value::Array(
      stage["settings_fields"]
        .as_array()
        .or_else(|| stage["settings"].as_array())
        .context("EI wrapper settings missing")?
        .clone(),
    );
    normalized
      .as_object_mut()
      .unwrap()
      .remove("settings_fields");
    crate::domain::registry::node_schema::validate_type_idl(&normalized)
      .with_context(|| format!("EI wrapper {id} violates shared node IDL"))?;
    let types = catalog["types"]
      .as_array_mut()
      .context("aggregate catalog lacks types")?;
    if let Some(existing) = types.iter().find(|item| item["id"] == id) {
      let mut expected = normalized.clone();
      expected["package"] = existing["package"].clone();
      ensure!(
        existing == &expected,
        "EI wrapper {id} disagrees with verified package declaration"
      );
    } else {
      types.push(normalized);
    }
  }
  if let Some(data_types) = reply["catalog"]["data_types"].as_array() {
    let catalog_data = catalog["data_types"]
      .as_array_mut()
      .context("aggregate catalog lacks data_types")?;
    for data_type in data_types {
      let id = data_type["id"].as_str().context("EI data type lacks id")?;
      ensure!(
        data_type["schema"].is_object(),
        "EI data type {id} lacks a schema"
      );
      if !catalog_data.iter().any(|item| item["id"] == id) {
        catalog_data.push(data_type.clone());
      }
    }
  }
  catalog["ei_port_routes"] = reply["catalog"]["port_routes"].clone();
  catalog["ei_bridge_adapters"] = reply["catalog"]["bridge_adapters"].clone();
  Ok(())
}

fn catalog_with_sidecars(sidecars: Vec<PathBuf>) -> Result<Value> {
  let program = catalog_program()?;
  let mut command = Command::new(&program);
  command.arg("catalog");
  let output = command
    .output()
    .with_context(|| format!("run EOS type catalog {}", Path::new(&program).display()))?;
  ensure!(
    output.status.success(),
    "EOS catalog failed: {}",
    String::from_utf8_lossy(&output.stderr).trim()
  );
  ensure!(
    output.stdout.len() <= MAX_GRAPH_BYTES as usize,
    "EOS catalog exceeds 1 MiB"
  );
  let mut catalog: Value = serde_json::from_slice(&output.stdout)?;
  ensure!(
    catalog.get("types").and_then(Value::as_array).is_some(),
    "EOS catalog has no types array"
  );
  let mut eos_routes = catalog["port_routes"]
    .as_array()
    .cloned()
    .unwrap_or_default();
  let mut eos_bridges = catalog["bridge_adapters"]
    .as_array()
    .cloned()
    .unwrap_or_default();
  let mut contributed = BTreeMap::new();
  let mut contributed_data = BTreeMap::new();
  let mut contributed_adapters = BTreeMap::new();
  for sidecar in sidecars {
    let value: Value = serde_json::from_slice(&fs::read(&sidecar)?)?;
    let package_id = value["package"]["id"]
      .as_str()
      .context("sidecar lacks package id")?;
    let package_version = value["package"]["version"]
      .as_str()
      .context("sidecar lacks package version")?;
    let admitted = Command::new(&program)
      .arg("catalog")
      .arg("--node-types")
      .arg(&sidecar)
      .output()
      .with_context(|| format!("catalog package sidecar {}", sidecar.display()))?;
    ensure!(
      admitted.status.success(),
      "EOS rejected package sidecar {}: {}",
      sidecar.display(),
      String::from_utf8_lossy(&admitted.stderr).trim()
    );
    let admitted: Value = serde_json::from_slice(&admitted.stdout)?;
    // The EOS owner also derives routes from verified contributed port declarations.
    for route in admitted["port_routes"].as_array().into_iter().flatten() {
      let eos_endpoint = route["source_type"]
        .as_str()
        .is_some_and(|id| id.starts_with("eos."))
        || route["target_type"]
          .as_str()
          .is_some_and(|id| id.starts_with("eos."));
      if !eos_endpoint {
        continue;
      }
      if !eos_routes.contains(route) {
        eos_routes.push(route.clone());
      }
    }
    for bridge in value["bridge_adapters"].as_array().into_iter().flatten() {
      if bridge["core"] != "eos" {
        continue;
      }
      ensure!(
        admitted["bridge_adapters"]
          .as_array()
          .is_some_and(|bridges| bridges.contains(bridge)),
        "EOS did not confirm package bridge"
      );
      if !eos_bridges.contains(bridge) {
        eos_bridges.push(bridge.clone());
      }
    }
    if let Some(types) = value["node_types"].as_array() {
      for node_type in types {
        let id = node_type["id"]
          .as_str()
          .context("contributed node type lacks id")?;
        let mut node_type = node_type.clone();
        node_type["package"] = value["package"].clone();
        ensure!(
          contributed
            .insert(
              (
                package_id.to_owned(),
                package_version.to_owned(),
                id.to_owned()
              ),
              node_type
            )
            .is_none(),
          "package release contributes duplicate node type {id}"
        );
      }
    }
    if let Some(data_types) = value["data_types"].as_array() {
      for data_type in data_types {
        let id = data_type["id"]
          .as_str()
          .context("contributed data type lacks id")?;
        let mut data_type = data_type.clone();
        data_type["package"] = value["package"].clone();
        ensure!(
          contributed_data
            .insert(
              (
                package_id.to_owned(),
                package_version.to_owned(),
                id.to_owned()
              ),
              data_type
            )
            .is_none(),
          "package release contributes duplicate data type {id}"
        );
      }
    }
    if let Some(adapters) = admitted["adapter_families"].as_array() {
      for adapter in adapters {
        let id = adapter["id"]
          .as_str()
          .context("contributed adapter family lacks id")?;
        if !value["adapter_families"]
          .as_array()
          .is_some_and(|families| families.iter().any(|family| family["id"] == id))
        {
          continue;
        }
        let mut adapter = adapter.clone();
        adapter["package"] = value["package"].clone();
        ensure!(
          contributed_adapters
            .insert(
              (
                package_id.to_owned(),
                package_version.to_owned(),
                id.to_owned()
              ),
              adapter
            )
            .is_none(),
          "installed packages contribute duplicate adapter family {id}"
        );
      }
    }
  }
  let types = catalog["types"].as_array_mut().unwrap();
  let button = serde_json::json!({
    "id":"isos.ui.button.trigger@v0.2",
    "description":"Run-fenced, general-purpose one-click activation",
    "core":"isos","layer":"ui","group":"button","kind":"software",
    "lifecycle":"internal","implementation":{"kind":"external","interface":"isos.ui.node.v1",
      "version":"1","lifecycle_adapter":"isos-workbench"},
    "capabilities":["bridge"],"interaction_roles":["publisher"],"settings":[],"inputs":[],
    "outputs":[{"name":"activation","data_type":"button_activation@v0.2","required":false,
      "description":"One activation event, without movement arguments",
      "transport":"unix_socket","delivery_mode":"event","cardinality":"one",
      "data_class":"event","contract":{"owner":"isos","id":"button-activation","version":"1"}}],
    "bridge":{"adapter":"unix_socket","inputs":[],"outputs":["activation"]}
  });
  crate::domain::registry::node_schema::validate_type(&button)?;
  types.push(button);
  let monitor = serde_json::json!({
    "id":"isos.ui.monitor.arm@v0.2",
    "description":"Read-only display of the run-fenced six-joint arm state",
    "core":"isos","layer":"ui","group":"monitor","kind":"software",
    "lifecycle":"internal","implementation":{"kind":"external","interface":"isos.ui.node.v1",
      "version":"1","lifecycle_adapter":"isos-workbench"},
    "capabilities":["bridge"],"interaction_roles":["subscriber"],
    "settings":[{"name":"refresh_rate_hz","data_type":"number","required":false,
      "default":5,"min":1,"max":5,"unit":"Hz",
      "description":"Maximum arm monitor display refresh rate"}],"outputs":[],
    "inputs":[{"name":"observation","data_type":"arm_monitor_state@v0.2","required":true,
      "description":"Latest state from the declared EI monitor path",
      "transport":"unix_socket","delivery_mode":"event","cardinality":"one",
      "data_class":"observation","contract":{"owner":"ei","id":"arm-monitor-to-isos","version":"1"}}],
    "bridge":{"adapter":"unix_socket","inputs":["observation"],"outputs":[]}
  });
  crate::domain::registry::node_schema::validate_type(&monitor)?;
  types.push(monitor);
  let vision = serde_json::json!({
    "id":"isos.ui.vision.monitor@v0.2",
    "description":"Displays a run-fenced camera vision flow",
    "core":"isos","layer":"ui","group":"vision","kind":"software",
    "lifecycle":"internal","implementation":{"kind":"external","interface":"isos.ui.node.v1",
      "version":"1","lifecycle_adapter":"isos-workbench"},
    "capabilities":["bridge"],"interaction_roles":["subscriber"],"settings":[],"outputs":[],
    "inputs":[{"name":"frame","data_type":"image_frame@v0.2","required":true,
      "description":"Frame from the declared EOS vision transfer",
      "transport":"unix_socket","delivery_mode":"stream","cardinality":"one",
      "data_class":"image","contract":{"owner":"eos","id":"vision-flow-to-isos","version":"1"}}],
    "bridge":{"adapter":"unix_socket","inputs":["frame"],"outputs":[]}
  });
  crate::domain::registry::node_schema::validate_type(&vision)?;
  types.push(vision);
  let builtin: BTreeSet<_> = types
    .iter()
    .filter_map(|value| value["id"].as_str().map(str::to_owned))
    .collect();
  for ((_, _, id), node_type) in contributed {
    ensure!(
      !builtin.contains(&id),
      "package node type {id} conflicts with built-in type"
    );
    types.push(node_type);
  }
  for bridge in [
    serde_json::json!({"core":"isos","adapter":"unix_socket","node_type":"isos.ui.button.trigger@v0.2"}),
    serde_json::json!({"core":"isos","adapter":"unix_socket","node_type":"isos.ui.monitor.arm@v0.2"}),
    serde_json::json!({"core":"isos","adapter":"unix_socket","node_type":"isos.ui.vision.monitor@v0.2"}),
  ] {
    if !eos_bridges.contains(&bridge) {
      eos_bridges.push(bridge);
    }
  }
  catalog["bridge_adapters"] = Value::Array(eos_bridges);
  catalog["port_routes"] = Value::Array(eos_routes);
  if catalog["types"].as_array().is_some_and(|types| {
    types
      .iter()
      .any(|entry| entry["id"] == "eos.interface.egress.vision_flow@v0.2")
  }) {
    catalog["port_routes"]
      .as_array_mut()
      .unwrap()
      .push(serde_json::json!({
        "source_type":"eos.interface.egress.vision_flow@v0.2","output":"frame",
        "target_type":"isos.ui.vision.monitor@v0.2","input":"frame",
        "transport":"unix_socket","delivery_mode":"stream","data_type":"image_frame@v0.2",
        "contract":{"owner":"eos","id":"vision-flow-to-isos","version":"1"}
      }));
  }
  if !catalog["data_types"].is_array() {
    catalog["data_types"] = serde_json::json!([]);
  }
  let data_types = catalog["data_types"].as_array_mut().unwrap();
  let builtin_data: BTreeSet<_> = data_types
    .iter()
    .filter_map(|value| value["id"].as_str().map(str::to_owned))
    .collect();
  for ((_, _, id), data_type) in contributed_data {
    ensure!(
      !builtin_data.contains(&id),
      "package data type {id} conflicts with built-in type"
    );
    data_types.push(data_type);
  }
  if !catalog["adapter_families"].is_array() {
    catalog["adapter_families"] = serde_json::json!([]);
  }
  let families = catalog["adapter_families"].as_array_mut().unwrap();
  let builtin_families: BTreeSet<_> = families
    .iter()
    .filter_map(|value| value["id"].as_str().map(str::to_owned))
    .collect();
  for ((_, _, id), family) in contributed_adapters {
    ensure!(
      !builtin_families.contains(&id),
      "package adapter family {id} conflicts with built-in family"
    );
    families.push(family);
  }
  Ok(catalog)
}

fn catalog_program() -> Result<PathBuf> {
  #[cfg(test)]
  if let Some(path) = std::env::var_os("ISOS_TEST_CATALOG_PROFILE") {
    return checked_test_catalog_program(PathBuf::from(path));
  }
  core_bins::resolve("aether-profile")
}

#[cfg(test)]
pub(super) fn checked_test_catalog_program(path: PathBuf) -> Result<PathBuf> {
  ensure!(
    path.is_absolute(),
    "test EOS catalog profile must be absolute"
  );
  ensure!(
    fs::metadata(&path)
      .is_ok_and(|metadata| { metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 }),
    "test EOS catalog profile is missing or not executable"
  );
  Ok(path)
}

#[cfg(test)]
mod ei_stage_catalog_tests {
  use super::*;
  use serde_json::json;

  fn ei_wrapper(id: &str, layer: &str, group: &str) -> Value {
    let first = layer == "io";
    let last = layer == "governance";
    let monitor = id.ends_with(".arm_status@v0.2");
    let port = |transport: &str, owner: &str, contract: &str, required: bool| {
      json!({
        "name":if monitor {"observation"} else {"activation"},
        "description":if monitor {"Read-only six-joint arm state."} else {"General button activation event."},
        "data_type":if monitor {"arm_monitor_state@v0.2"} else {"button_activation@v0.2"},
        "data_class":if monitor {"observation"} else {"event"},
        "transport":transport,"delivery_mode":"event","cardinality":"one",
        "required":required,"contract":{"owner":owner,"id":contract,"version":"1"}
      })
    };
    json!({
      "id":id,"core":"ei","layer":layer,"group":group,
      "description":if monitor {"Pass a fenced read-only mock-arm observation toward IsOS."} else {"Pass one activation through the EI input, intent, and safety path."},
      "kind":"software","lifecycle":"internal","capabilities":if first || last { json!(["bridge"]) } else { json!([]) },
      "settings_fields":if monitor && layer == "governance" { json!([{"name":"forward_rate_hz","data_type":"number","required":false,"default":20,"min":1,"max":20}]) } else { json!([]) },
      "service_inputs":{},
      "inputs":[port(if first {"unix_socket"} else {"in_process"},if first {if monitor {"eos"} else {"isos"}} else {"ei"},if first {if monitor {"arm-monitor-to-ei"} else {"button-activation"}} else {if monitor {"arm-monitor-local"} else {"activation-local"}},true)],
      "outputs":[port(if last {"unix_socket"} else {"in_process"},"ei",if last {if monitor {"arm-monitor-to-isos"} else {"activation-to-eos"}} else {if monitor {"arm-monitor-local"} else {"activation-local"}},false)],
      "bridge":if first || last { json!({"adapter":"unix_socket","inputs":if first { json!([if monitor {"observation"} else {"activation"}]) } else { json!([]) },"outputs":if last { json!([if monitor {"observation"} else {"activation"}]) } else { json!([]) }}) } else { Value::Null }
    })
  }

  fn ei_pipeline(id: &str, layer: &str, group: &str) -> Value {
    json!({
      "id":id,"core":"ei","layer":layer,"group":group,
      "description":"Full cognition pipeline stage.",
      "kind":"software","lifecycle":"internal","capabilities":[],
      "settings_fields":[],"service_inputs":{},
      "inputs":[],"outputs":[],"bridge":Value::Null
    })
  }

  fn ei_reply() -> Value {
    json!({"ok":true,"catalog":{"stage_types":[
      ei_wrapper("ei.io.input.trigger@v0.2","io","input"),
      ei_wrapper("ei.cognition.intent.context@v0.2","cognition","intent"),
      ei_wrapper("ei.cognition.intent.forwarder@v0.2","cognition","intent"),
      ei_wrapper("ei.governance.safety.gate@v0.2","governance","safety")
      ,ei_wrapper("ei.io.input.arm_status@v0.2","io","input")
      ,ei_wrapper("ei.cognition.state.arm_status@v0.2","cognition","state")
      ,ei_wrapper("ei.governance.monitor.arm_status@v0.2","governance","monitor")
      ,ei_pipeline("ei.io.link.gateway@v0.2","io","link")
      ,ei_pipeline("ei.evidence.perception.admission@v0.2","evidence","perception")
      ,ei_pipeline("ei.cognition.context.builder@v0.2","cognition","context")
      ,ei_pipeline("ei.cognition.missions.queue@v0.2","cognition","missions")
      ,ei_pipeline("ei.cognition.decision.planner@v0.2","cognition","decision")
      ,ei_pipeline("ei.governance.monitor.health@v0.2","governance","monitor")
    ],"data_types":[],"routed_edges":[]}})
  }

  #[test]
  fn demo_fixture_passes_selected_catalog_admission() {
    let path =
      Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/graph/demo-only.nodes.json");
    let graph: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let mut catalog = catalog_base().unwrap();
    merge_ei_stage_catalog(&mut catalog, &ei_reply()).unwrap();
    assert!(
      graph_contract::admission_diagnostic(&graph, Some(&catalog)).is_none(),
      "{:#?}",
      graph_contract::admission_diagnostic(&graph, Some(&catalog))
    );
  }

  #[test]
  fn builtin_catalog_contains_isolated_eos_library_without_routes() {
    let catalog = catalog_base().unwrap();
    let actual: BTreeSet<_> = catalog["types"]
      .as_array()
      .unwrap()
      .iter()
      .map(|entry| entry["id"].as_str().unwrap())
      .collect();
    let expected: BTreeSet<_> = [
      "isos.ui.button.trigger@v0.2",
      "isos.ui.monitor.arm@v0.2",
      "isos.ui.vision.monitor@v0.2",
      "eos.interface.ingress.trigger@v0.2",
      "eos.skill.demo.joint_delta@v0.2",
      "eos.dispatcher.arm.delta@v0.2",
      "eos.hardware.mock.arm@v0.2",
      "eos.hardware.mock.arm_output@v0.2",
      "eos.hardware.mock.arm_observer@v0.2",
      "eos.dispatcher.arm.monitor@v0.2",
      "eos.skill.demo.arm_status@v0.2",
      "eos.interface.egress.arm_status@v0.2",
      "eos.interface.egress.vision_flow@v0.2",
      "eos.hardware.mock.camera@v0.2",
      "eos.perception.physical.features@v0.2",
      "eos.dispatcher.command.binding@v0.2",
      "eos.safety.device.guard@v0.2",
      "eos.safety.device.estop@v0.2",
      "eos.evidence.episode.recorder@v0.2",
      "eos.dataset.export.manifest@v0.2",
    ]
    .into_iter()
    .collect();
    assert_eq!(actual, expected);
    for id in [
      "eos.hardware.mock.camera@v0.2",
      "eos.perception.physical.features@v0.2",
      "eos.dispatcher.command.binding@v0.2",
      "eos.safety.device.guard@v0.2",
      "eos.safety.device.estop@v0.2",
      "eos.evidence.episode.recorder@v0.2",
      "eos.dataset.export.manifest@v0.2",
    ] {
      let descriptor = catalog["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap();
      assert!(
        descriptor["inputs"]
          .as_array()
          .unwrap()
          .iter()
          .all(|port| port["required"] == false)
      );
      assert!(
        !catalog["port_routes"]
          .as_array()
          .unwrap()
          .iter()
          .any(|route| route["source_type"] == id || route["target_type"] == id)
      );
    }
    assert!(
      catalog["data_types"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["id"] == "arm_joint_delta@v0.2")
    );
    assert_eq!(
      catalog["types"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["deprecated"] == true)
        .count(),
      0
    );
  }

  #[test]
  fn ei_wrapper_catalog_completes_three_core_types() {
    let mut catalog = catalog_base().unwrap();
    merge_ei_stage_catalog(&mut catalog, &ei_reply()).unwrap();
    let actual: BTreeSet<_> = catalog["types"]
      .as_array()
      .unwrap()
      .iter()
      .map(|entry| entry["id"].as_str().unwrap())
      .collect();
    assert_eq!(actual.len(), 33);
    assert!(actual.contains("isos.ui.vision.monitor@v0.2"));
    assert!(actual.contains("ei.governance.safety.gate@v0.2"));
    assert!(actual.contains("ei.io.input.trigger@v0.2"));
    assert_eq!(
      catalog["types"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["deprecated"] == true)
        .count(),
      0
    );
    assert!(catalog["ei_port_routes"].is_null());
  }

  #[test]
  fn ei_wrapper_catalog_allows_partial_catalog_but_rejects_duplicates() {
    let mut missing = ei_reply();
    missing["catalog"]["stage_types"]
      .as_array_mut()
      .unwrap()
      .pop();
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &missing).is_ok());
    let mut duplicate = ei_reply();
    duplicate["catalog"]["stage_types"][1] = duplicate["catalog"]["stage_types"][0].clone();
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &duplicate).is_err());
  }

  #[test]
  fn ei_wrapper_catalog_rejects_wrong_placement_and_accepts_new_owner_type() {
    let mut wrong = ei_reply();
    wrong["catalog"]["stage_types"][0]["group"] = json!("wrong");
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &wrong).is_err());
    let mut unknown = ei_reply();
    unknown["catalog"]["stage_types"][0]["id"] = json!("ei.io.input.unknown@v0.2");
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &unknown).is_ok());
  }

  #[test]
  fn cross_core_routes_require_both_owner_claims() {
    let route =
      json!({"source_type":"ei.io.input.custom@v0.2","target_type":"eos.io.output.custom@v0.2"});
    let mut catalog = json!({
      "types":[
        {"id":"ei.io.input.custom@v0.2","core":"ei"},
        {"id":"eos.io.output.custom@v0.2","core":"eos"}
      ],
      "port_routes":[route],
      "ei_port_routes":[],
      "bridge_adapters":[],
      "ei_bridge_adapters":[]
    });
    finalize_owner_routes(&mut catalog).unwrap();
    assert!(catalog["port_routes"].as_array().unwrap().is_empty());
    catalog["port_routes"] = json!([route]);
    catalog["ei_port_routes"] = json!([route]);
    finalize_owner_routes(&mut catalog).unwrap();
    assert_eq!(catalog["port_routes"], json!([route]));
  }

  #[test]
  fn unimplemented_isos_endpoint_cannot_claim_route() {
    let route =
      json!({"source_type":"isos.ui.custom.source@v0.2","target_type":"ei.io.input.custom@v0.2"});
    let mut catalog = json!({
      "types":[
        {"id":"isos.ui.custom.source@v0.2","core":"isos"},
        {"id":"ei.io.input.custom@v0.2","core":"ei"}
      ],
      "port_routes":[],
      "ei_port_routes":[route],
      "bridge_adapters":[],
      "ei_bridge_adapters":[]
    });
    finalize_owner_routes(&mut catalog).unwrap();
    assert!(catalog["port_routes"].as_array().unwrap().is_empty());
  }
}
