//! Admission of IsOS package handoffs and bounded lifecycle for an owner child.
//! A child may be reported stopped only after its adapter supplies Stop proof.

use std::{
  collections::BTreeMap,
  fs,
  os::unix::{fs::PermissionsExt, process::CommandExt},
  path::{Path, PathBuf},
  process::{Child, Command, Stdio},
  thread,
  time::{Duration, Instant},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Default)]
pub(super) struct PackageInputs {
  pub(super) adapters: BTreeMap<String, AdapterLibrary>,
  pub(super) process: Option<ProcessLaunch>,
  pub(super) sidecars: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub(super) struct AdapterLibrary {
  pub(super) path: PathBuf,
  pub(super) sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProcessLaunch {
  schema_version: u32,
  node_path: String,
  node_type: String,
  package: PackageIdentity,
  pub(super) entrypoint: PathBuf,
  entrypoint_sha256: String,
  #[serde(default)]
  args: Vec<String>,
  resources: BTreeMap<String, ResourceSelection>,
  lifecycle_adapter: String,
  release_sha256: Option<String>,
  payload_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageIdentity {
  id: String,
  version: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceSelection {
  kind: String,
  id: String,
  path: Option<PathBuf>,
  sha256: Option<String>,
  metadata: Option<serde_json::Value>,
}

fn valid_digest(value: &str) -> bool {
  value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn file_digest(path: &Path) -> Result<String, String> {
  let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
  if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 {
    return Err("package file missing or too large".into());
  }
  let bytes = fs::read(path).map_err(|error| error.to_string())?;
  Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(super) fn parse_adapter(value: &str) -> Result<(String, AdapterLibrary), String> {
  let (family, source) = value
    .split_once('=')
    .ok_or("adapter library requires family=path@sha256")?;
  let (path, digest) = source
    .rsplit_once('@')
    .ok_or("adapter library requires path@sha256")?;
  if family.is_empty()
    || !family
      .bytes()
      .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    || !valid_digest(digest)
  {
    return Err("invalid adapter family or SHA-256".into());
  }
  let path = PathBuf::from(path);
  if !path.is_absolute() || path.extension().is_none_or(|extension| extension != "so") {
    return Err("adapter library must be an absolute .so".into());
  }
  let actual = file_digest(&path)?;
  if actual != digest.to_ascii_lowercase() {
    return Err("adapter library changed after IsOS verification".into());
  }
  Ok((
    family.to_owned(),
    AdapterLibrary {
      path,
      sha256: actual,
    },
  ))
}

pub(super) fn parse_process(path: &Path, sidecars: &[PathBuf]) -> Result<ProcessLaunch, String> {
  if !path.is_absolute() {
    return Err("package process handoff path must be absolute".into());
  }
  let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
  if !metadata.is_file() || metadata.len() > 65_536 {
    return Err("package process handoff missing or too large".into());
  }
  let bytes = fs::read(path).map_err(|error| error.to_string())?;
  let mut launch: ProcessLaunch =
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid package handoff: {error}"))?;
  if launch.schema_version != 1
    || !launch.node_path.starts_with("eos/process/")
    || !launch.node_type.starts_with("eos.process.")
    || launch.lifecycle_adapter.is_empty()
    || !launch.entrypoint.is_absolute()
    || !valid_digest(&launch.entrypoint_sha256)
    || launch.resources.is_empty()
    || launch.args.len() > 16
    || launch
      .release_sha256
      .as_deref()
      .is_none_or(|digest| !valid_digest(digest))
    || launch
      .payload_sha256
      .as_deref()
      .is_none_or(|digest| !valid_digest(digest))
  {
    return Err("invalid EOS package process handoff".into());
  }
  let selected = sidecars.iter().find_map(|sidecar| {
    let contents: serde_json::Value = serde_json::from_slice(&fs::read(sidecar).ok()?).ok()?;
    (contents["package"]["id"] == launch.package.id
      && contents["package"]["version"] == launch.package.version)
      .then_some(sidecar)
  });
  let sidecar = selected.ok_or("package process has no selected sidecar")?;
  let store = sidecar.parent().ok_or("invalid sidecar location")?;
  let release = store.join("release.json");
  if file_digest(&release)?
    != launch
      .release_sha256
      .as_deref()
      .unwrap()
      .to_ascii_lowercase()
  {
    return Err("package release changed after IsOS verification".into());
  }
  let receipt: serde_json::Value =
    serde_json::from_slice(&fs::read(store.join("receipt.json")).map_err(|e| e.to_string())?)
      .map_err(|e| format!("invalid package receipt: {e}"))?;
  if receipt["payload_sha256"] != launch.payload_sha256.as_deref().unwrap() {
    return Err("package payload digest differs from receipt".into());
  }
  let root = receipt["local_link"]
    .as_str()
    .map(PathBuf::from)
    .unwrap_or_else(|| store.to_path_buf())
    .canonicalize()
    .map_err(|e| e.to_string())?;
  let entrypoint = launch
    .entrypoint
    .canonicalize()
    .map_err(|e| e.to_string())?;
  if !entrypoint.starts_with(&root) || entrypoint == root {
    return Err("package entrypoint escapes verified root".into());
  }
  launch.entrypoint = entrypoint.clone();
  let executable = fs::metadata(&entrypoint).map_err(|e| e.to_string())?;
  if !executable.is_file() || executable.permissions().mode() & 0o111 == 0 {
    return Err("package entrypoint is not executable".into());
  }
  if file_digest(&entrypoint)? != launch.entrypoint_sha256.to_ascii_lowercase() {
    return Err("package entrypoint differs from IsOS handoff".into());
  }
  for resource in launch.resources.values_mut() {
    if resource.kind.is_empty() || resource.id.is_empty() {
      return Err("invalid package resource selection".into());
    }
    if let Some(path) = &resource.path {
      let canonical = path.canonicalize().map_err(|e| e.to_string())?;
      if !canonical.starts_with(&root) || canonical == root {
        return Err("package resource escapes verified root".into());
      }
      if let Some(expected) = &resource.sha256 {
        if !valid_digest(expected) || file_digest(&canonical)? != expected.to_ascii_lowercase() {
          return Err("package resource changed after IsOS verification".into());
        }
      }
      resource.path = Some(canonical);
    }
    if resource
      .metadata
      .as_ref()
      .is_some_and(|value| !value.is_object())
    {
      return Err("invalid package resource metadata".into());
    }
  }
  Ok(launch)
}

impl ProcessLaunch {
  pub(super) fn node_path(&self) -> &str {
    &self.node_path
  }
  pub(super) fn node_type(&self) -> &str {
    &self.node_type
  }
  pub(super) fn package_id(&self) -> &str {
    &self.package.id
  }
  pub(super) fn package_version(&self) -> &str {
    &self.package.version
  }
  pub(super) fn lifecycle_adapter(&self) -> &str {
    &self.lifecycle_adapter
  }
  pub(super) fn arguments(&self, socket: &Path) -> Result<Vec<String>, String> {
    if !self.entrypoint_sha256.is_empty()
      && file_digest(&self.entrypoint)? != self.entrypoint_sha256
    {
      return Err("package entrypoint changed before launch".into());
    }
    for resource in self.resources.values() {
      if let (Some(path), Some(expected)) = (&resource.path, &resource.sha256) {
        if file_digest(path)? != expected.to_ascii_lowercase() {
          return Err("package resource changed before launch".into());
        }
      }
    }
    let mut result = Vec::new();
    for argument in &self.args {
      if argument.is_empty() || argument.len() > 256 || argument.contains('\0') {
        return Err("invalid process argument".into());
      }
      let resolved = if argument == "${socket}" {
        socket.to_string_lossy().into_owned()
      } else if let Some(name) = argument
        .strip_prefix("${resource.")
        .and_then(|v| v.strip_suffix(".path}"))
      {
        self
          .resources
          .get(name)
          .and_then(|resource| resource.path.as_ref())
          .ok_or("process resource argument missing")?
          .to_string_lossy()
          .into_owned()
      } else if argument.contains("${") {
        return Err("unsupported process argument template".into());
      } else {
        argument.clone()
      };
      result.push(resolved);
    }
    Ok(result)
  }
}

pub(super) fn validate_inputs(
  inputs: &PackageInputs,
  registry: &aether_hal::NodeTypeRegistry,
) -> Result<(), String> {
  for sidecar in &inputs.sidecars {
    if !sidecar.is_absolute() {
      return Err("package sidecar path must be absolute".into());
    }
    let metadata = fs::symlink_metadata(sidecar).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 4 * 1024 * 1024 {
      return Err("package sidecar missing or too large".into());
    }
    let receipt: serde_json::Value = serde_json::from_slice(
      &fs::read(
        sidecar
          .parent()
          .ok_or("invalid sidecar location")?
          .join("receipt.json"),
      )
      .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let expected = receipt["sidecar_sha256"]
      .as_str()
      .ok_or("package receipt lacks sidecar digest")?;
    if !valid_digest(expected) || file_digest(sidecar)? != expected.to_ascii_lowercase() {
      return Err("package sidecar changed after IsOS verification".into());
    }
  }
  for (family, supplied) in &inputs.adapters {
    let registered = registry
      .verified_adapter_library_for_family(family)
      .map_err(|error| error.to_string())?;
    if registered
      .canonicalize()
      .map_err(|error| error.to_string())?
      != supplied
        .path
        .canonicalize()
        .map_err(|error| error.to_string())?
      || file_digest(registered)? != supplied.sha256
    {
      return Err(format!(
        "adapter {family} differs from selected package sidecar"
      ));
    }
  }
  if let Some(process) = &inputs.process {
    if !inputs.adapters.contains_key(&process.lifecycle_adapter) {
      return Err("package process lacks selected owner adapter library".into());
    }
  }
  Ok(())
}

/// A direct owner child. The caller supplies a bounded, owner-specific Stop proof.
#[allow(dead_code)] // Run remains fail-closed until owner sampling and Stop proof are qualified.
pub(super) struct ProcessChild {
  child: Child,
}
#[allow(dead_code)]
impl ProcessChild {
  pub(super) fn spawn(launch: &ProcessLaunch, args: &[String]) -> Result<Self, String> {
    let mut command = Command::new(&launch.entrypoint);
    command
      .args(args)
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .stderr(Stdio::null());
    let parent = unsafe { libc::getpid() };
    unsafe {
      command.pre_exec(move || {
        if libc::setpgid(0, 0) != 0
          || libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0
          || libc::getppid() != parent
        {
          return Err(std::io::Error::other("owner child setup failed"));
        }
        Ok(())
      });
    }
    let child = command.spawn().map_err(|error| error.to_string())?;
    Ok(Self { child })
  }

  pub(super) fn pid(&self) -> u32 {
    self.child.id()
  }

  /// Even with failed proof, clean up the process group and report Stop as unknown.
  pub(super) fn stop_with_proof(
    &mut self,
    proof: impl FnOnce() -> Result<(), String>,
  ) -> Result<(), String> {
    let proved = proof();
    let pid = self.child.id() as libc::pid_t;
    unsafe { libc::kill(-pid, libc::SIGTERM) };
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut child_exited = false;
    while Instant::now() < deadline {
      match self.child.try_wait() {
        Ok(Some(_)) => child_exited = true,
        Ok(None) => {}
        Err(error) => return Err(format!("child reap failed: {error}")),
      }
      if child_exited && unsafe { libc::kill(-pid, 0) } != 0 {
        return proved;
      }
      thread::sleep(Duration::from_millis(10));
    }
    unsafe { libc::kill(-pid, libc::SIGKILL) };
    if !child_exited {
      self
        .child
        .wait()
        .map_err(|error| format!("child reap failed: {error}"))?;
    }
    for _ in 0..20 {
      if unsafe { libc::kill(-pid, 0) } != 0 {
        return proved;
      }
      thread::sleep(Duration::from_millis(10));
    }
    Err("package process group exit unproven".into())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_tampered_adapter_bytes() {
    let root = tempfile::tempdir().unwrap();
    let library = root.path().join("node.so");
    fs::write(&library, b"before").unwrap();
    let digest = file_digest(&library).unwrap();
    let (family, admitted) =
      parse_adapter(&format!("camera={}@{digest}", library.display())).unwrap();
    assert_eq!(family, "camera");
    assert_eq!(admitted.sha256, digest);
    assert_eq!(admitted.path, library);
    fs::write(&library, b"after").unwrap();
    assert!(parse_adapter(&format!("camera={}@{digest}", library.display())).is_err());
  }

  #[test]
  fn admits_exact_package_handoff_and_rejects_changed_resource() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let store = root.join("store");
    fs::create_dir(&store).unwrap();
    let executable = root.join("backend");
    fs::write(&executable, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let scene = root.join("scene.xml");
    fs::write(&scene, b"<driver/>").unwrap();
    let release = store.join("release.json");
    fs::write(
      &release,
      serde_json::to_vec(&serde_json::json!({
        "contract":"isos.package@v1","schema_version":"0.2.0","id":"vendor/test","kind":"backend",
        "version":"0.2.0","source_commit":"a".repeat(40),
        "assets":[{"name":"vendor-test.tar.gz","os":"linux","arch":"x86_64","url":"https://github.com/vendor/test/releases/download/v0.2.0/vendor-test.tar.gz","size_bytes":1,"sha256":"b".repeat(64)}]
      }))
      .unwrap(),
    )
    .unwrap();
    let sidecar = store.join("node-types.json");
    fs::write(
      &sidecar,
      b"{\"package\":{\"id\":\"vendor/test\",\"version\":\"0.2.0\"}}",
    )
    .unwrap();
    fs::write(
      store.join("receipt.json"),
      serde_json::to_vec(&serde_json::json!({
        "local_link":root,"payload_sha256":"a".repeat(64)
      }))
      .unwrap(),
    )
    .unwrap();
    let handoff = root.join("handoff.json");
    fs::write(
      &handoff,
      serde_json::to_vec(&serde_json::json!({
        "schema_version":1,"node_path":"eos/process/test/sim",
        "node_type":"eos.process.test@v0.2",
        "package":{"id":"vendor/test","version":"0.2.0"},
      "entrypoint":executable,"entrypoint_sha256":file_digest(&executable).unwrap(),
        "resources":{"scene":{"kind":"scene","id":"first","path":scene,
          "sha256":file_digest(&scene).unwrap(),"metadata":null}},
        "lifecycle_adapter":"test-v1",
        "release_sha256":file_digest(&release).unwrap(),
        "payload_sha256":"a".repeat(64)
      }))
      .unwrap(),
    )
    .unwrap();
    let admitted = parse_process(&handoff, std::slice::from_ref(&sidecar)).unwrap();
    assert!(admitted.arguments(&root.join("run.sock")).is_ok());
    fs::write(&executable, b"#!/bin/sh\nexit 1\n").unwrap();
    assert!(admitted.arguments(&root.join("run.sock")).is_err());
    assert!(parse_process(&handoff, std::slice::from_ref(&sidecar)).is_err());
    fs::write(&executable, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::write(scene, b"changed").unwrap();
    assert!(parse_process(&handoff, &[sidecar]).is_err());
  }

  #[test]
  fn failed_stop_proof_still_reaps_group() {
    let executable = PathBuf::from("/bin/sleep");
    let launch = ProcessLaunch {
      schema_version: 1,
      node_path: "eos/process/test/node".into(),
      node_type: "eos.process.test@v0.2".into(),
      package: PackageIdentity {
        id: "test/pkg".into(),
        version: "0.2.0".into(),
      },
      entrypoint_sha256: file_digest(&executable).unwrap(),
      entrypoint: executable,
      args: Vec::new(),
      resources: BTreeMap::new(),
      lifecycle_adapter: "test".into(),
      release_sha256: None,
      payload_sha256: None,
    };
    let mut child = ProcessChild::spawn(&launch, &["30".into()]).unwrap();
    let pid = child.pid();
    assert!(child
      .stop_with_proof(|| Err("Stop unknown".into()))
      .is_err());
    assert_eq!(unsafe { libc::kill(pid as libc::pid_t, 0) }, -1);
  }

  #[test]
  fn process_arguments_bind_only_selected_resources_and_owner_socket() {
    let launch = ProcessLaunch {
      schema_version: 1,
      node_path: "eos/process/test/node".into(),
      node_type: "eos.process.test@v0.2".into(),
      package: PackageIdentity {
        id: "test/pkg".into(),
        version: "0.2.0".into(),
      },
      entrypoint: "/bin/true".into(),
      entrypoint_sha256: file_digest(Path::new("/bin/true")).unwrap(),
      args: vec![
        "--socket".into(),
        "${socket}".into(),
        "--scene".into(),
        "${resource.scene.path}".into(),
      ],
      resources: BTreeMap::from([(
        "scene".into(),
        ResourceSelection {
          kind: "scene".into(),
          id: "one".into(),
          path: Some("/verified/scene.xml".into()),
          sha256: None,
          metadata: None,
        },
      )]),
      lifecycle_adapter: "test".into(),
      release_sha256: None,
      payload_sha256: None,
    };
    assert_eq!(
      launch.arguments(Path::new("/private/run.sock")).unwrap(),
      vec![
        "--socket",
        "/private/run.sock",
        "--scene",
        "/verified/scene.xml"
      ]
    );
    let mut invalid = launch;
    invalid.args = vec!["${resource.other.path}".into()];
    assert!(invalid.arguments(Path::new("/private/run.sock")).is_err());
  }
}
