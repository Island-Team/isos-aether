//! Selected node and adapter catalogs from the verified graph and package pins.

use super::*;

pub fn catalog() -> Result<Value> {
  catalog_at(&Paths::discover()?)
}

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
    graph["version"] == "0.2.0",
    "node graph must be version 0.2.0"
  );
  let mut sidecars = BTreeSet::new();
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
    if let (Some(id), Some(version)) = (
      node["package"]["id"].as_str(),
      node["package"]["version"].as_str(),
    ) && let Ok(verified) = packages::verify(paths, id, version)
      && let Some(sidecar) = verified.sidecar
    {
      sidecars.insert(sidecar);
    }
  }
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
  }
  Ok(catalog)
}

pub(crate) fn merge_ei_stage_catalog(catalog: &mut Value, reply: &Value) -> Result<()> {
  ensure!(reply["ok"] == true, "EI owner catalog rejected graph");
  let stages = reply["catalog"]["stage_types"]
    .as_array()
    .context("EI owner catalog lacks stage_types")?;
  let expected: BTreeSet<&str> = [
    "ei.io.input.trigger@v0.2",
    "ei.cognition.intent.forwarder@v0.2",
    "ei.governance.safety.gate@v0.2",
    "ei.io.input.arm_status@v0.2",
    "ei.cognition.state.arm_status@v0.2",
    "ei.governance.monitor.arm_status@v0.2",
  ]
  .into_iter()
  .collect();
  let mut actual = BTreeSet::new();
  for stage in stages {
    let id = stage["id"].as_str().context("EI wrapper lacks id")?;
    ensure!(
      expected.contains(id) && actual.insert(id),
      "EI owner catalog has unsupported or duplicate wrapper {id}"
    );
    let (layer, group) = match id {
      "ei.io.input.trigger@v0.2" => ("io", "input"),
      "ei.cognition.intent.forwarder@v0.2" => ("cognition", "intent"),
      "ei.governance.safety.gate@v0.2" => ("governance", "safety"),
      "ei.io.input.arm_status@v0.2" => ("io", "input"),
      "ei.cognition.state.arm_status@v0.2" => ("cognition", "state"),
      "ei.governance.monitor.arm_status@v0.2" => ("governance", "monitor"),
      _ => unreachable!(),
    };
    ensure!(
      stage["core"] == "ei" && stage["layer"] == layer && stage["group"] == group,
      "EI wrapper {id} has invalid placement"
    );
    let mut normalized = stage.clone();
    normalized["settings"] = Value::Array(
      stage["settings_fields"]
        .as_array()
        .context("EI wrapper settings_fields missing")?
        .clone(),
    );
    normalized
      .as_object_mut()
      .unwrap()
      .remove("settings_fields");
    crate::domain::registry::node_schema::validate_type_idl(&normalized)
      .with_context(|| format!("EI wrapper {id} violates shared node IDL"))?;
    ensure!(
      !catalog["types"]
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item["id"] == id)),
      "EI wrapper {id} conflicts with another owner"
    );
    catalog["types"]
      .as_array_mut()
      .context("aggregate catalog lacks types")?
      .push(normalized);
  }
  ensure!(actual == expected, "EI owner catalog is missing wrappers");
  ensure!(
    reply["catalog"]["routed_edges"] == serde_json::json!([]),
    "EI wrapper catalog must not declare runtime routes"
  );
  ensure!(
    reply["catalog"]["data_types"] == serde_json::json!([]),
    "EI wrapper catalog must not declare payload data types"
  );
  catalog["ei_routed_edges"] = serde_json::json!([]);
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
  let mut contributed = BTreeMap::new();
  let mut contributed_data = BTreeMap::new();
  let mut contributed_adapters = BTreeMap::new();
  for sidecar in sidecars {
    let value: Value = serde_json::from_slice(&fs::read(&sidecar)?)?;
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
    if let Some(types) = value["node_types"].as_array() {
      for node_type in types {
        let id = node_type["id"]
          .as_str()
          .context("contributed node type lacks id")?;
        let mut node_type = node_type.clone();
        node_type["package"] = value["package"].clone();
        ensure!(
          contributed.insert(id.to_owned(), node_type).is_none(),
          "installed packages contribute duplicate node type {id}; remove one package version"
        );
      }
    }
    if value["package"]["id"] == "island/mujoco-sim"
      && value["package"]["version"] == "0.1.1"
      && value["node_types"].as_array().is_some_and(|types| {
        types
          .iter()
          .any(|node| node["id"] == "eos.process.mujoco@v0.1")
      })
    {
      let id = "isos.eos_process.mujoco@v0.2";
      let mut alias = admitted["types"]
        .as_array()
        .and_then(|types| types.iter().find(|entry| entry["id"] == id))
        .cloned()
        .context("EOS omitted pinned MuJoCo single-core type")?;
      alias["package"] = value["package"].clone();
      ensure!(
        contributed.insert(id.into(), alias).is_none(),
        "duplicate pinned MuJoCo single-core type"
      );
    }
    if let Some(data_types) = value["data_types"].as_array() {
      for data_type in data_types {
        let id = data_type["id"]
          .as_str()
          .context("contributed data type lacks id")?;
        let mut data_type = data_type.clone();
        data_type["package"] = value["package"].clone();
        ensure!(
          contributed_data.insert(id.to_owned(), data_type).is_none(),
          "installed packages contribute duplicate data type {id}; remove one package version"
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
            .insert(id.to_owned(), adapter)
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
    "lifecycle":"internal","capabilities":["bridge"],"settings":[],"inputs":[],
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
    "lifecycle":"internal","capabilities":["bridge"],
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
  let builtin: BTreeSet<_> = types
    .iter()
    .filter_map(|value| value["id"].as_str().map(str::to_owned))
    .collect();
  for (id, node_type) in contributed {
    ensure!(
      !builtin.contains(&id),
      "package node type {id} conflicts with built-in type"
    );
    types.push(node_type);
  }
  catalog["bridge_adapters"] = serde_json::json!([
    {"core":"isos","adapter":"unix_socket","node_type":"isos.ui.button.trigger@v0.2"},
    {"core":"isos","adapter":"unix_socket","node_type":"isos.ui.monitor.arm@v0.2"},
    {"core":"ei","adapter":"unix_socket","node_type":"ei.io.input.trigger@v0.2"},
    {"core":"ei","adapter":"unix_socket","node_type":"ei.governance.safety.gate@v0.2"},
    {"core":"ei","adapter":"unix_socket","node_type":"ei.io.input.arm_status@v0.2"},
    {"core":"ei","adapter":"unix_socket","node_type":"ei.governance.monitor.arm_status@v0.2"},
    {"core":"eos","adapter":"unix_socket","node_type":"eos.interface.ingress.trigger@v0.2"},
    {"core":"eos","adapter":"unix_socket","node_type":"eos.interface.egress.arm_status@v0.2"}
  ]);
  let route = |source: &str,
               output: &str,
               target: &str,
               input: &str,
               transport: &str,
               data_type: &str,
               owner: &str,
               contract: &str| {
    serde_json::json!({"source_type":source,"output":output,"target_type":target,"input":input,
      "transport":transport,"delivery_mode":"event","data_type":data_type,
      "contract":{"owner":owner,"id":contract,"version":"1"}})
  };
  catalog["port_routes"] = serde_json::json!([
    route(
      "isos.ui.button.trigger@v0.2",
      "activation",
      "ei.io.input.trigger@v0.2",
      "activation",
      "unix_socket",
      "button_activation@v0.2",
      "isos",
      "button-activation"
    ),
    route(
      "ei.io.input.trigger@v0.2",
      "activation",
      "ei.cognition.intent.forwarder@v0.2",
      "activation",
      "in_process",
      "button_activation@v0.2",
      "ei",
      "activation-local"
    ),
    route(
      "ei.cognition.intent.forwarder@v0.2",
      "activation",
      "ei.governance.safety.gate@v0.2",
      "activation",
      "in_process",
      "button_activation@v0.2",
      "ei",
      "activation-local"
    ),
    route(
      "ei.governance.safety.gate@v0.2",
      "activation",
      "eos.interface.ingress.trigger@v0.2",
      "activation",
      "unix_socket",
      "button_activation@v0.2",
      "ei",
      "activation-to-eos"
    ),
    route(
      "eos.interface.ingress.trigger@v0.2",
      "activation",
      "eos.skill.demo.joint_delta@v0.2",
      "activation",
      "in_process",
      "button_activation@v0.2",
      "eos",
      "activation-local"
    ),
    route(
      "eos.skill.demo.joint_delta@v0.2",
      "delta",
      "eos.dispatcher.arm.delta@v0.2",
      "delta",
      "in_process",
      "arm_joint_delta@v0.2",
      "eos",
      "arm-delta"
    ),
    route(
      "eos.dispatcher.arm.delta@v0.2",
      "delta",
      "eos.hardware.mock.arm@v0.2",
      "delta",
      "in_process",
      "arm_joint_delta@v0.2",
      "eos",
      "arm-delta"
    ),
    route(
      "eos.hardware.mock.arm@v0.2",
      "observation",
      "eos.hardware.mock.arm_output@v0.2",
      "observation",
      "in_process",
      "arm_monitor_state@v0.2",
      "eos",
      "arm-monitor-local"
    ),
    route(
      "eos.hardware.mock.arm_observer@v0.2",
      "observation",
      "eos.dispatcher.arm.monitor@v0.2",
      "observation",
      "in_process",
      "arm_monitor_state@v0.2",
      "eos",
      "arm-monitor-local"
    ),
    route(
      "eos.hardware.mock.arm_output@v0.2",
      "observation",
      "eos.dispatcher.arm.monitor@v0.2",
      "observation",
      "in_process",
      "arm_monitor_state@v0.2",
      "eos",
      "arm-monitor-local"
    ),
    route(
      "eos.dispatcher.arm.monitor@v0.2",
      "observation",
      "eos.skill.demo.arm_status@v0.2",
      "observation",
      "in_process",
      "arm_monitor_state@v0.2",
      "eos",
      "arm-monitor-local"
    ),
    route(
      "eos.skill.demo.arm_status@v0.2",
      "observation",
      "eos.interface.egress.arm_status@v0.2",
      "observation",
      "in_process",
      "arm_monitor_state@v0.2",
      "eos",
      "arm-monitor-local"
    ),
    route(
      "eos.interface.egress.arm_status@v0.2",
      "observation",
      "ei.io.input.arm_status@v0.2",
      "observation",
      "unix_socket",
      "arm_monitor_state@v0.2",
      "eos",
      "arm-monitor-to-ei"
    ),
    route(
      "ei.io.input.arm_status@v0.2",
      "observation",
      "ei.cognition.state.arm_status@v0.2",
      "observation",
      "in_process",
      "arm_monitor_state@v0.2",
      "ei",
      "arm-monitor-local"
    ),
    route(
      "ei.cognition.state.arm_status@v0.2",
      "observation",
      "ei.governance.monitor.arm_status@v0.2",
      "observation",
      "in_process",
      "arm_monitor_state@v0.2",
      "ei",
      "arm-monitor-local"
    ),
    route(
      "ei.governance.monitor.arm_status@v0.2",
      "observation",
      "isos.ui.monitor.arm@v0.2",
      "observation",
      "unix_socket",
      "arm_monitor_state@v0.2",
      "ei",
      "arm-monitor-to-isos"
    )
  ]);
  if !catalog["data_types"].is_array() {
    catalog["data_types"] = serde_json::json!([]);
  }
  let data_types = catalog["data_types"].as_array_mut().unwrap();
  let builtin_data: BTreeSet<_> = data_types
    .iter()
    .filter_map(|value| value["id"].as_str().map(str::to_owned))
    .collect();
  for (id, data_type) in contributed_data {
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
  for (id, family) in contributed_adapters {
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

  fn ei_reply() -> Value {
    json!({"ok":true,"catalog":{"stage_types":[
      ei_wrapper("ei.io.input.trigger@v0.2","io","input"),
      ei_wrapper("ei.cognition.intent.forwarder@v0.2","cognition","intent"),
      ei_wrapper("ei.governance.safety.gate@v0.2","governance","safety")
      ,ei_wrapper("ei.io.input.arm_status@v0.2","io","input")
      ,ei_wrapper("ei.cognition.state.arm_status@v0.2","cognition","state")
      ,ei_wrapper("ei.governance.monitor.arm_status@v0.2","governance","monitor")
    ],"data_types":[],"routed_edges":[]}})
  }

  #[test]
  fn pooled_demo_passes_selected_catalog_admission() {
    let path =
      Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/graph-pool/demo.nodes.json");
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
  fn builtin_catalog_contains_only_isos_and_eos_wrappers() {
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
      "eos.interface.ingress.trigger@v0.2",
      "eos.skill.demo.joint_delta@v0.2",
      "eos.dispatcher.arm.delta@v0.2",
      "eos.hardware.mock.arm@v0.2",
      "eos.hardware.mock.arm_output@v0.2",
      "eos.hardware.mock.arm_observer@v0.2",
      "eos.dispatcher.arm.monitor@v0.2",
      "eos.skill.demo.arm_status@v0.2",
      "eos.interface.egress.arm_status@v0.2",
    ]
    .into_iter()
    .collect();
    assert_eq!(actual, expected);
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
    assert_eq!(actual.len(), 17);
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
    assert_eq!(catalog["ei_routed_edges"], json!([]));
  }

  #[test]
  fn ei_wrapper_catalog_rejects_missing_duplicate_and_routed_types() {
    let mut missing = ei_reply();
    missing["catalog"]["stage_types"]
      .as_array_mut()
      .unwrap()
      .pop();
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &missing).is_err());
    let mut duplicate = ei_reply();
    duplicate["catalog"]["stage_types"][1] = duplicate["catalog"]["stage_types"][0].clone();
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &duplicate).is_err());
    let mut routed = ei_reply();
    routed["catalog"]["routed_edges"] = json!([{"source_type":"ei.io.input.trigger@v0.2"}]);
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &routed).is_err());
    let mut payload = ei_reply();
    payload["catalog"]["data_types"] = json!([{"id":"old_payload"}]);
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &payload).is_err());
  }

  #[test]
  fn ei_wrapper_catalog_rejects_wrong_placement_and_identity() {
    let mut wrong = ei_reply();
    wrong["catalog"]["stage_types"][0]["group"] = json!("wrong");
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &wrong).is_err());
    let mut unknown = ei_reply();
    unknown["catalog"]["stage_types"][0]["id"] = json!("ei.io.input.unknown@v0.2");
    assert!(merge_ei_stage_catalog(&mut json!({"types":[],"data_types":[]}), &unknown).is_err());
  }
}
