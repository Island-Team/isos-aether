use aether_node_profile::{NodeTypeRegistry, NodeTypeContribution};
use serde_json::json;
use sha2::{Digest, Sha256};

#[test]
fn sidecar_verifies_linux_library_bytes_and_rejects_change() {
  let directory = tempfile::tempdir().unwrap();
  std::fs::write(directory.path().join("node.so"), b"sample-native-bytes").unwrap();
  let digest = format!("{:x}", Sha256::digest(b"sample-native-bytes"));
  let mut node = serde_json::to_value(
    NodeTypeRegistry::builtin()
      .get("eos.skill.demo.joint_delta@v0.2")
      .unwrap(),
  )
  .unwrap();
  node["id"] = json!("eos.skill.demo.custom_delta@v0.2");
  node["implementation"] = json!({"kind":"native_plugin","abi":"isos_node_v1","library":"node.so"});
  let sidecar = json!({"package":{"id":"vendor/example","version":"1.0.0"},
    "libraries":[{"path":"node.so","sha256":digest}],"node_types":[node]});
  let path = directory.path().join("node-types.json");
  std::fs::write(&path, sidecar.to_string()).unwrap();
  let registry = NodeTypeRegistry::with_contribution_files(std::slice::from_ref(&path)).unwrap();
  assert!(registry.get("eos.skill.demo.custom_delta@v0.2").is_some());
  assert!(registry
    .verified_native_library_for_type("eos.skill.demo.custom_delta@v0.2")
    .is_ok());
  std::fs::write(directory.path().join("node.so"), b"changed").unwrap();
  assert!(registry
    .verified_native_library_for_type("eos.skill.demo.custom_delta@v0.2")
    .is_err());
  assert!(NodeTypeRegistry::with_contribution_files(&[path]).is_err());
}

#[test]
fn contribution_rejects_other_library_suffix_and_missing_abi() {
  let mut node = serde_json::to_value(
    NodeTypeRegistry::builtin()
      .get("eos.skill.demo.joint_delta@v0.2")
      .unwrap(),
  )
  .unwrap();
  node["id"] = json!("eos.skill.demo.custom_delta@v0.2");
  node["implementation"] = json!({"kind":"native_plugin","abi":"wrong","library":"node.dll"});
  let contribution: NodeTypeContribution = serde_json::from_value(json!({
    "package":{"id":"vendor/example","version":"1.0.0"},
    "libraries":[{"path":"node.dll","sha256":"a".repeat(64)}],"node_types":[node]
  }))
  .unwrap();
  assert!(NodeTypeRegistry::builtin()
    .register_contribution(contribution)
    .is_err());
}

#[test]
fn adapter_family_uses_only_its_verified_external_library() {
  let directory = tempfile::tempdir().unwrap();
  std::fs::write(directory.path().join("adapter.so"), b"adapter-bytes").unwrap();
  let digest = format!("{:x}", Sha256::digest(b"adapter-bytes"));
  let sidecar = json!({
    "package":{"id":"vendor/example","version":"1.0.0"},
    "libraries":[{"path":"adapter.so","sha256":digest}],
    "adapters":[{"family":"vendor-sim","library":"adapter.so","sha256":digest}],
    "adapter_families":[{"id":"vendor-sim","device_kinds":["camera"],
      "transport":"unix_socket","description":"Example camera adapter"}]
  });
  let path = directory.path().join("node-types.json");
  std::fs::write(&path, sidecar.to_string()).unwrap();
  let registry = NodeTypeRegistry::with_contribution_files(std::slice::from_ref(&path)).unwrap();
  assert!(registry
    .verified_adapter_library_for_family("vendor-sim")
    .is_ok());
  assert!(registry
    .catalog_adapter_families()
    .iter()
    .any(|family| family.descriptor.id == "vendor-sim" && family.loadable));
  std::fs::write(directory.path().join("adapter.so"), b"changed").unwrap();
  assert!(registry
    .verified_adapter_library_for_family("vendor-sim")
    .is_err());
  assert!(NodeTypeRegistry::with_contribution_files(&[path]).is_err());
}
