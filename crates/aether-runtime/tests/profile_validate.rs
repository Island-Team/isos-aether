use std::{fs, process::Command};
use serde_json::{json, Value};

fn demo_projection() -> Value {
  let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../tests/fixtures/graph/demo-only.nodes.json");
  let mut graph: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
  let eos = graph["cores"]["eos"].clone();
  graph["cores"] = json!({"eos":eos});
  graph
}

#[test]
fn validate_cli_accepts_executable_projection_and_rejects_unsupported_graph() {
  let dir = tempfile::tempdir().unwrap();
  let file = dir.path().join("graph.json");
  let mut graph = demo_projection();
  fs::write(&file, serde_json::to_vec(&graph).unwrap()).unwrap();
  let run = || {
    Command::new(env!("CARGO_BIN_EXE_aether-profile"))
      .arg("validate")
      .arg("--file")
      .arg(&file)
      .output()
      .unwrap()
  };
  let output = run();
  assert!(
    output.status.success(),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
  assert!(output.stdout.is_empty());

  graph["cores"]["eos"]["layers"]["skill"]["groups"]["demo"]["nodes"]
    .as_object_mut()
    .unwrap()
    .remove("arm_status");
  fs::write(&file, serde_json::to_vec(&graph).unwrap()).unwrap();
  let output = run();
  assert!(!output.status.success());
  assert!(!output.stderr.is_empty());
}

#[test]
fn validate_cli_rejects_wrong_core_and_unreadable_sidecar() {
  let dir = tempfile::tempdir().unwrap();
  let file = dir.path().join("graph.json");
  let graph: Value = serde_json::from_slice(
    &fs::read(
      std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/graph/demo-only.nodes.json"),
    )
    .unwrap(),
  )
  .unwrap();
  fs::write(&file, serde_json::to_vec(&graph).unwrap()).unwrap();
  let output = Command::new(env!("CARGO_BIN_EXE_aether-profile"))
    .arg("validate")
    .arg("--file")
    .arg(&file)
    .output()
    .unwrap();
  assert!(!output.status.success());
  assert!(String::from_utf8_lossy(&output.stderr).contains("EOS-only"));

  let output = Command::new(env!("CARGO_BIN_EXE_aether-profile"))
    .arg("validate")
    .arg("--file")
    .arg(&file)
    .arg("--node-types")
    .arg(dir.path().join("missing.json"))
    .output()
    .unwrap();
  assert!(!output.status.success());
}

#[test]
fn validate_cli_rejects_non_executable_eos_projection() {
  let dir = tempfile::tempdir().unwrap();
  let file = dir.path().join("graph.json");
  let mut graph = demo_projection();
  let ingress = graph["cores"]["eos"]["layers"]["interface"]["groups"]["ingress"].clone();
  graph["cores"]["eos"]["layers"] = json!({"interface":{"groups":{"ingress":ingress}}});
  fs::write(&file, serde_json::to_vec(&graph).unwrap()).unwrap();
  let output = Command::new(env!("CARGO_BIN_EXE_aether-profile"))
    .arg("validate")
    .arg("--file")
    .arg(&file)
    .output()
    .unwrap();
  assert!(!output.status.success());
  assert!(
    String::from_utf8_lossy(&output.stderr).contains("unsupported_execution"),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
}
