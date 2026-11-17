//! Exact-version inspection of saved deployment graph bytes.

use anyhow::{Result, bail, ensure};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

pub const MAX_GRAPH_BYTES: usize = 1024 * 1024;
pub const SUPPORTED_VERSIONS: [&str; 1] = ["0.2.1"];

pub fn uses_revisioned_generation(graph: &Value) -> bool {
  graph["version"] == "0.2.1"
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
  pub path: String,
  pub code: &'static str,
  pub message: String,
  #[serde(flatten)]
  pub context: BTreeMap<String, Value>,
}

impl std::fmt::Display for Diagnostic {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.context.is_empty() {
      write!(formatter, "{}: {}: {}", self.path, self.code, self.message)
    } else {
      write!(
        formatter,
        "{}",
        serde_json::to_string(self).map_err(|_| std::fmt::Error)?
      )
    }
  }
}
impl std::error::Error for Diagnostic {}

#[derive(Debug)]
pub struct SavedGraph {
  pub value: Value,
  pub revision: String,
}

fn diagnostic(path: String, code: &'static str, message: String) -> Diagnostic {
  Diagnostic {
    path,
    code,
    message,
    context: BTreeMap::new(),
  }
}

pub(crate) fn pointer_segment(segment: &str) -> String {
  segment.replace('~', "~0").replace('/', "~1")
}

/// Iterate saved instances at their authored placement without assuming a group name.
pub struct GraphInstance<'a> {
  pub path: String,
  pub pointer: String,
  pub node: &'a Value,
}

pub fn instances(graph: &Value) -> Vec<GraphInstance<'_>> {
  let mut result = Vec::new();
  for (core_id, core) in graph["cores"].as_object().into_iter().flatten() {
    for (layer_id, layer) in core["layers"].as_object().into_iter().flatten() {
      for (group_id, group) in layer["groups"].as_object().into_iter().flatten() {
        for (node_id, node) in group["nodes"].as_object().into_iter().flatten() {
          result.push(GraphInstance {
            path: format!("{core_id}/{layer_id}/{group_id}/{node_id}"),
            pointer: format!(
              "/cores/{}/layers/{}/groups/{}/nodes/{}",
              pointer_segment(core_id),
              pointer_segment(layer_id),
              pointer_segment(group_id),
              pointer_segment(node_id)
            ),
            node,
          });
        }
      }
    }
  }
  result
}

pub fn revision(bytes: &[u8]) -> String {
  hex::encode(Sha256::digest(bytes))
}

pub fn read(path: &Path) -> Result<SavedGraph> {
  let bytes = fs::read(path)?;
  parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<SavedGraph> {
  ensure!(bytes.len() <= MAX_GRAPH_BYTES, "node graph exceeds 1 MiB");
  let value: Value = serde_json::from_slice(bytes)?;
  ensure!(value.is_object(), "node graph must be a JSON object");
  Ok(SavedGraph {
    revision: revision(bytes),
    value,
  })
}

/// Convert exact saved 0.2.0 bytes into a 0.2.1 draft. The caller must
/// compare the same revision again when saving the returned graph.
pub fn convert_020(bytes: &[u8], expected_revision: &str) -> Result<Value> {
  let saved = parse(bytes)?;
  ensure!(
    saved.revision == expected_revision,
    "graph revision changed; reload before conversion"
  );
  ensure!(
    saved.value["version"] == "0.2.0",
    "conversion requires graph 0.2.0"
  );
  let mut graph = saved.value;
  graph["version"] = Value::String("0.2.1".into());
  Ok(graph)
}

pub fn version_diagnostic(graph: &Value) -> Option<Diagnostic> {
  let version = graph.get("version").and_then(Value::as_str);
  if version.is_some_and(|version| SUPPORTED_VERSIONS.contains(&version)) {
    return None;
  }
  Some(Diagnostic {
    path: "/version".into(),
    code: "unsupported_graph_version",
    message: format!(
      "graph version {} is unsupported; supported graph versions: {}",
      version.unwrap_or("<missing or invalid>"),
      SUPPORTED_VERSIONS.join(", ")
    ),
    context: BTreeMap::new(),
  })
}

/// A contributed type is identified by both its ID and exact package pin.
/// An unpinned instance may select only a built-in type.
pub fn type_for_node<'a>(catalog: &'a Value, node: &Value) -> Option<&'a Value> {
  let types = catalog["types"].as_array()?;
  let selected = node.get("package");
  let mut matches = types
    .iter()
    .filter(|entry| entry["id"] == node["node_type"] && entry.get("package") == selected);
  let definition = matches.next()?;
  matches.next().is_none().then_some(definition)
}

