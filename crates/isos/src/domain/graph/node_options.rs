//! Selector options derived from the EOS registry and the current draft graph.
use crate::{
  app::config::Paths, domain::graph::graph_contract, domain::launch as package_process,
  domain::packages, runtime::process,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn device_kind<'a>(catalog: &'a Value, node_type: &str) -> Option<&'a str> {
  catalog["types"].as_array()?.iter().find(|entry| {
    entry["id"] == node_type && entry["core"] == "isos" && entry["layer"] == "eos_hardware"
  })?["kind"]
    .as_str()
}

pub fn compose(graph: &Value, catalog: &Value) -> Result<Value> {
  ensure!(
    graph["version"] == "0.2.0",
    "node graph must be version 0.2.0"
  );
  let families = catalog["adapter_families"]
    .as_array()
    .context("catalog lacks adapter_families")?;
  let specs = catalog["primitive_specs"]
    .as_array()
    .context("catalog lacks primitive_specs")?;
  let mut adapter_families = Vec::new();
  for family in families {
    let compiled = family["compiled"].as_bool().unwrap_or(false);
    let reason = if compiled {
      Value::Null
    } else {
      family.get("reason").cloned().unwrap_or(json!(
        "adapter is not compiled into the selected EOS runtime"
      ))
    };
    adapter_families.push(json!({
            "id":family["id"],"device_kinds":family["device_kinds"],"package":family.get("package"),
            "compiled":compiled,"transport":family["transport"],"description":family["description"],
            "supported_primitives":family.get("supported_primitives"),"available":compiled,"reason":reason
        }));
  }
  let mut primitive_bindings = Vec::new();
  for (kind, node_type) in [
    ("base", "isos.eos_dispatcher.primitive_base@v0.2"),
    (
      "manipulator",
      "isos.eos_dispatcher.primitive_manipulator@v0.2",
    ),
  ] {
    let primitives: Vec<_> = specs.iter().filter(|spec| spec["device_class"] == kind).map(|spec| json!({
            "name":spec["name"],"version":spec["version"],"movement_type":spec["movement_type"],"description":spec["description"]
        })).collect();
    let available = catalog["types"]
      .as_array()
      .is_some_and(|types| types.iter().any(|entry| entry["id"] == node_type))
      && !primitives.is_empty();
    let movement_types: Vec<_> = primitives
      .iter()
      .map(|p| p["movement_type"].clone())
      .collect();
    primitive_bindings.push(json!({
            "node_type":node_type,"device_kind":kind,"movement_types":movement_types,
            "available":available,"reason":if available {Value::Null} else {json!("binding type or primitive implementation is unavailable")},"primitives":primitives
        }));
  }
  let mut targets = Vec::new();
  if let Some(cores) = graph["cores"].as_object() {
    for (core_id, core) in cores {
      if let Some(layers) = core["layers"].as_object() {
        for (layer_id, layer) in layers {
          if let Some(groups) = layer["groups"].as_object() {
            for (group_id, group) in groups {
              if let Some(nodes) = group["nodes"].as_object() {
                for (node_id, node) in nodes {
                  let Some(kind) = node["node_type"]
                    .as_str()
                    .and_then(|id| device_kind(catalog, id))
                  else {
                    continue;
                  };
                  let adapter_type = node["adapter"]["type"].as_str().unwrap_or("");
                  let family = adapter_families
                    .iter()
                    .find(|family| family["id"] == adapter_type);
                  let available = family.is_some_and(|family| {
                    family["available"] == true
                      && family["device_kinds"]
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|entry| entry == kind))
                  });
                  let movement_types: Vec<_> = specs
                    .iter()
                    .filter(|spec| {
                      if spec["device_class"] != kind {
                        return false;
                      }
                      family.is_none_or(|family| {
                        family["supported_primitives"]
                          .as_array()
                          .is_none_or(|supported| {
                            let identity = format!(
                              "{}@{}",
                              spec["name"].as_str().unwrap_or(""),
                              spec["version"].as_str().unwrap_or("")
                            );
                            supported.iter().any(|entry| entry == &identity)
                          })
                      })
                    })
                    .map(|spec| spec["movement_type"].clone())
                    .collect();
                  targets.push(json!({
                                        "path":format!("{core_id}/{layer_id}/{group_id}/{node_id}"),
                                        "node_type":node["node_type"],"device_kind":kind,"adapter_type":adapter_type,
                                        "movement_types":movement_types,"available":available,
                                        "reason":if available {Value::Null} else {json!("selected adapter is unavailable or incompatible with device kind")}
                                    }));
                }
              }
            }
          }
        }
      }
    }
  }
  Ok(
    json!({"adapter_families":adapter_families,"primitive_bindings":primitive_bindings,"targets":targets}),
  )
}

