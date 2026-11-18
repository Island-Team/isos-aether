//! Direct, run-fenced EOS library handlers. These nodes do not create graph routes.
use std::{
  collections::{BTreeMap, BTreeSet},
  fs::{self, File, OpenOptions},
  io::Write,
  path::{Path, PathBuf},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{NodeTypeRegistry, ProfileError};

const MAX_AGE_NS: u64 = 1_000_000_000;

#[derive(Clone, Debug, PartialEq)]
pub struct TypedOutput {
  pub port: &'static str,
  pub data_type: &'static str,
  pub payload: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IsolatedHealth {
  pub state: &'static str,
  pub reason: Option<&'static str>,
  pub generation: u64,
  pub source_time_ns: Option<u64>,
}

/// The caller supplies a trusted run identity and current time. No graph setting can set either.
pub struct IsolatedNode {
  kind: Kind,
  run_id: String,
  frame_id: String,
  target: Option<String>,
  generation: u64,
  stopped: bool,
  latest: Option<TypedOutput>,
  source_time_ns: Option<u64>,
}

#[derive(Clone, Copy)]
enum Kind {
  Camera,
  Features,
  Binding,
  Guard,
}

impl IsolatedNode {
  pub fn start(
    type_id: &str,
    run_id: &str,
    settings: &BTreeMap<String, toml::Value>,
  ) -> Result<Self, ProfileError> {
    if run_id.is_empty() || run_id.len() > 128 {
      return Err(ProfileError("invalid run identity".into()));
    }
    let kind = match type_id {
      "eos.hardware.mock.camera@v0.2" => Kind::Camera,
      "eos.perception.physical.features@v0.2" => Kind::Features,
      "eos.dispatcher.command.binding@v0.2" => Kind::Binding,
      "eos.safety.device.guard@v0.2" => Kind::Guard,
      _ => return Err(ProfileError("isolated implementation unavailable".into())),
    };
    let allowed = match kind {
      Kind::Camera | Kind::Features | Kind::Guard => &["frame_id"][..],
      Kind::Binding => &["frame_id", "target_node"][..],
    };
    if settings.keys().any(|key| !allowed.contains(&key.as_str())) {
      return Err(ProfileError("unknown isolated node setting".into()));
    }
    let frame_id = settings
      .get("frame_id")
      .and_then(toml::Value::as_str)
      .ok_or_else(|| ProfileError("frame_id setting required".into()))?;
    if !valid_identity(frame_id, 128) {
      return Err(ProfileError("invalid frame_id setting".into()));
    }
    let target = if matches!(kind, Kind::Binding) {
      let target = settings
        .get("target_node")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| ProfileError("target_node setting required".into()))?;
      if target != "eos/hardware/mock/arm" || !valid_path(target) {
        return Err(ProfileError("invalid target_node setting".into()));
      }
      Some(target.to_owned())
    } else {
      None
    };
    Ok(Self {
      kind,
      run_id: run_id.to_owned(),
      frame_id: frame_id.to_owned(),
      target,
      generation: 1,
      stopped: false,
      latest: None,
      source_time_ns: None,
    })
  }

  pub fn input(
    &mut self,
    port: &str,
    payload: &Value,
    now_ns: u64,
  ) -> Result<Vec<TypedOutput>, ProfileError> {
    if self.stopped {
      return Err(ProfileError("node stopped".into()));
    }
    let registry = NodeTypeRegistry::builtin();
    let (expected_port, data_type) = match self.kind {
      Kind::Camera => ("sample", "sample_request@v0.2"),
      Kind::Features | Kind::Guard => ("state", "joint_state@v0.2"),
      Kind::Binding => ("request", "movement_binding_request@v0.2"),
    };
    if port != expected_port {
      return Err(ProfileError("undeclared input port".into()));
    }
    registry.validate_data_payload(data_type, payload)?;
    let sample = if matches!(self.kind, Kind::Binding | Kind::Camera) {
      payload
    } else {
      &payload["sample"]
    };
    let source_time_ns = sample["source_time_ns"]
      .as_u64()
      .ok_or_else(|| ProfileError("source time missing".into()))?;
    if source_time_ns > now_ns || now_ns - source_time_ns > MAX_AGE_NS {
      return Err(ProfileError("stale or future input".into()));
    }
    let frame_id = sample["frame_id"]
      .as_str()
      .ok_or_else(|| ProfileError("frame missing".into()))?;
    if frame_id != self.frame_id {
      return Err(ProfileError("frame mismatch".into()));
    }
    if matches!(self.kind, Kind::Features | Kind::Guard) && sample["clock"] != "monotonic" {
      return Err(ProfileError("source clock mismatch".into()));
    }
    if !matches!(self.kind, Kind::Binding)
      && self
        .source_time_ns
        .is_some_and(|last| source_time_ns <= last)
    {
      return Err(ProfileError("non-increasing source time".into()));
    }
    let output = match self.kind {
      Kind::Camera => {
        // Four deterministic RGB pixels. The caller's sample metadata is retained and marked mock.
        let image = json!({"sample":{"source_time_ns":source_time_ns,"clock":"monotonic",
          "sequence":payload["sequence"],"frame_id":frame_id},"width_px":2,"height_px":2,
          "encoding":"rgb8","data":"AAECAwQFBgcICQoL"});
        TypedOutput {
          port: "frame",
          data_type: "image_frame@v0.2",
          payload: image,
        }
      }
      Kind::Features => {
        let positions = payload["position_rad"]
          .as_array()
          .ok_or_else(|| ProfileError("joint positions missing".into()))?;
        if positions.len() != payload["joint_names"].as_array().map_or(0, Vec::len)
          || positions.len() != payload["velocity_rad_s"].as_array().map_or(0, Vec::len)
        {
          return Err(ProfileError("joint vector lengths mismatch".into()));
        }
        let near_limit = positions
          .iter()
          .any(|value| value.as_f64().is_some_and(|rad| rad.abs() >= 3.0));
        TypedOutput {
          port: "event",
          data_type: "physical_event@v0.2",
          payload: json!({"run_id":self.run_id,"source_time_ns":source_time_ns,"frame_id":frame_id,
            "source_sequence":payload["sample"]["sequence"],"event":"joint_limit_proximity",
            "state":if near_limit {"detected"} else {"clear"},"provenance":"input_joint_state",
            "rule_version":"1"}),
        }
      }
      Kind::Guard => {
        let positions = payload["position_rad"]
          .as_array()
          .ok_or_else(|| ProfileError("joint positions missing".into()))?;
        if positions.len() != payload["joint_names"].as_array().map_or(0, Vec::len)
          || positions.len() != payload["velocity_rad_s"].as_array().map_or(0, Vec::len)
        {
          return Err(ProfileError("joint vector lengths mismatch".into()));
        }
        let inhibited = positions
          .iter()
          .any(|value| value.as_f64().is_some_and(|rad| rad.abs() > 314.0 / 100.0));
        TypedOutput {
          port: "status",
          data_type: "device_status@v0.2",
          payload: json!({"run_id":self.run_id,"source_time_ns":source_time_ns,"frame_id":frame_id,
            "source_sequence":payload["sample"]["sequence"],"state":if inhibited {"inhibited"} else {"ready"},
            "reason":if inhibited {Value::String("joint_limit".into())} else {Value::Null}}),
        }
      }
      Kind::Binding => {
        if payload["target_node"].as_str() != self.target.as_deref()
          || payload["primitive"] != "arm.delta@1.0.0"
          || payload["mode"] != "joint_positions"
          || payload["unit"] != "rad"
        {
          return Err(ProfileError(
            "primitive, target, mode or unit mismatch".into(),
          ));
        }
        let deltas = payload["joint_deltas_rad"]
          .as_array()
          .ok_or_else(|| ProfileError("delta missing".into()))?;
        if deltas.len() != 6
          || deltas
            .iter()
            .any(|v| v.as_f64().is_none_or(|n| n.abs() > 0.25))
        {
          return Err(ProfileError("arm delta exceeds bound".into()));
        }
        TypedOutput {
          port: "binding",
          data_type: "binding_result@v0.2",
          payload: json!({"run_id":self.run_id,"target_node":self.target,"primitive":"arm.delta@1.0.0",
            "mode":"joint_positions","unit":"rad","valid":true}),
        }
      }
    };
    registry.validate_data_payload(output.data_type, &output.payload)?;
    if matches!(self.kind, Kind::Camera) {
      crate::vision_flow::validate_frame(&output.payload, &registry)
        .map_err(|error| ProfileError(format!("mock frame invalid: {error}")))?;
    }
    self.source_time_ns = Some(source_time_ns);
    self.latest = Some(output.clone());
    Ok(vec![output])
  }

  pub fn read(&self) -> Option<&TypedOutput> {
    self.latest.as_ref()
  }

  pub fn health(&self) -> IsolatedHealth {
    IsolatedHealth {
      state: if self.stopped { "unavailable" } else { "ready" },
      reason: self.stopped.then_some("stopped"),
      generation: self.generation,
      source_time_ns: self.source_time_ns,
    }
  }

  pub fn health_at(&self, now_ns: u64) -> IsolatedHealth {
    let mut health = self.health();
    if !self.stopped
      && self
        .source_time_ns
        .is_some_and(|time| time > now_ns || now_ns - time > MAX_AGE_NS)
    {
      health.state = "degraded";
      health.reason = Some("sample_stale");
    }
    health
  }

  pub fn stop(&mut self) -> bool {
    self.stopped = true;
    true
  }
}

fn valid_identity(value: &str, max: usize) -> bool {
  !value.is_empty()
    && value.len() <= max
    && value
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn valid_path(value: &str) -> bool {
  value.len() <= 128 && value.split('/').all(|part| valid_identity(part, 64))
}

/// EOS owner service adapter; a proof reference must come from the real Stop boundary.
pub trait EmergencyStopService {
  fn request_stop(&mut self, run_id: &str, reason: &str) -> Result<String, ProfileError>;
}

pub struct IsolatedEstop<S: EmergencyStopService> {
  run_id: String,
  service: S,
  latest: Option<TypedOutput>,
  stopped: bool,
}

impl<S: EmergencyStopService> IsolatedEstop<S> {
  pub fn start(run_id: &str, service: S) -> Result<Self, ProfileError> {
    if !valid_identity(run_id, 128) {
      return Err(ProfileError("invalid run identity".into()));
    }
    Ok(Self {
      run_id: run_id.into(),
      service,
      latest: None,
      stopped: false,
    })
  }

  pub fn input(
    &mut self,
    port: &str,
    payload: &Value,
    now_ns: u64,
  ) -> Result<TypedOutput, ProfileError> {
    if self.stopped || self.latest.is_some() || port != "stop" {
      return Err(ProfileError("Stop input closed or undeclared".into()));
    }
    NodeTypeRegistry::builtin().validate_data_payload("stop_request@v0.2", payload)?;
    if payload["run_id"] != self.run_id
      || payload["source_time_ns"]
        .as_u64()
        .is_none_or(|time| time > now_ns || now_ns - time > MAX_AGE_NS)
    {
      return Err(ProfileError("Stop run or source time stale".into()));
    }
    let reason = payload["reason"]
      .as_str()
      .ok_or_else(|| ProfileError("Stop reason missing".into()))?;
    let proof = self.service.request_stop(&self.run_id, reason)?;
    if !valid_identity(&proof, 128) {
      return Err(ProfileError(
        "Stop service returned invalid proof reference".into(),
      ));
    }
    let output = TypedOutput {
      port: "latch",
      data_type: "stop_latch@v0.2",
      payload: json!({"run_id":self.run_id,"latched":true,"proof_ref":proof}),
    };
    NodeTypeRegistry::builtin().validate_data_payload(output.data_type, &output.payload)?;
    self.latest = Some(output.clone());
    Ok(output)
  }

  pub fn read(&self) -> Option<&TypedOutput> {
    self.latest.as_ref()
  }
  pub fn health(&self) -> IsolatedHealth {
    IsolatedHealth {
      state: if self.stopped { "unavailable" } else { "ready" },
      reason: self.stopped.then_some("stopped"),
      generation: 1,
      source_time_ns: None,
    }
  }
  pub fn stop(&mut self) -> bool {
    self.stopped = true;
    true
  }
}

/// Owner supplies the storage root; graph settings cannot choose filesystem paths.
pub struct EpisodeRecorder {
  run_id: String,
  episode_id: String,
  path: PathBuf,
  file: File,
  max_bytes: u64,
  bytes: u64,
  record_count: u64,
  lost_count: u64,
  seen: BTreeSet<String>,
  stopped: bool,
}

impl EpisodeRecorder {
  pub fn start(
    root: &Path,
    run_id: &str,
    episode_id: &str,
    max_bytes: u64,
  ) -> Result<Self, ProfileError> {
    if !valid_identity(run_id, 128)
      || !valid_identity(episode_id, 128)
      || !(128..=16_777_216).contains(&max_bytes)
    {
      return Err(ProfileError(
        "invalid episode identity or byte budget".into(),
      ));
    }
    fs::create_dir_all(root)
      .map_err(|error| ProfileError(format!("evidence store unavailable: {error}")))?;
    let path = root.join(format!("{episode_id}.jsonl"));
    let file = OpenOptions::new()
      .write(true)
      .create_new(true)
      .open(&path)
      .map_err(|error| ProfileError(format!("episode unavailable: {error}")))?;
    Ok(Self {
      run_id: run_id.into(),
      episode_id: episode_id.into(),
      path,
      file,
      max_bytes,
      bytes: 0,
      record_count: 0,
      lost_count: 0,
      seen: BTreeSet::new(),
      stopped: false,
    })
  }

  pub fn input(&mut self, port: &str, payload: &Value) -> Result<Vec<TypedOutput>, ProfileError> {
    if self.stopped || port != "record" {
      return Err(ProfileError("record input closed or undeclared".into()));
    }
    NodeTypeRegistry::builtin().validate_data_payload("evidence_record@v0.2", payload)?;
    if payload["run_id"] != self.run_id {
      return Err(ProfileError("record run fence mismatch".into()));
    }
    let id = payload["record_id"]
      .as_str()
      .ok_or_else(|| ProfileError("record ID missing".into()))?;
    if !valid_identity(id, 128) || self.seen.contains(id) {
      return Err(ProfileError("invalid or duplicate record ID".into()));
    }
    if payload["source_ref"]
      .as_str()
      .is_none_or(|value| !valid_identity(value, 128))
    {
      return Err(ProfileError("invalid evidence source reference".into()));
    }
    let mut line = serde_json::to_vec(payload)
      .map_err(|error| ProfileError(format!("record encoding failed: {error}")))?;
    line.push(b'\n');
    if self.bytes.saturating_add(line.len() as u64) > self.max_bytes {
      self.lost_count += 1;
      return Ok(vec![self.gap("byte_budget")]);
    }
    if let Err(error) = self
      .file
      .write_all(&line)
      .and_then(|_| self.file.sync_data())
    {
      self.lost_count += 1;
      return Err(ProfileError(format!(
        "evidence write failed; gap count {}: {error}",
        self.lost_count
      )));
    }
    self.bytes += line.len() as u64;
    self.record_count += 1;
    self.seen.insert(id.into());
    Ok(Vec::new())
  }

  fn gap(&self, reason: &'static str) -> TypedOutput {
    TypedOutput {
      port: "gap",
      data_type: "gap_record@v0.2",
      payload: json!({
      "run_id":self.run_id,"episode_id":self.episode_id,"lost_count":self.lost_count,"reason":reason}),
    }
  }

  pub fn health(&self) -> IsolatedHealth {
    IsolatedHealth {
      state: if self.stopped {
        "unavailable"
      } else if self.lost_count > 0 {
        "degraded"
      } else {
        "ready"
      },
      reason: if self.stopped {
        Some("stopped")
      } else if self.lost_count > 0 {
        Some("evidence_gap")
      } else {
        None
      },
      generation: 1,
      source_time_ns: None,
    }
  }

  pub fn stop(&mut self) -> Result<TypedOutput, ProfileError> {
    if self.stopped {
      return Err(ProfileError("episode already stopped".into()));
    }
    self
      .file
      .sync_all()
      .map_err(|error| ProfileError(format!("episode flush uncertain: {error}")))?;
    self.stopped = true;
    let bytes = fs::read(&self.path)
      .map_err(|error| ProfileError(format!("episode digest unavailable: {error}")))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let output = TypedOutput {
      port: "episode",
      data_type: "episode_reference@v0.2",
      payload: json!({
      "run_id":self.run_id,"episode_id":self.episode_id,"record_count":self.record_count,"sha256":digest}),
    };
    NodeTypeRegistry::builtin().validate_data_payload(output.data_type, &output.payload)?;
    Ok(output)
  }
}

pub struct DatasetManifest {
  root: PathBuf,
  run_id: String,
  authorized: BTreeSet<String>,
  latest: Option<TypedOutput>,
  stopped: bool,
}

impl DatasetManifest {
  pub fn start(
    root: &Path,
    run_id: &str,
    authorized: BTreeSet<String>,
  ) -> Result<Self, ProfileError> {
    if !valid_identity(run_id, 128)
      || authorized.is_empty()
      || !authorized.iter().all(|id| valid_identity(id, 128))
    {
      return Err(ProfileError("invalid manifest authorization".into()));
    }
    Ok(Self {
      root: root.into(),
      run_id: run_id.into(),
      authorized,
      latest: None,
      stopped: false,
    })
  }

  pub fn input(&mut self, port: &str, payload: &Value) -> Result<TypedOutput, ProfileError> {
    if self.stopped || port != "request" {
      return Err(ProfileError("manifest input closed or undeclared".into()));
    }
    NodeTypeRegistry::builtin().validate_data_payload("manifest_request@v0.2", payload)?;
    if payload["run_id"] != self.run_id {
      return Err(ProfileError("manifest run fence mismatch".into()));
    }
    if ["split_policy_id", "label_policy_id"].iter().any(|field| {
      payload[*field]
        .as_str()
        .is_none_or(|value| !valid_identity(value, 64))
    }) {
      return Err(ProfileError("invalid manifest policy identity".into()));
    }
    let ids = payload["episodes"]
      .as_array()
      .ok_or_else(|| ProfileError("episodes missing".into()))?;
    let mut sources = Vec::new();
    let mut selected = BTreeSet::new();
    for reference in ids {
      let id = reference["episode_id"]
        .as_str()
        .ok_or_else(|| ProfileError("episode ID invalid".into()))?;
      if !selected.insert(id) {
        return Err(ProfileError("duplicate episode ID".into()));
      }
      if !valid_identity(id, 128) || !self.authorized.contains(id) {
        return Err(ProfileError("episode not authorized".into()));
      }
      let bytes = fs::read(self.root.join(format!("{id}.jsonl")))
        .map_err(|error| ProfileError(format!("episode unavailable: {error}")))?;
      if bytes.len() > 16_777_216 {
        return Err(ProfileError("episode exceeds export bound".into()));
      }
      let mut count = 0u64;
      for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
      {
        let record: Value = serde_json::from_slice(line)
          .map_err(|_| ProfileError("episode has malformed record".into()))?;
        NodeTypeRegistry::builtin().validate_data_payload("evidence_record@v0.2", &record)?;
        if record["run_id"] != self.run_id {
          return Err(ProfileError("episode run fence mismatch".into()));
        }
        count += 1;
      }
      let digest = format!("{:x}", Sha256::digest(&bytes));
      if reference["sha256"] != digest || reference["record_count"].as_u64() != Some(count) {
        return Err(ProfileError(
          "episode reference changed or incomplete".into(),
        ));
      }
      let split = if digest.as_bytes()[0] % 10 == 0 {
        "test"
      } else {
        "train"
      };
      sources.push(json!({"episode_id":id,"record_count":count,"sha256":digest,"split":split}));
    }
    sources.sort_by(|a, b| a["episode_id"].as_str().cmp(&b["episode_id"].as_str()));
    let manifest = json!({"run_id":self.run_id,"sources":sources,
      "split_policy_id":payload["split_policy_id"],"label_policy_id":payload["label_policy_id"]});
    let bytes = serde_json::to_vec(&manifest)
      .map_err(|error| ProfileError(format!("manifest encoding failed: {error}")))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let id = format!("manifest_{}", &digest[..24]);
    let path = self.root.join(format!("{id}.json"));
    match OpenOptions::new().write(true).create_new(true).open(&path) {
      Ok(mut file) => file
        .write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| ProfileError(format!("manifest write uncertain: {error}")))?,
      Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
        if fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
          return Err(ProfileError(
            "manifest digest collision or changed bytes".into(),
          ));
        }
      }
      Err(error) => return Err(ProfileError(format!("manifest store unavailable: {error}"))),
    }
    let output = TypedOutput {
      port: "manifest",
      data_type: "dataset_manifest_reference@v0.2",
      payload: json!({
      "run_id":self.run_id,"manifest_id":id,"episode_count":ids.len(),"sha256":digest}),
    };
    NodeTypeRegistry::builtin().validate_data_payload(output.data_type, &output.payload)?;
    self.latest = Some(output.clone());
    Ok(output)
  }

  pub fn read(&self) -> Option<&TypedOutput> {
    self.latest.as_ref()
  }
  pub fn health(&self) -> IsolatedHealth {
    IsolatedHealth {
      state: if self.stopped { "unavailable" } else { "ready" },
      reason: self.stopped.then_some("stopped"),
      generation: 1,
      source_time_ns: None,
    }
  }
  pub fn stop(&mut self) -> bool {
    self.stopped = true;
    true
  }
}