pub fn admission_diagnostic(graph: &Value, catalog: Option<&Value>) -> Option<Diagnostic> {
  if let Some(problem) = version_diagnostic(graph) {
    return Some(problem);
  }
  let Some(cores) = graph.get("cores").and_then(Value::as_object) else {
    return Some(diagnostic(
      "/cores".into(),
      "invalid_graph",
      "cores must be an object".into(),
    ));
  };
  for core_id in ["isos", "ei", "eos"] {
    if !cores.contains_key(core_id) {
      return Some(diagnostic(
        format!("/cores/{core_id}"),
        "invalid_graph",
        format!("graph requires the {core_id} core"),
      ));
    }
  }
  if let Some(other) = cores
    .keys()
    .find(|id| !matches!(id.as_str(), "isos" | "ei" | "eos"))
  {
    return Some(diagnostic(
      format!("/cores/{}", pointer_segment(other)),
      "unsupported_core",
      "graph permits only isos, ei, and eos cores".into(),
    ));
  }
  for (core_id, core) in cores {
    let Some(layers) = core["layers"].as_object() else {
      return Some(diagnostic(
        format!("/cores/{core_id}/layers"),
        "invalid_graph",
        "layers must be an object".into(),
      ));
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
          let path = format!(
            "/cores/{core_id}/layers/{}/groups/{}/nodes/{}",
            pointer_segment(layer_id),
            pointer_segment(group_id),
            pointer_segment(node_id)
          );
          let node_type = node["node_type"].as_str().unwrap_or("");
          let prefix = format!("{core_id}.{layer_id}.{group_id}.");
          let builtin_process = core_id == "isos"
            && layer_id == "process"
            && group_id == "component"
            && node_type == "isos.process.component@v0.2";
          let definition = catalog.and_then(|catalog| type_for_node(catalog, node));
          let registered = builtin_process
            || (node_type.starts_with(&prefix)
              && node_type.ends_with("@v0.2")
              && definition.is_some_and(|entry| {
                entry["core"] == *core_id
                  && entry["layer"] == *layer_id
                  && entry["group"] == *group_id
              }));
          if !registered {
            return Some(diagnostic(
              format!("{path}/node_type"),
              "unsupported_node_type",
              format!(
                "node type {node_type} has no registered 0.2.1 placement for its exact package pin"
              ),
            ));
          }
          if core_id == "eos"
            && let Some(frequency) = node.pointer("/adapter/settings/frequency_hz")
            && !frequency
              .as_f64()
              .is_some_and(|hz| hz.is_finite() && hz > 0.0 && hz <= 1000.0)
          {
            return Some(diagnostic(
              format!("{path}/adapter/settings/frequency_hz"),
              "invalid_source_frequency",
              "source capture frequency_hz must be finite and within (0, 1000] Hz".into(),
            ));
          }
          if let Some(definition) = definition {
            if core_id == "isos"
              && definition.get("package").is_some()
              && matches!(
                definition["implementation"]["kind"].as_str(),
                Some("external" | "native_plugin")
              )
              && definition["implementation"]
                != serde_json::json!({"kind":"external","interface":"isos.ui.external.v1","version":"1","lifecycle_adapter":"unix-socket-v1"})
            {
              return Some(diagnostic(
                format!("{path}/node_type"),
                "owner_reader_unavailable",
                format!("IsOS has no executable adapter for contributed node type {node_type}"),
              ));
            }
            if definition["settings"].is_array()
              && let Err(error) =
                crate::domain::registry::node_schema::validate_instance_settings(node, definition)
            {
              return Some(diagnostic(
                format!("{path}/settings"),
                "invalid_node_settings",
                error.to_string(),
              ));
            }
          }
          if let Some(inputs) = node["inputs"].as_object() {
            let external_isos = |value: &Value| {
              value.pointer("/package/id").is_some()
                && value["node_type"]
                  .as_str()
                  .is_some_and(|id| id.starts_with("isos."))
                && catalog
                  .and_then(|catalog| type_for_node(catalog, value))
                  .is_some_and(|definition| {
                    definition["implementation"]["interface"] == "isos.ui.external.v1"
                  })
            };
            if external_isos(node) && !inputs.is_empty() {
              return Some(diagnostic(format!("{path}/inputs"), "edge_route_unavailable", "IsOS external UI inputs are delivered through the run-fenced UI interface, not graph edges".into()));
            }
            for (input, binding) in inputs {
              let Some(source) = binding["node"].as_str() else {
                continue;
              };
              let segments: Vec<_> = source.split('/').collect();
              if segments.len() != 4 || segments[0] != "isos" {
                continue;
              }
              let pointer = format!(
                "/cores/{}/layers/{}/groups/{}/nodes/{}",
                segments[0], segments[1], segments[2], segments[3]
              );
              if graph.pointer(&pointer).is_some_and(&external_isos) {
                return Some(diagnostic(
                  format!("{path}/inputs/{}", pointer_segment(input)),
                  "edge_route_unavailable",
                  "IsOS external UI outputs have no owner-supported graph route".into(),
                ));
              }
            }
          }
          if layer_id == "process"
            && group_id == "component"
            && node.pointer("/settings/command").is_some()
          {
            return Some(diagnostic(
              format!("{path}/settings/command"),
              "unsafe_process_node",
              "settings.command cannot be admitted; use role and bounded args".into(),
            ));
          }
        }
      }
    }
  }
  None
}

