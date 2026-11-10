//! Admission and atomic publication for the current graph.

use crate::{
  app::config::{self, Paths},
  runtime::owners::core_setup,
  domain::graph::edge_admission,
  domain::graph::graph_contract::{self, Diagnostic},
  runtime::lock,
  domain::packages,
  runtime::process,
};
use serde_json::{Value, json};
use std::{
  collections::BTreeMap,
  fs,
  io::Read,
  os::unix::fs::{MetadataExt, OpenOptionsExt},
  path::PathBuf,
};

pub struct Proposal {
  pub graph: Value,
  pub bytes: Vec<u8>,
  pub revision: String,
}

pub struct Prepared {
  pub files: Vec<(PathBuf, Vec<u8>)>,
  pub edges: Vec<edge_admission::Edge>,
}

#[derive(Debug)]
pub enum ApplyError {
  NotFound,
  Conflict(String),
  Diagnostic(Diagnostic),
  Generated(crate::domain::graph::generated_v2::Diagnostic),
  Filesystem(anyhow::Error),
  OutcomeUncertain,
}

impl std::fmt::Display for ApplyError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NotFound => write!(formatter, "graph pool file is missing"),
      Self::Conflict(message) => write!(formatter, "{message}"),
      Self::Diagnostic(diagnostic) => write!(formatter, "{diagnostic}"),
      Self::Generated(diagnostic) => write!(formatter, "{diagnostic}"),
      Self::Filesystem(error) => write!(formatter, "{error:#}"),
      Self::OutcomeUncertain => write!(
        formatter,
        "generated_commit_uncertain: Reread the saved graph and selected generation"
      ),
    }
  }
}

fn generated_error(error: anyhow::Error) -> ApplyError {
  if let Some(diagnostic) = error.downcast_ref::<crate::domain::graph::generated_v2::Diagnostic>() {
    return ApplyError::Generated(diagnostic.clone());
  }
  ApplyError::Filesystem(error)
}

#[cfg(test)]
static FAIL_GRAPH_PARENT_SYNC_FOR: std::sync::OnceLock<
  std::sync::Mutex<Option<std::path::PathBuf>>,
> = std::sync::OnceLock::new();

fn sync_graph_parent(path: &std::path::Path) -> Result<(), ApplyError> {
  #[cfg(test)]
  {
    let mut pending = FAIL_GRAPH_PARENT_SYNC_FOR
      .get_or_init(|| std::sync::Mutex::new(None))
      .lock()
      .unwrap();
    if pending.as_deref() == Some(path) {
      *pending = None;
      return Err(ApplyError::OutcomeUncertain);
    }
  }
  #[cfg(debug_assertions)]
  if crate::domain::graph::generated_v2::test_fail_parent_sync(path, "graph") {
    return Err(ApplyError::OutcomeUncertain);
  }
  fs::File::open(path.parent().unwrap())
    .and_then(|file| file.sync_all())
    .map_err(|_| ApplyError::OutcomeUncertain)
}

fn diagnostic(
  path: impl Into<String>,
  code: &'static str,
  message: impl Into<String>,
) -> Diagnostic {
  Diagnostic {
    path: path.into(),
    code,
    message: message.into(),
    context: BTreeMap::new(),
  }
}

pub(crate) fn package_diagnostic(error: &anyhow::Error) -> Diagnostic {
  if let Some(failure) = error
    .chain()
    .find_map(|cause| cause.downcast_ref::<packages::PackageFailure>())
  {
    let mut context = BTreeMap::new();
    context.insert("id".into(), Value::String(failure.id.clone()));
    context.insert("version".into(), Value::String(failure.version.clone()));
    context.insert("stage".into(), Value::String(failure.stage.into()));
    return Diagnostic {
      path: failure.path.clone(),
      code: failure.code,
      message: failure.to_string(),
      context,
    };
  }
  let message = error.to_string();
  let path = message
    .split(" at ")
    .nth(1)
    .filter(|path| path.starts_with('/'))
    .unwrap_or("/");
  diagnostic(
    path,
    "package_invalid_manifest",
    "Repair the incomplete or invalid package pin",
  )
}

