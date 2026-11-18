//! Resolve package-contributed processes from verified declarations and stable resource IDs.

use crate::{
  app::config::Paths,
  domain::packages,
  domain::registry::release::{ProcessDeclaration, Release},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::PathBuf,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceCatalog {
  contract: String,
  schema_version: String,
  resources: Vec<Resource>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Resource {
  kind: String,
  id: String,
  label: String,
  description: String,
  #[serde(default)]
  path: Option<String>,
  #[serde(default)]
  sha256: Option<String>,
  #[serde(default)]
  metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageIdentity {
  pub id: String,
  pub version: String,
}

pub struct ResolvedProcess {
  pub package: PackageIdentity,
  pub launch: PackageLaunch,
  pub launch_bytes: Vec<u8>,
  pub environment: BTreeMap<String, String>,
  pub fingerprint: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageLaunch {
  pub schema_version: u32,
  pub node_path: String,
  pub node_type: String,
  pub package: PackageIdentity,
  pub entrypoint: PathBuf,
  pub entrypoint_sha256: String,
  #[serde(default)]
  pub args: Vec<String>,
  pub resources: BTreeMap<String, ResourceSelection>,
  pub lifecycle_adapter: String,
  pub release_sha256: Option<String>,
  pub payload_sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceSelection {
  pub kind: String,
  pub id: String,
  pub path: Option<PathBuf>,
  pub sha256: Option<String>,
  pub metadata: Option<Value>,
}

/// Bind a verified process declaration to the exact deployment generation at spawn time.
#[cfg(test)]
pub fn bind_launch(
  launch: &PackageLaunch,
  run_id: &str,
  graph_revision: &str,
  verified_root: &std::path::Path,
) -> Result<Vec<u8>> {
  ensure!(launch.schema_version == 1, "invalid unbound package launch");
  let owner = launch.node_path.split('/').next().unwrap_or_default();
  supported_owner(owner, &launch.node_path, &launch.node_type, &launch.package)?;
  ensure!(
    !run_id.is_empty()
      && run_id.len() <= 64
      && run_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')),
    "invalid package launch run identity"
  );
  ensure!(
    graph_revision.len() == 64
      && graph_revision
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
    "invalid package launch graph revision"
  );
  ensure!(
    launch.entrypoint.starts_with(verified_root)
      && launch.resources.values().all(|resource| resource
        .path
        .as_ref()
        .is_none_or(|path| path.starts_with(verified_root))),
    "package launch path escapes verified store"
  );
  let release_sha256 = launch
    .release_sha256
    .as_deref()
    .context("package launch release digest missing")?;
  let payload_sha256 = launch
    .payload_sha256
    .as_deref()
    .context("package launch payload digest missing")?;
  let bound = crate::interfaces::handoff::ProcessHandoff {
    contract: "isos.package-process@v1",
    schema_version: "0.1.0",
    owner_core: owner,
    run_id,
    graph_revision,
    node_path: &launch.node_path,
    node_type: &launch.node_type,
    package: &launch.package,
    entrypoint: &launch.entrypoint,
    entrypoint_sha256: &launch.entrypoint_sha256,
    args: &launch.args,
    resources: &launch.resources,
    lifecycle_adapter: &launch.lifecycle_adapter,
    release_sha256,
    payload_sha256,
  };
  let bytes = serde_json::to_vec(&bound)?;
  ensure!(bytes.len() <= 65_536, "package launch exceeds 64 KiB");
  Ok(bytes)
}

fn declared_process<'a>(
  release: &'a crate::domain::registry::release::Release,
  node_type: &str,
) -> Option<(&'a ProcessDeclaration, &'a str)> {
  let contributions = release.node_contributions.as_ref()?;
  let process = contributions
    .processes
    .iter()
    .find(|process| process.node_type == node_type)?;
  let owner = contributions
    .node_types
    .iter()
    .find(|registered| registered["id"] == node_type)?
    .get("core")?
    .as_str()?;
  Some((process, owner))
}

fn admission_failure(package: &PackageIdentity, code: &'static str, path: &str) -> anyhow::Error {
  packages::PackageFailure {
    code,
    stage: "admission",
    id: package.id.clone(),
    version: package.version.clone(),
    path: path.to_owned(),
  }
  .into()
}

fn supported_owner(
  owner: &str,
  node_path: &str,
  node_type: &str,
  package: &PackageIdentity,
) -> Result<()> {
  if !matches!(owner, "isos" | "ei" | "eos") {
    return Err(admission_failure(
      package,
      "package_owner_unavailable",
      node_path,
    ));
  }
  if !node_type.starts_with(&format!("{owner}.process."))
    || node_path.split('/').next() != Some(owner)
    || node_path.split('/').nth(1) != Some("process")
  {
    return Err(admission_failure(
      package,
      "package_owner_mismatch",
      node_path,
    ));
  }
  Ok(())
}

fn lifecycle_available(catalog: &Value, family: &str) -> bool {
  catalog["adapter_families"]
    .as_array()
    .is_some_and(|families| {
      families.iter().any(|entry| {
        entry["id"] == family && (entry["compiled"] == true || entry["loadable"] == true)
      })
    })
}

fn catalog(root: &std::path::Path, path: &str) -> Result<ResourceCatalog> {
  let file = packages::checked_file(root, path, false)?;
  let bytes = fs::read(file)?;
  ensure!(
    bytes.len() <= 262_144,
    "package resource catalog exceeds 256 KiB"
  );
  let catalog: ResourceCatalog = serde_json::from_slice(&bytes)?;
  ensure!(
    catalog.contract == "isos.package-resources@v1"
      && catalog.schema_version == "0.1.0"
      && catalog.resources.len() <= 256,
    "unsupported package resource catalog"
  );
  let mut ids = BTreeSet::new();
  let stable = |value: &str| {
    (1..=128).contains(&value.len())
      && value.split('-').all(|part| {
        !part.is_empty()
          && part
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
      })
  };
  for resource in &catalog.resources {
    ensure!(
      stable(&resource.kind)
        && stable(&resource.id)
        && (1..=256).contains(&resource.label.trim().len())
        && (1..=1024).contains(&resource.description.trim().len())
        && resource
          .metadata
          .as_ref()
          .is_none_or(|value| serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() <= 16_384))
        && ids.insert((&resource.kind, &resource.id)),
      "invalid or duplicate package resource"
    );
    if let Some(path) = &resource.path {
      let file = packages::checked_file(root, path, false)?;
      let digest = hex::encode(Sha256::digest(fs::read(file)?));
      ensure!(
        resource.sha256.as_deref() == Some(digest.as_str()),
        "package resource digest mismatch"
      );
    } else {
      ensure!(
        resource.sha256.is_none(),
        "logical resource cannot have file digest"
      );
    }
  }
  Ok(catalog)
}

pub(crate) fn service_resource_path(
  root: &std::path::Path,
  _release: &Release,
  declaration: &crate::domain::registry::release::ProcessResource,
  selected: &str,
) -> Result<std::path::PathBuf> {
  let catalog = catalog(root, &declaration.descriptor)?;
  let resource = catalog
    .resources
    .iter()
    .find(|resource| resource.kind == declaration.kind && resource.id == selected)
    .with_context(|| format!("package resource {selected} is unavailable"))?;
  let relative = resource
    .path
    .as_deref()
    .context("selected package resource has no file")?;
  packages::checked_file(root, relative, false)
}

pub(crate) fn validate_service_payload(
  root: &std::path::Path,
  _release: &Release,
  service: &crate::domain::registry::release::RuntimeService,
) -> Result<()> {
  packages::checked_file(root, &service.entrypoint, true)?;
  for declaration in &service.resources {
    catalog(root, &declaration.descriptor)?;
  }
  Ok(())
}

pub fn validate_payload(
  root: &std::path::Path,
  _release: &Release,
  process: &ProcessDeclaration,
) -> Result<()> {
  packages::checked_file(root, &process.entrypoint, true)?;
  for resource in &process.resources {
    catalog(root, &resource.descriptor)?;
  }
  Ok(())
}

pub fn option_resources(
  root: &std::path::Path,
  _release: &Release,
  process: &ProcessDeclaration,
) -> Result<Vec<Value>> {
  let mut rows = Vec::new();
  for declaration in &process.resources {
    for resource in catalog(root, &declaration.descriptor)?.resources {
      rows.push(json!({
          "kind":resource.kind,
          "id":resource.id,
          "label":resource.label,
          "description":resource.description,
          "metadata":resource.metadata,
      }));
    }
  }
  Ok(rows)
}

pub fn resolve(
  paths: &Paths,
  package: &PackageIdentity,
  node_type: &str,
  node_path: &str,
  settings: &Value,
) -> Result<Option<ResolvedProcess>> {
  let verified = packages::verify(paths, &package.id, &package.version)?;
  let Some((process, owner)) = declared_process(&verified.release, node_type) else {
    return Ok(None);
  };
  supported_owner(owner, node_path, node_type, package)?;
  validate_payload(&verified.root, &verified.release, process)?;
  let entrypoint = packages::checked_file(&verified.root, &process.entrypoint, true)?;
  let mut resources = BTreeMap::new();
  for declaration in &process.resources {
    let selected = settings
      .get(&declaration.setting)
      .and_then(Value::as_str)
      .with_context(|| format!("missing package resource setting {}", declaration.setting))?;
    let index = catalog(&verified.root, &declaration.descriptor)?;
    let item = index
      .resources
      .iter()
      .find(|r| r.kind == declaration.kind && r.id == selected)
      .with_context(|| format!("package resource {selected} is unavailable"))?;
    let path = item
      .path
      .as_deref()
      .map(|path| packages::checked_file(&verified.root, path, false))
      .transpose()?;
    resources.insert(
      declaration.setting.clone(),
      ResourceSelection {
        kind: item.kind.clone(),
        id: item.id.clone(),
        path,
        sha256: item.sha256.clone(),
        metadata: item.metadata.clone(),
      },
    );
  }
  let mut environment = BTreeMap::new();
  for input in &process.host_inputs {
    let value = settings
      .get("env")
      .and_then(|env| env.get(&input.name))
      .and_then(Value::as_str)
      .map(str::to_owned)
      .or_else(|| std::env::var(&input.name).ok());
    if input.required {
      ensure!(
        value.is_some(),
        "required package host input {} is missing",
        input.name
      );
    }
    if let Some(value) = value {
      let directory = PathBuf::from(&value);
      ensure!(
        directory.is_absolute() && directory.is_dir(),
        "package host input must be an absolute directory"
      );
      environment.insert(input.name.clone(), value);
    }
  }
  let launch = PackageLaunch {
    schema_version: 1,
    node_path: node_path.to_owned(),
    node_type: node_type.to_owned(),
    package: package.clone(),
    entrypoint_sha256: hex::encode(Sha256::digest(std::fs::read(&entrypoint)?)),
    entrypoint,
    args: process.args.clone(),
    resources,
    lifecycle_adapter: process.lifecycle_adapter.clone(),
    release_sha256: verified.receipt.release_sha256.clone(),
    payload_sha256: verified.receipt.payload_sha256.clone(),
  };
  let launch_bytes = serde_json::to_vec(&launch)?;
  let fingerprint = hex::encode(Sha256::digest(&launch_bytes));
  Ok(Some(ResolvedProcess {
    package: package.clone(),
    launch,
    launch_bytes,
    environment,
    fingerprint,
  }))
}

fn graph_nodes(graph: &Value) -> Vec<(String, &Value)> {
  let mut result = Vec::new();
  if let Some(cores) = graph.get("cores").and_then(Value::as_object) {
    for (core_id, core) in cores {
      if let Some(layers) = core.get("layers").and_then(Value::as_object) {
        for (layer_id, layer) in layers {
          if let Some(groups) = layer.get("groups").and_then(Value::as_object) {
            for (group_id, group) in groups {
              if let Some(nodes) = group.get("nodes").and_then(Value::as_object) {
                for (node_id, node) in nodes {
                  result.push((format!("{core_id}/{layer_id}/{group_id}/{node_id}"), node));
                }
              }
            }
          }
        }
      }
    }
  }
  result
}

pub(crate) fn ensure_single_eos_process(graph: &Value) -> Result<()> {
  let mut selected = None;
  for (path, node) in graph_nodes(graph) {
    let mut segments = path.split('/');
    let placement = matches!(
      (segments.next(), segments.next()),
      (Some("eos"), Some("process")) | (Some("isos"), Some("eos_process"))
    );
    if placement && node.get("package").is_some() {
      ensure!(
        selected.is_none(),
        "one EOS package process is supported per graph"
      );
      selected = Some(path);
    }
  }
  Ok(())
}

pub fn validate_graph(paths: &Paths, graph: &Value) -> Result<()> {
  ensure_single_eos_process(graph)?;
  let nodes = graph_nodes(graph);
  let mut processes = BTreeMap::new();
  for (path, node) in &nodes {
    let Some(node_type) = node.get("node_type").and_then(Value::as_str) else {
      continue;
    };
    let Some(pin) = node.get("package") else {
      continue;
    };
    let package: PackageIdentity = serde_json::from_value(pin.clone())?;
    let verified = packages::verify(paths, &package.id, &package.version)?;
    // EI and EOS validate and supervise their own contributed processes.
    if declared_process(&verified.release, node_type).is_none() {
      if matches!(path.split('/').nth(1), Some("process" | "eos_process")) {
        return Err(admission_failure(
          &package,
          "package_process_unsupported_contract",
          path,
        ));
      }
      continue;
    }
    if let Some(resolved) = resolve(paths, &package, node_type, path, &node["settings"])? {
      processes.insert(path.clone(), (resolved, &node["settings"]));
    }
  }
  if !processes.is_empty() {
    let catalog = crate::runtime::process::catalog_for_option_graph(paths, graph)?;
    for (process, _) in processes.values() {
      let family = &process.launch.lifecycle_adapter;
      if !lifecycle_available(&catalog, family) {
        return Err(admission_failure(
          &process.package,
          "package_lifecycle_adapter_unavailable",
          &process.launch.node_path,
        ));
      }
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn two_eos_package_processes_reject_before_owner_selection() {
    let process = json!({"node_type":"eos.process.driver.service@v0.2",
      "package":{"id":"vendor/driver","version":"0.1.2"}});
    let graph = json!({"version":"0.2.1","robot":{},"cores":{"eos":{"layers":{"process":{"groups":{"driver":{"nodes":{
      "first":process,"second":process
    }}}}}}}});
    assert!(ensure_single_eos_process(&graph).is_err());
    assert!(
      crate::runtime::process::project(&graph.to_string())
        .unwrap_err()
        .to_string()
        .contains("one EOS package process")
    );
    let one = json!({"cores":{"eos":{"layers":{"process":{"groups":{"driver":{"nodes":{
      "first":process
    }}}}}}}});
    ensure_single_eos_process(&one).unwrap();
  }

  #[test]
  fn uncompiled_driver_lifecycle_rejects_admission() {
    let catalog = json!({"adapter_families":[{"id":"vendor-socket","compiled":false}]});
    assert!(!lifecycle_available(&catalog, "vendor-socket"));
    assert!(!lifecycle_available(&catalog, "unknown"));
    assert!(lifecycle_available(
      &json!({"adapter_families":[{"id":"vendor-socket","compiled":true}]}),
      "vendor-socket"
    ));
    assert!(lifecycle_available(
      &json!({"adapter_families":[{"id":"mock-v1","compiled":false,"loadable":true}]}),
      "mock-v1"
    ));
  }

  #[test]
  fn unrelated_resource_families_share_digest_and_containment_checks() {
    for (kind, id, filename) in [
      ("calibration", "camera-a", "calibration.dat"),
      ("preset", "voice-b", "preset.dat"),
    ] {
      let root = tempfile::tempdir().unwrap();
      fs::write(root.path().join(filename), b"verified").unwrap();
      let digest = hex::encode(Sha256::digest(b"verified"));
      let mut declaration = json!({"contract":"isos.package-resources@v1",
        "schema_version":"0.1.0","resources":[{"kind":kind,"id":id,
        "label":"Selected resource","description":"Owner validated resource",
        "path":filename,"sha256":digest}]});
      let descriptor = root.path().join("resources.json");
      fs::write(&descriptor, serde_json::to_vec(&declaration).unwrap()).unwrap();
      assert_eq!(
        catalog(root.path(), "resources.json").unwrap().resources[0].id,
        id
      );

      fs::write(root.path().join(filename), b"changed").unwrap();
      assert!(
        catalog(root.path(), "resources.json").is_err(),
        "{kind} changed file"
      );
      declaration["resources"][0]["path"] = json!("../escape.dat");
      fs::write(&descriptor, serde_json::to_vec(&declaration).unwrap()).unwrap();
      assert!(
        catalog(root.path(), "resources.json").is_err(),
        "{kind} unsafe path"
      );
      declaration["resources"][0]["path"] = json!(filename);
      declaration["resources"][0]["metadata"] = json!({"opaque":"x".repeat(20_000)});
      fs::write(&descriptor, serde_json::to_vec(&declaration).unwrap()).unwrap();
      assert!(
        catalog(root.path(), "resources.json").is_err(),
        "{kind} oversized metadata"
      );
    }
  }
  #[test]
  fn unknown_or_mismatched_process_owner_is_typed() {
    let package = PackageIdentity {
      id: "vendor/demo".into(),
      version: "1.0.0".into(),
    };
    for (owner, path, code) in [
      (
        "vendor",
        "vendor/process/demo/one",
        "package_owner_unavailable",
      ),
      ("eos", "vendor/process/demo/one", "package_owner_mismatch"),
      ("eos", "eos/hardware/demo/one", "package_owner_mismatch"),
      ("eos", "isos/process/demo/one", "package_owner_mismatch"),
    ] {
      let error =
        supported_owner(owner, path, "isos.eos_process.driver@v0.2", &package).unwrap_err();
      let failure = error.downcast_ref::<packages::PackageFailure>().unwrap();
      assert_eq!(failure.code, code);
      assert_eq!(failure.path, path);
    }
  }
  #[test]
  fn generic_catalog_binds_file_and_logical_resources() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("model.bin"), b"model").unwrap();
    let digest = hex::encode(Sha256::digest(b"model"));
    let catalog_data = json!({"contract":"isos.package-resources@v1","schema_version":"0.1.0","resources":[
      {"kind":"model","id":"small","label":"Small","description":"small model","path":"model.bin","sha256":digest},
      {"kind":"mode","id":"fast","label":"Fast","description":"fast mode","metadata":{"hint":"owner validated"}}
    ]});
    fs::write(
      root.path().join("resources.json"),
      serde_json::to_vec(&catalog_data).unwrap(),
    )
    .unwrap();
    assert_eq!(
      catalog(root.path(), "resources.json")
        .unwrap()
        .resources
        .len(),
      2
    );
    fs::write(root.path().join("model.bin"), b"changed").unwrap();
    assert!(catalog(root.path(), "resources.json").is_err());
    let mut old = catalog_data;
    old["schema_version"] = json!(1);
    fs::write(
      root.path().join("resources.json"),
      serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    assert!(catalog(root.path(), "resources.json").is_err());
  }

  #[test]
  fn bind_launch_requires_current_fence_and_contained_paths() {
    let root = std::path::Path::new("/bundle");
    let launch = PackageLaunch {
      schema_version: 1,
      node_path: "eos/process/driver/service".into(),
      node_type: "eos.process.driver.service@v0.2".into(),
      package: PackageIdentity {
        id: "vendor/sim".into(),
        version: "1.0.0".into(),
      },
      entrypoint: root.join("bin/start"),
      entrypoint_sha256: "a".repeat(64),
      args: Vec::new(),
      resources: BTreeMap::from([(
        "model".into(),
        ResourceSelection {
          kind: "model".into(),
          id: "small".into(),
          path: Some(root.join("model.bin")),
          sha256: Some("a".repeat(64)),
          metadata: None,
        },
      )]),
      lifecycle_adapter: "process".into(),
      release_sha256: Some("a".repeat(64)),
      payload_sha256: Some("b".repeat(64)),
    };
    let fence = "c".repeat(64);
    let bytes = bind_launch(&launch, "run_1", &fence, root).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["contract"], "isos.package-process@v1");
    assert_eq!(value["schema_version"], "0.1.0");
    assert_eq!(value["resources"]["model"]["id"], "small");
    assert_eq!(value["owner_core"], "eos");
    assert_eq!(value["node_type"], "eos.process.driver.service@v0.2");
    assert!(bind_launch(&launch, "run_1", "bad", root).is_err());
    assert!(bind_launch(&launch, "run_1", &fence, std::path::Path::new("/other")).is_err());
  }
}
