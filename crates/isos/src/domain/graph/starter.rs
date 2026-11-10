//! Create the editable shared graph from the generic preset.

use crate::app::config::Paths;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::{
  fs,
  path::{Path, PathBuf},
};

fn at<'a>(graph: &'a mut Value, pointer: &str) -> Result<&'a mut Value> {
  graph
    .pointer_mut(pointer)
    .with_context(|| format!("preset lacks {pointer}"))
}

pub fn draft(paths: &Paths, name: &str) -> Result<Value> {
  let preset: toml::Value = toml::from_str(include_str!("../../../templates/generic-graph.toml"))?;
  let mut graph = serde_json::to_value(preset)?;
  at(&mut graph, "/robot/id")?.clone_from(&json!(name));
  let _ = paths;
  Ok(graph)
}

pub fn init(paths: &Paths, name: &str) -> Result<PathBuf> {
  let path = paths.node_graph_path(name);
  if !path.exists() {
    anyhow::ensure!(
      !paths.graph_binding_path(name).exists(),
      "bound graph pool file is missing; choose a new pool file before saving"
    );
    let bytes = serde_json::to_vec_pretty(&draft(paths, name)?)?;
    crate::app::config::atomic_write(&path, &bytes)?;
  }
  Ok(path)
}

pub fn revision(path: &Path) -> Result<String> {
  use sha2::{Digest, Sha256};
  Ok(hex::encode(Sha256::digest(fs::read(path)?)))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn paths(temp: &tempfile::TempDir) -> Paths {
    Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    }
  }

  #[test]
  fn starter_has_three_cores_and_eight_connected_demo_nodes() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(&temp);
    paths.prepare().unwrap();
    let graph = draft(&paths, "robot-a").unwrap();
    assert_eq!(graph["version"], "0.2.0");
    assert_eq!(graph["robot"]["id"], "robot-a");
    assert_eq!(graph["cores"].as_object().unwrap().len(), 3);
    let expected = [
      (
        "/cores/isos/layers/ui/groups/button/nodes/trigger/node_type",
        "isos.ui.button.trigger@v0.2",
      ),
      (
        "/cores/ei/layers/io/groups/input/nodes/trigger/node_type",
        "ei.io.input.trigger@v0.2",
      ),
      (
        "/cores/ei/layers/cognition/groups/intent/nodes/forwarder/node_type",
        "ei.cognition.intent.forwarder@v0.2",
      ),
      (
        "/cores/ei/layers/governance/groups/safety/nodes/gate/node_type",
        "ei.governance.safety.gate@v0.2",
      ),
      (
        "/cores/eos/layers/interface/groups/ingress/nodes/trigger/node_type",
        "eos.interface.ingress.trigger@v0.2",
      ),
      (
        "/cores/eos/layers/skill/groups/demo/nodes/joint_delta/node_type",
        "eos.skill.demo.joint_delta@v0.2",
      ),
      (
        "/cores/eos/layers/dispatcher/groups/arm/nodes/delta/node_type",
        "eos.dispatcher.arm.delta@v0.2",
      ),
      (
        "/cores/eos/layers/hardware/groups/mock/nodes/arm/node_type",
        "eos.hardware.mock.arm@v0.2",
      ),
    ];
    for (path, node_type) in expected {
      assert_eq!(graph.pointer(path).and_then(Value::as_str), Some(node_type));
    }
    assert_eq!(
      graph["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm"]["joint_names"]
        .as_array()
        .unwrap()
        .len(),
      6
    );
    assert_eq!(
      graph["cores"]["ei"]["layers"]["io"]["groups"]["input"]["nodes"]["trigger"]["inputs"]["activation"]
        ["node"],
      "isos/ui/button/trigger"
    );
    let observer =
      &graph["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm_observer"];
    assert_eq!(observer["node_type"], "eos.hardware.mock.arm_observer@v0.2");
    assert!(observer.get("inputs").is_none());
    assert_eq!(
      graph["cores"]["eos"]["layers"]["dispatcher"]["groups"]["arm"]["nodes"]["monitor"]["inputs"]
        ["observation"]["node"],
      "eos/hardware/mock/arm_observer"
    );
  }

  #[test]
  fn pooled_demo_has_two_separate_eight_node_chains() {
    use std::collections::BTreeMap;

    let path =
      Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/graph-pool/demo.nodes.json");
    let graph: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let mut roots = BTreeMap::new();
    let mut edges = Vec::new();
    for (core_id, core) in graph["cores"].as_object().unwrap() {
      for (layer_id, layer) in core["layers"].as_object().unwrap() {
        for (group_id, group) in layer["groups"].as_object().unwrap() {
          for (node_id, node) in group["nodes"].as_object().unwrap() {
            let target = format!("{core_id}/{layer_id}/{group_id}/{node_id}");
            roots.insert(target.clone(), target.clone());
            for input in node["inputs"]
              .as_object()
              .into_iter()
              .flat_map(|v| v.values())
            {
              edges.push((input["node"].as_str().unwrap().to_owned(), target.clone()));
            }
          }
        }
      }
    }
    fn root<'a>(roots: &'a BTreeMap<String, String>, node: &'a str) -> &'a str {
      let parent = roots.get(node).unwrap();
      if parent == node {
        node
      } else {
        root(roots, parent)
      }
    }
    for (source, target) in &edges {
      assert!(roots.contains_key(source), "missing source {source}");
      let source_root = root(&roots, source).to_owned();
      let target_root = root(&roots, target).to_owned();
      roots.insert(source_root, target_root);
    }
    let mut components = BTreeMap::<String, usize>::new();
    for node in roots.keys() {
      *components.entry(root(&roots, node).to_owned()).or_default() += 1;
    }
    let mut sizes: Vec<_> = components.into_values().collect();
    sizes.sort_unstable();
    assert_eq!(edges.len(), 14);
    assert_eq!(sizes, [8, 8]);
  }

  #[test]
  fn init_preserves_existing_authored_graph() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(&temp);
    paths.prepare().unwrap();
    let path = init(&paths, "robot-a").unwrap();
    let graph: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(graph["cores"].as_object().unwrap().len(), 3);
    fs::write(&path, "authored").unwrap();
    assert_eq!(init(&paths, "robot-a").unwrap(), path);
    assert_eq!(fs::read_to_string(path).unwrap(), "authored");
  }

  #[test]
  fn unrelated_json_does_not_block_current_graph_init() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(&temp);
    paths.prepare().unwrap();
    let other = paths.config.join("deployments/robot-a.json");
    fs::create_dir_all(other.parent().unwrap()).unwrap();
    fs::write(&other, b"unrelated data").unwrap();
    assert_eq!(
      init(&paths, "robot-a").unwrap(),
      paths.node_graph_path("robot-a")
    );
    assert_eq!(fs::read(&other).unwrap(), b"unrelated data");
  }
}