fn pointer_segment(segment: &str) -> String {
  segment.replace('~', "~0").replace('/', "~1")
}

fn stable_id(value: &str) -> bool {
  (1..=128).contains(&value.len())
    && value.split('-').all(|part| {
      !part.is_empty()
        && part
          .bytes()
          .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}

fn node_path(value: &str) -> bool {
  value.split('/').count() == 4
    && value.split('/').all(|part| {
      !part.is_empty()
        && part.len() <= 48
        && part
          .bytes()
          .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    })
}

fn option_error(code: &'static str, path: String, node: &Value) -> anyhow::Error {
  let pin = if path.contains("/adapter/") {
    &node["adapter"]["package"]
  } else {
    &node["package"]
  };
  packages::PackageFailure {
    code,
    path,
    stage: "admission",
    id: pin["id"].as_str().unwrap_or("").to_owned(),
    version: pin["version"].as_str().unwrap_or("").to_owned(),
  }
  .into()
}

fn group(
  node_path: &str,
  setting: &str,
  selected: Option<String>,
  mut options: Vec<Value>,
  absent_code: &str,
) -> Value {
  if let Some(id) = &selected
    && !options.iter().any(|row| row["value"] == *id)
  {
    options.push(json!({"value":id,"label":id,"description":"Saved value is not in the verified catalog",
            "available":false,"reason_code":absent_code,"reason":"Select a verified available value"}));
  }
  options.sort_by(|left, right| left["value"].as_str().cmp(&right["value"].as_str()));
  options.dedup_by(|left, right| left["value"] == right["value"]);
  json!({"node_path":node_path,"setting":setting,"selected":selected,"options":options})
}

fn reason(code: &str) -> String {
  match code {
    "package_missing_file" => "Install and verify the selected package version",
    "package_changed" => "Relink the changed package release",
    "package_host_input_unavailable" => "Provide the required package host input",
    "adapter_unavailable" => "Choose an EOS runtime with this adapter compiled",
    _ => "Select a verified available value",
  }
  .to_owned()
}

fn declared_process<'a>(
  verified: &'a packages::VerifiedPackage,
  node_type: &str,
) -> Option<&'a crate::domain::registry::release::ProcessDeclaration> {
  verified
    .release
    .node_contributions
    .as_ref()?
    .processes
    .iter()
    .find(|process| process.node_type == node_type)
}

fn resource_rows(
  verified: &packages::VerifiedPackage,
  process: &crate::domain::registry::release::ProcessDeclaration,
) -> Result<Vec<Value>> {
  package_process::option_resources(&verified.root, &verified.release, process)
}

fn host_input_unavailable(
  process: &crate::domain::registry::release::ProcessDeclaration,
  node: &Value,
) -> Option<&'static str> {
  process
    .host_inputs
    .iter()
    .find(|input| {
      input.required && {
        let value = node["settings"]["env"][&input.name]
          .as_str()
          .map(str::to_owned)
          .or_else(|| std::env::var(&input.name).ok());
        !value.is_some_and(|path| {
          std::path::Path::new(&path).is_absolute() && std::path::Path::new(&path).is_dir()
        })
      }
    })
    .map(|_| "package_host_input_unavailable")
}

