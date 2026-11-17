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
  fn starter_has_three_cores_and_nine_connected_demo_nodes() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(&temp);
    paths.prepare().unwrap();
    let graph = draft(&paths, "robot-a").unwrap();
    assert_eq!(graph["version"], "0.2.1");
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
        "/cores/ei/layers/cognition/groups/intent/nodes/context/node_type",
        "ei.cognition.intent.context@v0.2",
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
  fn repository_demo_extends_the_generic_draft_with_package_vision() {
    let temp = tempfile::tempdir().unwrap();
    let expected = draft(&paths(&temp), "default").unwrap();
    let saved: Value = serde_json::from_str(include_str!(concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/../../config/graph-pool/demo.nodes.json"
    )))
    .unwrap();
    assert_eq!(
      saved.pointer(
        "/cores/eos/layers/hardware/groups/camera/nodes/front_camera/settings/camera_width"
      ),
      Some(&json!(1280))
    );
    assert_eq!(
      saved.pointer(
        "/cores/eos/layers/hardware/groups/camera/nodes/front_camera/settings/camera_height"
      ),
      Some(&json!(720))
    );
    let camera =
      &saved["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"]["nodes"]["front_camera"];
    assert_eq!(
      camera["package"],
      json!({"id":"island/mujoco-sim","version":"0.2.6"})
    );
    assert_eq!(camera["adapter"]["package"], camera["package"]);
    assert_eq!(camera["capabilities"], json!(["jpeg"]));
    assert_eq!(camera["adapter"]["settings"]["service_instance"], "native");
    assert_eq!(camera["adapter"]["settings"]["device_id"], "cam-0");
    assert_eq!(camera["adapter"]["settings"]["frequency_hz"], 30);
    assert_eq!(
      saved["cores"]["eos"]["layers"]["interface"]["groups"]["egress"]["nodes"]["front_vision_flow"]
        ["inputs"]["frame"]["sources"]["node"],
      "eos/hardware/camera/front_camera"
    );
    assert_eq!(
      saved["cores"]["isos"]["layers"]["ui"]["groups"]["vision"]["nodes"]["front_monitor"]["inputs"]
        ["frame"]["sources"]["node"],
      "eos/interface/egress/front_vision_flow"
    );
    let mut generic = saved.clone();
    generic
      .pointer_mut("/cores/eos/layers/hardware/groups")
      .unwrap()
      .as_object_mut()
      .unwrap()
      .remove("camera");
    generic
      .pointer_mut("/cores/eos/layers/interface/groups/egress/nodes")
      .unwrap()
      .as_object_mut()
      .unwrap()
      .remove("front_vision_flow");
    generic
      .pointer_mut("/cores/isos/layers/ui/groups")
      .unwrap()
      .as_object_mut()
      .unwrap()
      .remove("vision");
    assert_eq!(generic, expected);
    let count = saved["cores"]
      .as_object()
      .unwrap()
      .values()
      .flat_map(|core| core["layers"].as_object().unwrap().values())
      .flat_map(|layer| layer["groups"].as_object().unwrap().values())
      .map(|group| group["nodes"].as_object().unwrap().len())
      .sum::<usize>();
    assert_eq!(count, 20);
  }

  #[test]
  fn demo_fixture_has_two_separate_eight_node_chains() {
    use std::collections::BTreeMap;

    let path =
      Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/graph/demo-only.nodes.json");
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