fn package_or_owner_diagnostic(error: &anyhow::Error, owner_path: &str) -> Diagnostic {
  if error
    .chain()
    .any(|cause| cause.downcast_ref::<packages::PackageFailure>().is_some())
  {
    package_diagnostic(error)
  } else {
    diagnostic(owner_path, "owner_reader_unavailable", error.to_string())
  }
}

fn edge_diagnostic(edge: edge_admission::Diagnostic) -> Diagnostic {
  let mut context = BTreeMap::new();
  for (key, value) in [
    ("source", edge.source),
    ("target", edge.target),
    ("input", edge.input),
    ("source_data_type", edge.source_data_type),
    ("target_data_type", edge.target_data_type),
  ] {
    if let Some(value) = value {
      context.insert(key.into(), json!(value));
    }
  }
  Diagnostic {
    path: edge.path,
    code: edge.code,
    message: edge.message,
    context,
  }
}

#[cfg(test)]
fn core_of(path: &str) -> Option<&str> {
  if path.starts_with("cores/") {
    path.split('/').nth(1)
  } else {
    path.split('/').next()
  }
}

#[cfg(test)]
pub fn cross_core_diagnostic(graph: &Value) -> Option<Diagnostic> {
  let cores = graph["cores"].as_object()?;
  for (core_id, core) in cores {
    for (layer_id, layer) in core["layers"].as_object()? {
      for (group_id, group) in layer["groups"].as_object()? {
        for (node_id, node) in group["nodes"].as_object()? {
          let base = format!(
            "/cores/{}/layers/{}/groups/{}/nodes/{}",
            graph_contract::pointer_segment(core_id),
            graph_contract::pointer_segment(layer_id),
            graph_contract::pointer_segment(group_id),
            graph_contract::pointer_segment(node_id)
          );
          if let Some(references) = node["references"].as_array() {
            for (index, value) in references.iter().enumerate() {
              if value
                .as_str()
                .and_then(core_of)
                .is_some_and(|source| source != core_id)
              {
                return Some(diagnostic(
                  format!("{base}/references/{index}"),
                  "cross_core_link_unavailable",
                  "cross-core references require G06 admission",
                ));
              }
            }
          }
          if let Some(dependencies) = node
            .pointer("/settings/depends_on")
            .and_then(Value::as_array)
          {
            for (index, value) in dependencies.iter().enumerate() {
              if value
                .as_str()
                .and_then(core_of)
                .is_some_and(|source| source != core_id)
              {
                return Some(diagnostic(
                  format!("{base}/settings/depends_on/{index}"),
                  "cross_core_link_unavailable",
                  "cross-core process dependencies require G06 admission",
                ));
              }
            }
          }
          // Input edges are admitted by edge_admission against the exact
          // registered port and trusted owner bridge catalog.
        }
      }
    }
  }
  None
}

#[cfg(test)]
fn eos_presence_diagnostic(graph: &Value) -> Option<Diagnostic> {
  let present = graph
    .pointer("/cores/isos/layers")
    .and_then(Value::as_object)
    .is_some_and(|layers| layers.keys().any(|id| id.starts_with("eos_")));
  (!present).then(|| {
    diagnostic(
      "/cores/isos/layers/eos_dispatcher",
      "owner_reader_unavailable",
      "deployment requires EOS-owned layers",
    )
  })
}

