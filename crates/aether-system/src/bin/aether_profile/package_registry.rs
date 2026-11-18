//! Build the graph-scoped EOS registry from IsOS-verified package references.
use std::{
  fs,
  path::{Path, PathBuf},
};
use aether_hal::NodeTypeRegistry;
use serde_json::Value;

pub(super) fn selected(
  request: &Value,
  fallback: &NodeTypeRegistry,
) -> Result<NodeTypeRegistry, &'static str> {
  let Some(packages) = request["packages"].as_array() else {
    return Ok(fallback.clone());
  };
  let mut sidecars = Vec::new();
  for package in packages {
    let Some(store) = package["path"].as_str().map(Path::new) else {
      return Err("invalid_package_ref");
    };
    if !store.is_absolute()
      || package["payload_root"]
        .as_str()
        .is_none_or(|root| !Path::new(root).is_absolute())
    {
      return Err("invalid_package_ref");
    }
    let sidecar = store.join("node-types.json");
    if !sidecar.exists() {
      continue;
    }
    let contribution = fs::read(&sidecar)
      .ok()
      .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    if contribution.as_ref().is_none_or(|value| {
      value["package"]["id"] != package["id"] || value["package"]["version"] != package["version"]
    }) {
      return Err("package_identity_mismatch");
    }
    let actual_root = fs::read(store.join("receipt.json"))
      .ok()
      .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
      .and_then(|receipt| receipt["local_link"].as_str().map(PathBuf::from))
      .unwrap_or_else(|| store.to_path_buf());
    if actual_root.canonicalize().ok()
      != package["payload_root"]
        .as_str()
        .and_then(|root| Path::new(root).canonicalize().ok())
    {
      return Err("package_payload_mismatch");
    }
    sidecars.push(sidecar);
  }
  NodeTypeRegistry::with_contribution_files(&sidecars).map_err(|_| "invalid_package_contribution")
}