fn check_selectors(definition: &Value, node: &Value, pointer: &str) -> Result<()> {
  if let Some(settings) = definition["settings"].as_array() {
    for setting in settings {
      if let Some(selector) = setting.get("selector") {
        let name = setting["name"].as_str().unwrap_or("");
        let package_selector = node["package"].is_object()
          && selector
            == &json!({"provider":"package","resource":name,"package_from":"node.package"});
        let graph_selector = selector["provider"] == "graph"
          && selector["resource"] == "node"
          && selector["filter"].as_object().is_some_and(|filter| {
            !filter.is_empty()
              && filter.keys().all(|key| {
                matches!(
                  key.as_str(),
                  "core" | "capability" | "output_data_type" | "output_transport"
                )
              })
              && filter.values().all(Value::is_string)
          });
        let ui_selector = selector == &json!({"provider":"ui","resource":"keys"});
        let owner_selector = selector["provider"] == "owner"
          && selector["resource"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
          && selector.get("package_from").is_none()
          && selector.get("filter").is_none();
        let allowed = package_selector || graph_selector || ui_selector || owner_selector;
        if !allowed {
          return Err(option_error(
            "node_option_selector_invalid",
            format!("{pointer}/settings/{}", pointer_segment(name)),
            node,
          ));
        }
        if let Some(value) = node["settings"].get(name)
          && !value.as_str().is_some_and(|value| {
            if package_selector {
              stable_id(value)
            } else if ui_selector {
              (1..=64).contains(&value.len())
                && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
            } else if owner_selector {
              (1..=128).contains(&value.len()) && !value.chars().any(char::is_control)
            } else {
              node_path(value)
            }
          })
        {
          return Err(option_error(
            "node_option_invalid_id",
            format!("{pointer}/settings/{}", pointer_segment(name)),
            node,
          ));
        }
      }
    }
  }
  Ok(())
}

fn graph_selector_rows(graph: &Value, catalog: &Value, filter: &Value) -> Vec<Value> {
  let mut rows = Vec::new();
  let Some(cores) = graph["cores"].as_object() else {
    return rows;
  };
  for (core_id, core) in cores {
    if filter["core"]
      .as_str()
      .is_some_and(|value| value != core_id)
    {
      continue;
    }
    let Some(layers) = core["layers"].as_object() else {
      continue;
    };
    for (layer_id, layer) in layers {
      let Some(groups) = layer["groups"].as_object() else {
        continue;
      };
      for (group_id, group) in groups {
        let Some(nodes) = group["nodes"].as_object() else {
          continue;
        };
        for (node_id, node) in nodes {
          let Some(definition) = catalog["types"]
            .as_array()
            .and_then(|types| types.iter().find(|entry| entry["id"] == node["node_type"]))
          else {
            continue;
          };
          if filter["capability"].as_str().is_some_and(|value| {
            !definition["capabilities"]
              .as_array()
              .is_some_and(|items| items.iter().any(|item| item == value))
          }) {
            continue;
          }
          if filter.get("output_data_type").is_some() || filter.get("output_transport").is_some() {
            let compatible = definition["outputs"].as_array().is_some_and(|ports| {
              ports.iter().any(|port| {
                filter["output_data_type"]
                  .as_str()
                  .is_none_or(|value| port["data_type"] == value)
                  && filter["output_transport"]
                    .as_str()
                    .is_none_or(|value| port["transport"] == value)
              })
            });
            if !compatible {
              continue;
            }
          }
          let path = format!("{core_id}/{layer_id}/{group_id}/{node_id}");
          rows.push(json!({"value":path,"label":path,"description":definition["description"],"available":true}));
        }
      }
    }
  }
  rows
}

/// Project selectors from the current exact draft and exact verified package pins.
pub fn compose_with_paths(paths: &Paths, graph: &Value, catalog: &Value) -> Result<Value> {
  if let Some(diagnostic) = graph_contract::admission_diagnostic(graph, Some(catalog)) {
    anyhow::bail!(diagnostic);
  }
  let mut data = compose(graph, catalog)?;
  let mut groups: BTreeMap<(String, String), Value> = BTreeMap::new();
  if graph["version"] == "0.2.0" {
    let mut nodes = Vec::new();
    if let Some(cores) = graph["cores"].as_object() {
      for (core_id, core) in cores {
        if let Some(layers) = core["layers"].as_object() {
          for (layer_id, layer) in layers {
            if let Some(families) = layer["groups"].as_object() {
              for (family_id, family) in families {
                if let Some(instances) = family["nodes"].as_object() {
                  for (node_id, node) in instances {
                    let short = format!("{core_id}/{layer_id}/{family_id}/{node_id}");
                    let pointer = format!(
                      "/cores/{}/layers/{}/groups/{}/nodes/{}",
                      pointer_segment(core_id),
                      pointer_segment(layer_id),
                      pointer_segment(family_id),
                      pointer_segment(node_id)
                    );
                    nodes.push((short, pointer, family_id.as_str(), node));
                  }
                }
              }
            }
          }
        }
      }
    }
    // The process type is package-contributed. A damaged exact pin still
    // produces a repairable unavailable selected row.
    for (short, pointer, _kind, node) in nodes {
      if let Some(definition) = catalog["types"]
        .as_array()
        .and_then(|types| types.iter().find(|entry| entry["id"] == node["node_type"]))
      {
        check_selectors(definition, node, &pointer)?;
        for setting in definition["settings"].as_array().into_iter().flatten() {
          let selector = &setting["selector"];
          if selector["provider"] != "graph" {
            continue;
          }
          let name = setting["name"].as_str().unwrap_or("");
          let selected = node["settings"][name].as_str().map(str::to_owned);
          groups.insert(
            (short.clone(), name.to_owned()),
            group(
              &short,
              name,
              selected,
              graph_selector_rows(graph, catalog, &selector["filter"]),
              "graph_node_unavailable",
            ),
          );
        }
      }
      if let (Some(node_type), Some(id), Some(version)) = (
        node["node_type"].as_str(),
        node["package"]["id"].as_str(),
        node["package"]["version"].as_str(),
      ) && let Ok(verified) = packages::verify(paths, id, version)
        && let Some(process) = declared_process(&verified, node_type)
      {
        let resources = resource_rows(&verified, process)?;
        for declaration in &process.resources {
          let setting = declaration.setting.as_str();
          let selected_value = node["settings"].get(setting);
          let selected_id = match selected_value {
            None => None,
            Some(Value::String(value)) if stable_id(value) => Some(value.clone()),
            Some(_) => {
              return Err(option_error(
                "node_option_invalid_id",
                format!("{pointer}/settings/{}", pointer_segment(setting)),
                node,
              ));
            }
          };
          let unavailable = host_input_unavailable(process, node);
          let rows = resources.iter().filter(|entry| entry["kind"] == declaration.kind).map(|entry| {
                            json!({"value":entry["id"],"label":entry["label"],"description":entry["description"],
                                "available":unavailable.is_none(),"reason_code":unavailable,"reason":unavailable.map(reason)})
                        }).collect();
          groups.insert(
            (short.clone(), setting.to_owned()),
            group(
              &short,
              setting,
              selected_id,
              rows,
              "package_resource_unavailable",
            ),
          );
        }
        continue;
      }
    }
  }
  for role in ["eos", "ei"] {
    let prefix = if role == "eos" { "eos_" } else { "ei_" };
    let legacy_selected = graph["cores"]["isos"]["layers"]
      .as_object()
      .is_some_and(|layers| layers.keys().any(|name| name.starts_with(prefix)));
    let selected = legacy_selected
      || graph["cores"][role]["layers"]
        .as_object()
        .is_some_and(|layers| !layers.is_empty());
    if !selected {
      continue;
    }
    let revision = graph_contract::revision(&serde_json::to_vec(graph)?);
    let reply = crate::runtime::owners::core_setup::preview_options(
      paths, "default", role, graph, &revision,
    )?;
    ensure!(
      reply["ok"] == true,
      "{role} owner options rejected the draft"
    );
    let rows = reply["catalog"]["setting_options"]
      .as_array()
      .with_context(|| format!("{role} owner options lack setting_options"))?;
    ensure!(
      rows.len() <= 512,
      "{role} owner returned too many setting option groups"
    );
    for row in rows {
      let path = row["node_path"]
        .as_str()
        .context("owner option node_path is missing")?;
      let setting = row["setting"]
        .as_str()
        .context("owner option setting is missing")?;
      ensure!(
        path.len() <= 256 && setting.len() <= 128,
        "owner option identity is too long"
      );
      let parts: Vec<_> = path.split('/').collect();
      ensure!(
        parts.len() == 4
          && ((parts[0] == "isos" && parts[1].starts_with(prefix)) || parts[0] == role),
        "owner option path is outside its projection"
      );
      let node = graph
        .pointer(&format!(
          "/cores/{}/layers/{}/groups/{}/nodes/{}",
          parts[0], parts[1], parts[2], parts[3]
        ))
        .context("owner option node is absent from draft")?;
      let selected = if setting == "adapter.type"
        || setting == "adapter.settings.device_id"
        || setting == "adapter.settings.simulator_node"
      {
        ensure!(
          role == "eos"
            && ((parts[0] == "isos" && parts[1] == "eos_hardware")
              || (parts[0] == "eos" && parts[1] == "hardware")),
          "owner adapter selector is outside EOS hardware"
        );
        match setting {
          "adapter.type" => node["adapter"]["type"].as_str(),
          "adapter.settings.device_id" => node["adapter"]["settings"]["device_id"].as_str(),
          _ => node["adapter"]["settings"]["simulator_node"].as_str(),
        }
      } else {
        let definition = catalog["types"]
          .as_array()
          .and_then(|types| types.iter().find(|item| item["id"] == node["node_type"]))
          .context("owner selector type is not registered")?;
        ensure!(
          definition["settings"]
            .as_array()
            .is_some_and(|settings| settings
              .iter()
              .any(|field| field["name"] == setting && field["selector"]["provider"] == "owner")),
          "owner option setting is not declared by the type"
        );
        node["settings"][setting].as_str()
      };
      ensure!(
        row["selected"].as_str() == selected || (row["selected"].is_null() && selected.is_none()),
        "owner option selected value differs from draft"
      );
      let options = row["options"]
        .as_array()
        .context("owner option choices are missing")?;
      ensure!(options.len() <= 512, "owner returned too many choices");
      let mut seen = std::collections::BTreeSet::new();
      for option in options {
        let value = option["value"]
          .as_str()
          .context("owner choice value is missing")?;
        ensure!(
          !value.is_empty() && value.len() <= 128 && seen.insert(value),
          "owner choice value is invalid or duplicated"
        );
        ensure!(
          option["label"]
            .as_str()
            .is_some_and(|text| text.len() <= 256)
            && option["description"]
              .as_str()
              .is_some_and(|text| text.len() <= 1024)
            && option["available"].is_boolean()
            && option.get("reason_code").is_none_or(
              |value| value.is_null() || value.as_str().is_some_and(|text| text.len() <= 128)
            )
            && option.get("reason").is_none_or(
              |value| value.is_null() || value.as_str().is_some_and(|text| text.len() <= 1024)
            ),
          "owner choice metadata is invalid"
        );
      }
      ensure!(
        groups
          .insert((path.to_owned(), setting.to_owned()), row.clone())
          .is_none(),
        "owner option group duplicates a selector"
      );
    }
  }
  data["setting_options"] = json!(groups.into_values().collect::<Vec<_>>());
  Ok(data)
}

pub fn for_graph(paths: &Paths, graph: &Value) -> Result<Value> {
  if let Some(diagnostic) = graph_contract::version_diagnostic(graph) {
    anyhow::bail!(diagnostic);
  }
  let catalog = process::catalog_for_option_graph(paths, graph)?;
  compose_with_paths(paths, graph, &catalog)
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn health_wrapper_has_no_runtime_selector() {
    let descriptor = json!({"id":"ei.cognition.context.builder@v0.2","settings":[]});
    let node = json!({"id":"builder","node_type":"ei.cognition.context.builder@v0.2"});
    check_selectors(
      &descriptor,
      &node,
      "/cores/ei/layers/cognition/groups/context/nodes/builder",
    )
    .unwrap();
  }

  #[test]
  fn selector_uses_short_paths_and_availability() {
    let graph = json!({"version":"0.2.0","cores":{"isos":{"layers":{"eos_hardware":{"groups":{"base":{"nodes":{"drive":{"node_type":"isos.eos_hardware.base@v0.2","adapter":{"type":"vendor-v1"}}}}}}}}}});
    let catalog = json!({"types":[{"id":"isos.eos_dispatcher.primitive_base@v0.2"},{"id":"isos.eos_hardware.base@v0.2","core":"isos","layer":"eos_hardware","kind":"base"}],"adapter_families":[{"id":"vendor-v1","device_kinds":["base"],"transport":"unix_socket","description":"sim","compiled":true}],"primitive_specs":[{"name":"base.velocity","version":"1.0.0","device_class":"base","movement_type":"body_velocity","description":"velocity"}]});
    let options = compose(&graph, &catalog).unwrap();
    assert_eq!(
      options["targets"][0]["path"],
      "isos/eos_hardware/base/drive"
    );
    assert_eq!(options["targets"][0]["available"], true);
    assert_eq!(
      options["primitive_bindings"][0]["primitives"][0]["name"],
      "base.velocity"
    );
  }

  #[test]
  fn selector_declaration_and_ids_fail_at_exact_pointer() {
    let pointer = "/cores/isos/layers/process/groups/demo/nodes/simulator";
    let node = json!({"node_type":"vendor.process.demo@v0.2","package":{"id":"vendor/demo","version":"1.0.0"},"settings":{"model":"small"}});
    let invalid = json!({"settings":[{"name":"model","selector":{"provider":"package","resource":"other","package_from":"node.package"}}]});
    let failure = check_selectors(&invalid, &node, pointer).unwrap_err();
    let failure = failure.downcast_ref::<packages::PackageFailure>().unwrap();
    assert_eq!(failure.code, "node_option_selector_invalid");
    assert_eq!(failure.path, format!("{pointer}/settings/model"));
    let valid = json!({"settings":[{"name":"model","selector":{"provider":"package","resource":"model","package_from":"node.package"}}]});
    let mut invalid_id = node;
    invalid_id["settings"]["model"] = json!("/tmp/secret");
    let failure = check_selectors(&valid, &invalid_id, pointer).unwrap_err();
    assert_eq!(
      failure
        .downcast_ref::<packages::PackageFailure>()
        .unwrap()
        .code,
      "node_option_invalid_id"
    );
  }

  #[test]
  fn ui_key_selector_accepts_bounded_code_without_package_identity() {
    let definition =
      json!({"settings":[{"name":"key","selector":{"provider":"ui","resource":"keys"}}]});
    let mut node = json!({"node_type":"isos.ui.keyboard@v0.2","settings":{"key":"KeyW"}});
    let pointer = "/cores/isos/layers/ui/groups/keyboard/nodes/drive";
    check_selectors(&definition, &node, pointer).unwrap();
    node["settings"]["key"] = json!("/tmp/unsafe");
    assert!(check_selectors(&definition, &node, pointer).is_err());
  }

  #[test]
  fn package_process_selector_is_identity_agnostic() {
    for (package, node_type, selected) in [
      ("field/sensor-rig", "field.process.capture@v0.2", "camera-a"),
      (
        "studio/synth-bank",
        "studio.process.render@v0.2",
        "preset-b",
      ),
    ] {
      let definition = json!({"settings":[{"name":"selection","selector":{
          "provider":"package","resource":"selection","package_from":"node.package"
      }}]});
      let node = json!({"node_type":node_type,"package":{
          "id":package,"version":"1.2.3"
      },"settings":{"selection":selected}});
      check_selectors(
        &definition,
        &node,
        "/cores/isos/layers/process/groups/component/nodes/instance",
      )
      .unwrap();

      let mut invalid = node.clone();
      invalid["settings"]["selection"] = json!("../escape");
      let failure = check_selectors(
        &definition,
        &invalid,
        "/cores/isos/layers/process/groups/component/nodes/instance",
      )
      .unwrap_err();
      assert_eq!(
        failure
          .downcast_ref::<packages::PackageFailure>()
          .unwrap()
          .code,
        "node_option_invalid_id",
        "{package}"
      );
    }
  }

  #[test]
  fn graph_selector_uses_registered_capability_and_port_not_package_identity() {
    let filter = json!({"core":"isos","capability":"primitive_binding",
      "output_data_type":"base_movement","output_transport":"in_process"});
    let graph = json!({"cores":{"isos":{"layers":{"eos_dispatcher":{"groups":{"base":{"nodes":{
      "target":{"node_type":"vendor.binding@v0.2"},
      "wrong":{"node_type":"vendor.other@v0.2"}
    }}}}}}}});
    let catalog = json!({"types":[
      {"id":"vendor.binding@v0.2","description":"Verified primitive binding",
        "capabilities":["primitive_binding"],"outputs":[{"data_type":"base_movement","transport":"in_process"}]},
      {"id":"vendor.other@v0.2","description":"Unrelated output",
        "capabilities":["primitive_binding"],"outputs":[{"data_type":"status","transport":"in_process"}]}
    ]});
    let rows = graph_selector_rows(&graph, &catalog, &filter);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["value"], "isos/eos_dispatcher/base/target");
    let definition = json!({"settings":[{"name":"target","selector":{
      "provider":"graph","resource":"node","filter":filter}}]});
    check_selectors(
      &definition,
      &json!({"settings":{"target":"isos/eos_dispatcher/base/target"}}),
      "/node",
    )
    .unwrap();
    let projected = group(
      "isos/interface/gateway/bridge",
      "target",
      Some("isos/eos_dispatcher/base/missing".into()),
      rows,
      "graph_node_unavailable",
    );
    assert!(
      projected["options"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["value"] == "isos/eos_dispatcher/base/missing" && row["available"] == false)
    );
  }
}