pub fn prepare(paths: &Paths, name: &str, proposal: &Proposal) -> Result<Prepared, Diagnostic> {
  if proposal.graph["version"] == "0.2.0"
    && proposal.graph["cores"].get("ei").is_some()
    && proposal.graph["cores"].get("eos").is_some()
  {
    if let Some(other) = proposal.graph["cores"].as_object().and_then(|cores| {
      cores
        .keys()
        .find(|id| !matches!(id.as_str(), "isos" | "ei" | "eos"))
    }) {
      return Err(diagnostic(
        format!("/cores/{other}"),
        "unsupported_core",
        "graph permits only isos, ei, and eos cores",
      ));
    }
    packages::verify_graph_packages(paths, &proposal.graph)
      .map_err(|error| package_diagnostic(&error))?;
    let catalog = process::catalog_for_graph(paths, &proposal.graph)
      .map_err(|error| package_or_owner_diagnostic(&error, "/cores"))?;
    if let Some(problem) = graph_contract::admission_diagnostic(&proposal.graph, Some(&catalog)) {
      return Err(problem);
    }
    let edges =
      edge_admission::admit(&proposal.graph, &catalog).map_err(|error| edge_diagnostic(*error))?;
    for role in ["eos", "ei"] {
      let reply = core_setup::call(
        paths,
        name,
        role,
        "validate",
        &proposal.graph,
        &proposal.revision,
      )
      .map_err(|error| package_or_owner_diagnostic(&error, &format!("/cores/{role}")))?;
      if reply["ok"] != true {
        return Err(diagnostic(
          format!("/cores/{role}"),
          "owner_rejected",
          format!(
            "{role} rejected its graph projection: {}",
            reply["diagnostics"]
          ),
        ));
      }
    }
    let files = core_setup::render_graph(paths, name, &proposal.graph, &proposal.revision)
      .map_err(|error| package_or_owner_diagnostic(&error, "/cores"))?;
    return Ok(Prepared { files, edges });
  }
  Err(
    graph_contract::admission_diagnostic(&proposal.graph, None).unwrap_or_else(|| {
      diagnostic(
        "/cores",
        "unsupported_graph",
        "Apply accepts only the three-core health graph",
      )
    }),
  )
}

pub fn public_diagnostic(diagnostic: &Diagnostic) -> Value {
  if diagnostic.code.starts_with("skill_") {
    return json!({
        "code": diagnostic.code,
        "path": diagnostic.path,
        "message": diagnostic.message,
        "stage": diagnostic.context.get("stage").and_then(Value::as_str).unwrap_or("graph")
    });
  }
  json!(diagnostic)
}

fn check_digest(value: &str) -> Result<(), ApplyError> {
  if value.len() != 64
    || !value
      .bytes()
      .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
  {
    return Err(ApplyError::Diagnostic(diagnostic(
      "/revision",
      "invalid_revision",
      "revision must be a lowercase 64-character SHA-256 hex digest",
    )));
  }
  Ok(())
}

pub fn apply_new(
  paths: &Paths,
  name: &str,
  graph: Value,
  expected_revision: &str,
) -> Result<Proposal, ApplyError> {
  if !expected_revision.is_empty() {
    check_digest(expected_revision)?;
  }
  if graph["version"] != "0.2.0" {
    return Err(ApplyError::Diagnostic(diagnostic(
      "/version",
      "unsupported_graph_version",
      "writer requires graph 0.2.0",
    )));
  }
  let bytes = serde_json::to_vec_pretty(&graph)
    .map_err(|error| ApplyError::Diagnostic(diagnostic("/", "invalid_graph", error.to_string())))?;
  let revision = graph_contract::revision(&bytes);
  let proposal = Proposal {
    graph,
    bytes,
    revision,
  };
  let _lock = lock::Lock::take(paths).map_err(ApplyError::Filesystem)?;
  let path = paths.node_graph_path(name);
  let previous = match fs::read(&path) {
    Ok(bytes) => Some(bytes),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
    Err(error) => return Err(ApplyError::Filesystem(error.into())),
  };
  if previous
    .as_deref()
    .map(graph_contract::revision)
    .as_deref()
    .unwrap_or("")
    != expected_revision
  {
    return Err(ApplyError::Conflict(
      "node graph revision changed; reload before applying".into(),
    ));
  }
  // The expected revision fences replacement of an obsolete saved graph.
  // Only the proposed graph is admitted against the current contract.
  let prepared = prepare(paths, name, &proposal).map_err(ApplyError::Diagnostic)?;
  if let Some(previous) = &previous
    && let Ok(saved) = graph_contract::parse(previous)
    && saved.value == proposal.graph
  {
    core_setup::write_revisioned(paths, name, &saved.value, &saved.revision, prepared.files)
      .map_err(generated_error)?;
    return Ok(Proposal {
      revision: saved.revision,
      bytes: previous.clone(),
      ..proposal
    });
  }
  core_setup::write_revisioned(
    paths,
    name,
    &proposal.graph,
    &proposal.revision,
    prepared.files,
  )
  .map_err(generated_error)?;
  config::atomic_write(&path, &proposal.bytes).map_err(ApplyError::Filesystem)?;
  sync_graph_parent(&path)?;
  Ok(proposal)
}