pub fn admit(graph: &Value, catalog: Option<&Value>) -> Result<()> {
  if let Some(diagnostic) = admission_diagnostic(graph, catalog) {
    bail!(diagnostic);
  }
  Ok(())
}

pub fn has_unified_ei_layers(graph: &Value) -> bool {
  graph["version"] == "0.2.1"
    && graph
      .pointer("/cores/ei/layers")
      .and_then(Value::as_object)
      .is_some_and(|layers| !layers.is_empty())
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn graph() -> Value {
    json!({"version":"0.2.1","robot":{"id":"r","name":"R"},"cores":{
      "isos":{"layers":{"ui":{"groups":{"workbench":{"nodes":{"panel":{
        "id":"panel","node_type":"isos.ui.workbench.panel@v0.2"}}}}}}},
      "ei":{"layers":{}},"eos":{"layers":{}}
    }})
  }
  fn catalog() -> Value {
    json!({"types":[{"id":"isos.ui.workbench.panel@v0.2","core":"isos","layer":"ui","group":"workbench","settings":[]}]})
  }

  #[test]
  fn accepts_only_graph_format_021() {
    assert!(version_diagnostic(&graph()).is_none());
    for version in ["0.1.0", "0.2.0", "1.0.0", "2.0.0"] {
      let diagnostic = version_diagnostic(&json!({"version":version})).unwrap();
      assert_eq!(diagnostic.path, "/version");
      assert_eq!(diagnostic.code, "unsupported_graph_version");
    }
  }

  #[test]
  fn requires_exactly_three_cores() {
    let mut missing = graph();
    missing["cores"].as_object_mut().unwrap().remove("ei");
    assert_eq!(
      admission_diagnostic(&missing, Some(&catalog()))
        .unwrap()
        .path,
      "/cores/ei"
    );
    let mut extra = graph();
    extra["cores"]["other"] = json!({"layers":{}});
    assert_eq!(
      admission_diagnostic(&extra, Some(&catalog())).unwrap().code,
      "unsupported_core"
    );
    assert!(admission_diagnostic(&graph(), Some(&catalog())).is_none());
  }

  #[test]
  fn rejects_out_of_range_instance_capture_frequency() {
    let mut value = graph();
    value["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"]["nodes"]["front"] =
      json!({"id":"front","node_type":"eos.hardware.camera.device@v0.2"});
    let mut types = catalog();
    types["types"]
      .as_array_mut()
      .unwrap()
      .push(json!({"id":"eos.hardware.camera.device@v0.2",
      "core":"eos","layer":"hardware","group":"camera","settings":[]}));
    for invalid in [json!(0), json!(-1), json!(1000.1), json!("30")] {
      value["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"]["nodes"]["front"]["adapter"]
        ["settings"]["frequency_hz"] = invalid;
      let issue = admission_diagnostic(&value, Some(&types)).unwrap();
      assert_eq!(issue.code, "invalid_source_frequency");
      assert!(issue.path.ends_with("/adapter/settings/frequency_hz"));
    }
    value["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"]["nodes"]["front"]["adapter"]
      ["settings"]["frequency_hz"] = json!(1000.0);
    assert!(admission_diagnostic(&value, Some(&types)).is_none());
  }

  #[test]
  fn instance_paths_follow_authored_placement() {
    let value = json!({"cores":{"eos":{"layers":{"sensors":{"groups":{"front~rgb":{"nodes":{"camera_one":{"node_type":"eos.sensors.front~rgb.device@v0.2"}}}}}}}}});
    let entries = instances(&value);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, "eos/sensors/front~rgb/camera_one");
    assert_eq!(
      entries[0].pointer,
      "/cores/eos/layers/sensors/groups/front~0rgb/nodes/camera_one"
    );
  }

  #[test]
  fn admits_builtin_owner_process_without_eos_catalog_entry() {
    let mut value = graph();
    value["cores"]["isos"]["layers"]["process"] = json!({"groups":{"component":{"nodes":{"eos":{
      "id":"eos","node_type":"isos.process.component@v0.2","settings":{"role":"eos"}
    }}}}});
    assert!(admission_diagnostic(&value, Some(&catalog())).is_none());
  }

  #[test]
  fn rejects_legacy_type_and_wrong_core_placement() {
    let mut legacy = graph();
    legacy["cores"]["isos"]["layers"]["ui"]["groups"]["workbench"]["nodes"]["panel"]["node_type"] =
      json!("isos.ui.keyboard@v0.2");
    assert_eq!(
      admission_diagnostic(&legacy, Some(&catalog()))
        .unwrap()
        .code,
      "unsupported_node_type"
    );
    let mut misplaced = graph();
    let node =
      misplaced["cores"]["isos"]["layers"]["ui"]["groups"]["workbench"]["nodes"]["panel"].take();
    misplaced["cores"]["ei"]["layers"]["ui"]["groups"]["workbench"]["nodes"]["panel"] = node;
    misplaced["cores"]["isos"]["layers"]["ui"]["groups"]["workbench"]["nodes"]
      .as_object_mut()
      .unwrap()
      .remove("panel");
    assert_eq!(
      admission_diagnostic(&misplaced, Some(&catalog()))
        .unwrap()
        .code,
      "unsupported_node_type"
    );
  }

  #[test]
  fn detects_ei_layers_in_the_ei_core() {
    assert!(!has_unified_ei_layers(&graph()));
    let mut populated = graph();
    populated["cores"]["ei"]["layers"]["io"] = json!({"groups":{}});
    assert!(has_unified_ei_layers(&populated));
  }

  #[test]
  fn conversion_preserves_instances_edges_settings_and_pins() {
    let source = json!({"version":"0.2.0","robot":{"id":"r","name":"R"},"cores":{"isos":{"layers":{"ui":{"groups":{"x":{"nodes":{"a":{"id":"a","node_type":"isos.ui.x.a@v0.2","settings":{"n":7},"package":{"id":"x/y","version":"1"},"inputs":{"in":{"node":"ei/io/x/b","output":"out"}}}}}}}}},"ei":{"layers":{}},"eos":{"layers":{}}}});
    let bytes = serde_json::to_vec(&source).unwrap();
    assert!(convert_020(&bytes, &"0".repeat(64)).is_err());
    let converted = convert_020(&bytes, &revision(&bytes)).unwrap();
    let mut expected = source;
    expected["version"] = json!("0.2.1");
    assert_eq!(converted, expected);
  }

  #[test]
  fn contributed_isos_external_node_requires_an_executable_owner_adapter() {
    let mut graph = graph();
    graph["cores"]["isos"]["layers"]["ui"]["groups"]["workbench"]["nodes"]["panel"]["package"] =
      json!({"id":"community/panel","version":"1.0.0"});
    let mut catalog = catalog();
    catalog["types"][0]["package"] = json!({"id":"community/panel","version":"1.0.0"});
    catalog["types"][0]["implementation"] = json!({"kind":"external","interface":"community.panel.v1","version":"1","lifecycle_adapter":"process-v1"});
    let problem = admission_diagnostic(&graph, Some(&catalog)).unwrap();
    assert_eq!(problem.code, "owner_reader_unavailable");
    assert!(problem.path.ends_with("/node_type"));
    catalog["types"][0]["implementation"] = json!({"kind":"external","interface":"isos.ui.external.v1","version":"1","lifecycle_adapter":"unix-socket-v1"});
    assert!(admission_diagnostic(&graph, Some(&catalog)).is_none());
  }

  #[test]
  fn same_type_id_from_two_packages_resolves_only_by_exact_pin() {
    let mut graph = graph();
    let node_path = "/cores/isos/layers/ui/groups/workbench/nodes/panel";
    let mut catalog = catalog();
    let mut first = catalog["types"][0].clone();
    first["package"] = json!({"id":"vendor/one","version":"1.0.0"});
    first["settings"] = json!([{"name":"one","data_type":"string","required":true}]);
    first["implementation"] = json!({"kind":"external","interface":"isos.ui.external.v1","version":"1","lifecycle_adapter":"unix-socket-v1"});
    let mut second = first.clone();
    second["package"] = json!({"id":"vendor/two","version":"2.0.0"});
    second["settings"][0]["name"] = json!("two");
    catalog["types"] = json!([first, second]);

    assert_eq!(
      admission_diagnostic(&graph, Some(&catalog)).unwrap().code,
      "unsupported_node_type"
    );
    graph.pointer_mut(node_path).unwrap()["package"] = json!({"id":"vendor/two","version":"2.0.0"});
    graph.pointer_mut(node_path).unwrap()["settings"] = json!({"two":"selected"});
    assert!(admission_diagnostic(&graph, Some(&catalog)).is_none());
    graph.pointer_mut(node_path).unwrap()["package"]["version"] = json!("1.0.0");
    assert_eq!(
      admission_diagnostic(&graph, Some(&catalog)).unwrap().code,
      "unsupported_node_type"
    );
    graph.pointer_mut(node_path).unwrap()["package"] = json!({"id":"vendor/one","version":"1.0.0"});
    assert_eq!(
      admission_diagnostic(&graph, Some(&catalog)).unwrap().code,
      "invalid_node_settings"
    );
  }
}
