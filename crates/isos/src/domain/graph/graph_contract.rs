//! Exact-version inspection of saved deployment graph bytes.

use anyhow::{Result, bail, ensure};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

pub const MAX_GRAPH_BYTES: usize = 1024 * 1024;
pub const SUPPORTED_VERSIONS: [&str; 1] = ["0.2.0"];

pub fn uses_revisioned_generation(graph: &Value) -> bool {
  graph["version"] == "0.2.0"
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
          let registered = node_type.starts_with(&prefix)
            && node_type.ends_with("@v0.2")
            && catalog.is_some_and(|catalog| {
              catalog["types"].as_array().is_some_and(|types| {
                types.iter().any(|entry| {
                  entry["id"] == node_type
                    && entry["core"] == *core_id
                    && entry["layer"] == *layer_id
                    && entry["group"] == *group_id
                })
              })
            });
          if !registered {
            return Some(diagnostic(
              format!("{path}/node_type"),
              "unsupported_node_type",
              format!(
                "node type {node_type} has no registered 0.2.0 placement; recreate old single-core graphs"
              ),
            ));
          }
          if let Some(definition) = catalog
            .and_then(|catalog| catalog["types"].as_array())
            .and_then(|types| types.iter().find(|entry| entry["id"] == node_type))
            && definition["settings"].is_array()
            && let Err(error) =
              crate::domain::registry::node_schema::validate_instance_settings(node, definition)
          {
            return Some(diagnostic(
              format!("{path}/settings"),
              "invalid_node_settings",
              error.to_string(),
            ));
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
  graph["version"] == "0.2.0"
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
    json!({"version":"0.2.0","robot":{"id":"r","name":"R"},"cores":{
      "isos":{"layers":{"ui":{"groups":{"workbench":{"nodes":{"panel":{
        "id":"panel","node_type":"isos.ui.workbench.panel@v0.2"}}}}}}},
      "ei":{"layers":{}},"eos":{"layers":{}}
    }})
  }
  fn catalog() -> Value {
    json!({"types":[{"id":"isos.ui.workbench.panel@v0.2","core":"isos","layer":"ui","group":"workbench","settings":[]}]})
  }

  #[test]
  fn accepts_only_graph_format_020() {
    assert!(version_diagnostic(&graph()).is_none());
    for version in ["0.1.0", "1.0.0", "2.0.0"] {
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
}