/// Apply the exact bytes of a named pool file and make it the deployment's binding.
pub fn apply_pool(
  paths: &Paths,
  deployment: &str,
  pool_name: &str,
  expected_revision: &str,
) -> Result<Proposal, ApplyError> {
  check_digest(expected_revision)?;
  crate::runtime::process::validate_deployment_name(pool_name).map_err(ApplyError::Filesystem)?;
  let _lock = lock::Lock::take(paths).map_err(ApplyError::Filesystem)?;
  let path = paths.graph_pool_path(pool_name);
  let directory =
    fs::symlink_metadata(path.parent().expect("pool file has parent")).map_err(|error| {
      if error.kind() == std::io::ErrorKind::NotFound {
        ApplyError::NotFound
      } else {
        ApplyError::Filesystem(error.into())
      }
    })?;
  if !directory.file_type().is_dir() {
    return Err(ApplyError::Filesystem(anyhow::anyhow!(
      "unsafe graph pool directory"
    )));
  }
  let mut file = fs::OpenOptions::new()
    .read(true)
    .custom_flags(libc::O_NOFOLLOW)
    .open(&path)
    .map_err(|error| {
      if error.kind() == std::io::ErrorKind::NotFound {
        ApplyError::NotFound
      } else {
        ApplyError::Filesystem(error.into())
      }
    })?;
  let metadata = file
    .metadata()
    .map_err(|error| ApplyError::Filesystem(error.into()))?;
  if !metadata.is_file() || metadata.nlink() != 1 {
    return Err(ApplyError::Filesystem(anyhow::anyhow!(
      "unsafe graph pool file"
    )));
  }
  let mut bytes = Vec::new();
  file
    .read_to_end(&mut bytes)
    .map_err(|error| ApplyError::Filesystem(error.into()))?;
  let saved = graph_contract::parse(&bytes).map_err(ApplyError::Filesystem)?;
  if saved.revision != expected_revision {
    return Err(ApplyError::Conflict(
      "graph pool revision changed; reload before applying".into(),
    ));
  }
  if saved.value["version"] != "0.2.0" {
    return Err(ApplyError::Diagnostic(diagnostic(
      "/version",
      "unsupported_graph_version",
      "writer requires graph 0.2.0",
    )));
  }
  let proposal = Proposal {
    graph: saved.value,
    bytes,
    revision: saved.revision,
  };
  paths
    .preflight_graph_binding(deployment)
    .map_err(ApplyError::Filesystem)?;
  let prepared = prepare(paths, deployment, &proposal).map_err(ApplyError::Diagnostic)?;
  core_setup::write_revisioned(
    paths,
    deployment,
    &proposal.graph,
    &proposal.revision,
    prepared.files,
  )
  .map_err(generated_error)?;
  paths
    .bind_graph(deployment, pool_name)
    .map_err(|_| ApplyError::OutcomeUncertain)?;
  Ok(proposal)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn apply_pool_rejects_symlinked_directory() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let elsewhere = temp.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    fs::write(elsewhere.join("demo.nodes.json"), b"{}").unwrap();
    std::os::unix::fs::symlink(&elsewhere, paths.config.join("graph-pool")).unwrap();
    let error = apply_pool(&paths, "default", "demo", &"0".repeat(64))
      .err()
      .unwrap();
    assert!(matches!(error, ApplyError::Filesystem(_)));
  }

  #[test]
  fn apply_pool_rejects_hardlinked_file() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let original = temp.path().join("outside.nodes.json");
    fs::write(&original, b"{}").unwrap();
    fs::create_dir(paths.config.join("graph-pool")).unwrap();
    fs::hard_link(&original, paths.graph_pool_path("demo")).unwrap();
    let error = apply_pool(&paths, "default", "demo", &"0".repeat(64))
      .err()
      .unwrap();
    assert!(matches!(error, ApplyError::Filesystem(_)));
  }

  #[test]
  fn three_core_graph_requires_owner_setup_before_publication() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let graph = crate::domain::graph::starter::draft(&paths, "default").unwrap();
    assert_eq!(graph["cores"].as_object().unwrap().len(), 3);
    let error = apply_new(&paths, "default", graph, "").err().unwrap();
    assert!(matches!(
      error,
      ApplyError::Diagnostic(_) | ApplyError::Generated(_)
    ));
    assert!(!paths.node_graph_path("default").exists());
  }

  #[test]
  fn obsolete_saved_graph_needs_revision_and_valid_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let old = br#"{"version":"0.1.0","cores":{"eos":{}}}"#;
    config::atomic_write(&paths.node_graph_path("default"), old).unwrap();
    let graph = json!({"version":"0.2.0","robot":{"id":"default","name":"New deployment"},"cores":{"isos":{"layers":{}}}});
    assert!(matches!(
      apply_new(&paths, "default", graph.clone(), ""),
      Err(ApplyError::Conflict(_))
    ));
    assert_eq!(fs::read(paths.node_graph_path("default")).unwrap(), old);
    let revision = graph_contract::revision(old);
    let error = apply_new(&paths, "default", graph, &revision)
      .err()
      .unwrap();
    assert!(matches!(error, ApplyError::Diagnostic(_)));
    assert_eq!(fs::read(paths.node_graph_path("default")).unwrap(), old);
  }

  #[test]
  fn malformed_saved_graph_is_preserved_when_replacement_is_invalid() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let old = b"{broken graph";
    config::atomic_write(&paths.node_graph_path("default"), old).unwrap();
    let graph = json!({"version":"0.2.0","robot":{"id":"default","name":"New deployment"},"cores":{"isos":{"layers":{}}}});
    let revision = graph_contract::revision(old);
    let error = apply_new(&paths, "default", graph, &revision)
      .err()
      .unwrap();
    assert!(matches!(error, ApplyError::Diagnostic(_)));
    assert_eq!(fs::read(paths.node_graph_path("default")).unwrap(), old);
  }

  /// Test-only staging bridge for the first complete manager replay.
  #[test]
  #[ignore = "requires real EOS and EI owner readers and an explicit isolated home"]
  fn unified_snapshot_fixture_for_supervisor_replay() {
    let source = std::env::var("ISOS_TEST_UNIFIED_GRAPH_PATH").unwrap();
    let home = PathBuf::from(std::env::var("ISOS_TEST_UNIFIED_HOME").unwrap());
    assert!(home.is_absolute());
    let paths = Paths {
      config: home.join("config"),
      data: home.join("data"),
      runtime: home.join("run"),
    };
    paths.prepare().unwrap();
    let path = paths.node_graph_path("default");
    assert!(!path.exists(), "test home already has a graph");
    let bytes = fs::read(source).unwrap();
    let graph: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(graph["version"], "0.2.0");
    let revision = graph_contract::revision(&bytes);
    let proposal = Proposal {
      graph,
      bytes,
      revision,
    };
    let prepared = prepare(&paths, "default", &proposal).unwrap();
    core_setup::write_revisioned(
      &paths,
      "default",
      &proposal.graph,
      &proposal.revision,
      prepared.files,
    )
    .unwrap();
    config::atomic_write(&path, &proposal.bytes).unwrap();
    assert!(sync_graph_parent(&path).is_ok());
    println!("unified test snapshot revision={}", proposal.revision);
  }

  #[test]
  fn unified_prepare_names_missing_eos_owner_layer() {
    let graph = json!({"version":"0.2.0","cores":{"isos":{"layers":{"ei_io":{}}}}});
    let diagnostic = eos_presence_diagnostic(&graph).unwrap();
    assert_eq!(diagnostic.code, "owner_reader_unavailable");
    assert_eq!(diagnostic.path, "/cores/isos/layers/eos_dispatcher");
  }

  #[test]
  fn graph_parent_sync_failure_reports_uncertain_after_visible_rename() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("default.nodes.json");
    let old =
      b"{\"version\":\"0.2.0\",\"robot\":{\"id\":\"old\"},\"cores\":{\"isos\":{\"layers\":{}}}}";
    let new =
      b"{\"version\":\"0.2.0\",\"robot\":{\"id\":\"new\"},\"cores\":{\"isos\":{\"layers\":{}}}}";
    config::atomic_write(&path, old).unwrap();
    config::atomic_write(&path, new).unwrap();
    *FAIL_GRAPH_PARENT_SYNC_FOR
      .get_or_init(|| std::sync::Mutex::new(None))
      .lock()
      .unwrap() = Some(path.clone());
    assert!(matches!(
      sync_graph_parent(&path),
      Err(ApplyError::OutcomeUncertain)
    ));
    let observed = fs::read(&path).unwrap();
    assert!(observed == old || observed == new);
    assert_eq!(observed, new);
    assert_eq!(
      graph_contract::revision(&observed),
      graph_contract::revision(new)
    );
  }

  #[test]
  fn edge_diagnostic_keeps_structured_cli_and_http_context() {
    let graph = json!({"version":"0.2.0","cores":{"isos":{"layers":{"eos_hardware":{"groups":{"camera":{"nodes":{"front":{"id":"front","node_type":"isos.eos_hardware.camera@v0.2"}}}}},"eos_interface":{"groups":{"gate":{"nodes":{"preview":{"id":"preview","node_type":"isos.eos_interface.camera_gate@v0.2","inputs":{"frame":{"node":"isos/eos_hardware/camera/front","output":"frame"}}}}}}}}}}});
    let catalog = json!({"types":[
      {"id":"isos.eos_hardware.camera@v0.2","core":"isos","layer":"eos_hardware","group":"camera","outputs":[{"name":"frame","data_type":"observation_ref","transport":"in_process"}],"inputs":[]},
      {"id":"isos.eos_interface.camera_gate@v0.2","core":"isos","layer":"eos_interface","group":"gate","inputs":[{"name":"frame","data_type":"camera_frame","transport":"in_process"}],"outputs":[]}
    ],"data_types":[{"id":"observation_ref","version":"1.0","purpose":"Reference","media_type":"application/json","codec":"json_v1","schema":{"type":"object"},"max_payload_bytes":1024},{"id":"camera_frame","version":"1.0","purpose":"Frame","media_type":"application/json","codec":"json_v1","schema":{"type":"object"},"max_payload_bytes":1024}]});
    let error = edge_admission::admit(&graph, &catalog).unwrap_err();
    let diagnostic = edge_diagnostic(*error);
    let value = serde_json::to_value(&diagnostic).unwrap();
    assert_eq!(value["code"], "edge_type_mismatch");
    assert_eq!(value["source"], "isos/eos_hardware/camera/front");
    assert_eq!(value["target"], "isos/eos_interface/gate/preview");
    assert_eq!(value["input"], "frame");
    assert_eq!(value["source_data_type"], "observation_ref");
    assert_eq!(value["target_data_type"], "camera_frame");
    assert_eq!(
      serde_json::from_str::<Value>(&diagnostic.to_string()).unwrap(),
      value
    );
  }

  #[test]
  fn cross_core_input_is_left_to_port_admission() {
    let mut graph = serde_json::json!({
        "cores": {
            "eos": { "layers": { "vision": { "groups": { "camera": {
                "nodes": { "cam": { "inputs": {
                    "frame": { "node": "cores/ei/layers/vision/groups/camera/nodes/source" }
                } } }
            } } } } }
        }
    });
    assert!(cross_core_diagnostic(&graph).is_none());
    graph["cores"]["eos"]["layers"]["vision"]["groups"]["camera"]["nodes"]["cam"]["inputs"]["frame"] = serde_json::json!([
        { "node": "cores/eos/layers/vision/groups/camera/nodes/local" },
        { "node": "cores/ei/layers/vision/groups/camera/nodes/source" }
    ]);
    assert!(cross_core_diagnostic(&graph).is_none());
  }

  #[test]
  fn new_apply_rejects_unowned_core_before_any_write() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let mut graph = crate::domain::graph::starter::draft(&paths, "default").unwrap();
    graph["cores"]["evil"] = serde_json::json!({"layers":{}});
    let error = apply_new(&paths, "default", graph, "").err().unwrap();
    let ApplyError::Diagnostic(diagnostic) = error else {
      panic!("unexpected error type");
    };
    assert_eq!(diagnostic.path, "/cores/evil");
    assert_eq!(diagnostic.code, "unsupported_core");
    assert!(!paths.node_graph_path("default").exists());
    assert!(!paths.generated("default").exists());
  }
}
