//! Built-in mock arm and its graph-bound observation endpoint.
use std::{
  collections::BTreeMap,
  fs::File,
  io::{BufRead, BufReader, Write},
  path::Path,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
  },
  time::{Duration, Instant},
};
use serde_json::{json, Value};

pub static STOP: AtomicBool = AtomicBool::new(false);
pub const DEMO_DELTA: [f64; 6] = [0.05, -0.04, 0.03, -0.02, 0.01, -0.01];
const MAX_ABS_JOINT_RAD: f64 = 314.0 / 100.0;
pub const MAX_EVENTS_PER_RUN: usize = 4096;
type DemoDelta = fn(&Value) -> Result<[f64; 6], Box<dyn std::error::Error>>;
pub fn valid_id(value: &str, max: usize) -> bool {
  !value.is_empty()
    && value.len() <= max
    && value
      .bytes()
      .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
pub fn valid_button_source(path: &str) -> bool {
  let parts: Vec<_> = path.split('/').collect();
  matches!(parts.as_slice(), ["isos", "ui", "button", id] if !id.is_empty() && id.len() <= 48 && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')))
}
pub struct Arm {
  pub joints: [f64; 6],
  pub joint_limit_rad: f64,
  pub joint_names: [String; 6],
  pub source_node: String,
  pub sequence: u64,
  pub sampled_at_monotonic_ns: u64,
  pub sampled_at: Instant,
  pub freshness_timeout: Duration,
  pub sample_state: &'static str,
  pub sample_reason: Option<&'static str>,
  pub previous: BTreeMap<String, (String, Value)>,
  pub journal: Option<File>,
  pub recovery_uncertain: bool,
}
/// Read-only hardware-layer endpoint bound to the graph's observation source.
/// It shares the sole mock arm adapter's sampler and cannot submit movement.
pub struct ArmObserver {
  pub node_path: String,
  pub payload_source_node: String,
  pub arm: Arc<Mutex<Arm>>,
}
impl ArmObserver {
  pub fn observe(&self, run_id: &str, revision: &str) -> Result<Value, &'static str> {
    let arm = self.arm.lock().map_err(|_| "arm state poisoned")?;
    if self.payload_source_node != self.node_path && self.payload_source_node != arm.source_node {
      return Err("observer source mismatch");
    }
    let mut observation = arm.observation(run_id, revision);
    observation["source_node"] = json!(self.payload_source_node);
    Ok(observation)
  }

  pub fn output(&self, request: &Value, run_id: &str, revision: &str) -> Value {
    if STOP.load(Ordering::SeqCst) {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_stopping"});
    }
    if request.as_object().is_none_or(|fields| fields.len() != 7)
      || request["schema_version"] != 1
      || request["core"] != "eos"
      || request["op"] != "observe_output"
      || request["run_id"] != run_id
      || request["revision"] != revision
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"observation_fence_stale"});
    }
    if request["node"] != self.node_path || request["port"] != "observation" {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"observation_route_unsupported"});
    }
    match self.observe(run_id, revision) {
      Ok(payload) if !STOP.load(Ordering::SeqCst) => {
        json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"node":self.node_path,"port":"observation","payload":payload})
      }
      Ok(_) => json!({"schema_version":1,"core":"eos","ok":false,"error":"worker_stopping"}),
      Err(_) => {
        json!({"schema_version":1,"core":"eos","ok":false,"error":"observation_unavailable"})
      }
    }
  }
}
impl Default for Arm {
  fn default() -> Self {
    let mut arm = Self {
      joints: [0.0; 6],
      joint_limit_rad: MAX_ABS_JOINT_RAD,
      joint_names: std::array::from_fn(|index| format!("joint_{}", index + 1)),
      source_node: "eos/hardware/mock/arm".into(),
      sequence: 0,
      sampled_at_monotonic_ns: 0,
      sampled_at: Instant::now(),
      freshness_timeout: Duration::from_millis(50),
      sample_state: "stale",
      sample_reason: Some("not_sampled"),
      previous: BTreeMap::new(),
      journal: None,
      recovery_uncertain: false,
    };
    arm.sample();
    arm
  }
}
fn monotonic_ns() -> Option<u64> {
  let mut time = libc::timespec {
    tv_sec: 0,
    tv_nsec: 0,
  };
  if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) } != 0 {
    return None;
  }
  u64::try_from(time.tv_sec)
    .ok()?
    .checked_mul(1_000_000_000)?
    .checked_add(u64::try_from(time.tv_nsec).ok()?)
}
impl Arm {
  pub fn sample(&mut self) {
    self.sequence = self.sequence.saturating_add(1);
    self.sampled_at = Instant::now();
    self.sampled_at_monotonic_ns = monotonic_ns().unwrap_or(0);
    let healthy = self.sampled_at_monotonic_ns > 0
      && self
        .joints
        .iter()
        .all(|value| value.is_finite() && value.abs() <= self.joint_limit_rad);
    self.sample_state = if healthy { "ready" } else { "degraded" };
    self.sample_reason = if healthy {
      None
    } else {
      Some("invalid_joint_sample")
    };
  }
  pub fn monitor_state(&self) -> (&'static str, Option<&'static str>) {
    if STOP.load(Ordering::SeqCst) {
      ("stopped", Some("stopped"))
    } else if self.sampled_at.elapsed() > self.freshness_timeout {
      ("stale", Some("sample_timeout"))
    } else {
      (self.sample_state, self.sample_reason)
    }
  }
  pub fn observation(&self, run_id: &str, revision: &str) -> Value {
    let (state, reason_code) = self.monitor_state();
    json!({"run_id":run_id,"revision":revision,"source_node":self.source_node,
      "sequence":self.sequence,"sampled_at_monotonic_ns":self.sampled_at_monotonic_ns,
      "joint_names":self.joint_names,"joint_positions_rad":if state == "ready" { json!(self.joints) } else { Value::Null },
      "state":state,"reason_code":reason_code})
  }
  fn record(&mut self, event_id: &str, source_node: &str, result: &Value) -> std::io::Result<()> {
    if let Some(journal) = self.journal.as_mut() {
      serde_json::to_writer(
        &mut *journal,
        &json!({"event_id":event_id,"source_node":source_node,"result":result}),
      )?;
      journal.write_all(b"\n")?;
      journal.sync_all()?;
    }
    Ok(())
  }
  pub fn load_journal(&mut self, path: &Path, run_id: &str, revision: &str) -> std::io::Result<()> {
    if !path.exists() {
      return Ok(());
    }
    let reader = BufReader::new(File::open(path)?);
    for line in reader.lines() {
      let line = line?;
      let entry: Value = serde_json::from_str(&line)?;
      let event_id = entry["event_id"]
        .as_str()
        .ok_or_else(|| std::io::Error::other("journal event missing"))?;
      let source = entry["source_node"]
        .as_str()
        .ok_or_else(|| std::io::Error::other("journal source missing"))?;
      let result = &entry["result"];
      if !valid_id(event_id, 64)
        || !valid_button_source(source)
        || result["run_id"] != run_id
        || result["revision"] != revision
        || result["event_id"] != event_id
        || result["command_id"] != format!("arm-{event_id}")
        || result["ok"] != true
      {
        return Err(std::io::Error::other("journal identity mismatch"));
      }
      if result["status"] == "succeeded" {
        let values = result["joint_positions_rad"]
          .as_array()
          .ok_or_else(|| std::io::Error::other("journal positions missing"))?;
        if values.len() != 6 {
          return Err(std::io::Error::other("journal positions invalid"));
        }
        let mut positions = [0.0; 6];
        for (index, value) in values.iter().enumerate() {
          positions[index] = value
            .as_f64()
            .filter(|value| value.is_finite() && value.abs() <= self.joint_limit_rad)
            .ok_or_else(|| std::io::Error::other("journal positions invalid"))?;
        }
        self.joints = positions;
      }
      self
        .previous
        .insert(event_id.into(), (source.into(), result.clone()));
    }
    for (_, result) in self.previous.values_mut() {
      if matches!(result["status"].as_str(), Some("accepted" | "running")) {
        let event_id = result["event_id"].as_str().unwrap_or("");
        *result = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"outcome_unknown"});
        self.recovery_uncertain = true;
      }
    }
    self.sample();
    Ok(())
  }
  pub fn reserve(&mut self, request: &Value, run_id: &str, revision: &str) -> Value {
    let event_id = request["event_id"].as_str().unwrap_or("");
    let source_node = request["source_node"].as_str().unwrap_or("");
    if request["schema_version"] != 1
      || request["op"] != "activate"
      || request["core"] != "eos"
      || request["run_id"] != run_id
      || request["revision"] != revision
      || request["kind"] != "activate"
      || !valid_id(event_id, 64)
      || !valid_button_source(source_node)
      || request.as_object().is_none_or(|fields| fields.len() != 8)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_activation"});
    }
    if let Some((prior_source, _)) = self.previous.get(event_id) {
      return if prior_source == source_node {
        json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"accepted"})
      } else {
        json!({"schema_version":1,"core":"eos","ok":false,"error":"event_conflict"})
      };
    }
    if STOP.load(Ordering::SeqCst) {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"stopping"});
    }
    if self.recovery_uncertain {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"outcome_unknown"});
    }
    let (state, _) = self.monitor_state();
    if state != "ready" {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":if state == "stale" {"monitor_stale"} else {"monitor_unavailable"}});
    }
    if self.previous.len() >= MAX_EVENTS_PER_RUN {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"event_capacity"});
    }
    let accepted = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"accepted"});
    if self.record(event_id, source_node, &accepted).is_err() {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"outcome_unknown"});
    }
    self
      .previous
      .insert(event_id.into(), (source_node.into(), accepted.clone()));
    accepted
  }
  pub fn execute_reserved(
    &mut self,
    event_id: &str,
    run_id: &str,
    revision: &str,
    demo_joint_delta: DemoDelta,
  ) {
    let Some((source_node, prior)) = self.previous.get(event_id).cloned() else {
      return;
    };
    if prior["status"] != "accepted" {
      return;
    }
    if self.recovery_uncertain {
      let unknown = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"outcome_unknown"});
      let _ = self.record(event_id, &source_node, &unknown);
      self
        .previous
        .insert(event_id.into(), (source_node, unknown));
      return;
    }
    let status = if STOP.load(Ordering::SeqCst) {
      "cancelled"
    } else {
      "running"
    };
    let running = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":status});
    if self.record(event_id, &source_node, &running).is_err() {
      self.recovery_uncertain = true;
      let unknown = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"outcome_unknown"});
      self
        .previous
        .insert(event_id.into(), (source_node, unknown));
      return;
    }
    self
      .previous
      .insert(event_id.into(), (source_node.clone(), running));
    if status == "cancelled" {
      return;
    }
    let delta = demo_joint_delta(
      &json!({"run_id":run_id,"revision":revision,"event_id":event_id,"source_node":source_node}),
    );
    let mut target = self.joints;
    if let Ok(delta) = &delta {
      for (joint, value) in target.iter_mut().zip(delta) {
        *joint += value;
      }
    }
    let result = if delta.is_err() {
      json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"failed","error":"skill_unavailable"})
    } else if self.monitor_state().0 != "ready" {
      json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"failed","error":"monitor_unavailable"})
    } else if target
      .iter()
      .any(|joint| !joint.is_finite() || joint.abs() > self.joint_limit_rad)
    {
      json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"failed","error":"joint_limit"})
    } else {
      self.joints = target;
      self.sample();
      json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"succeeded","joint_positions_rad":self.joints})
    };
    if self.record(event_id, &source_node, &result).is_err() {
      self.recovery_uncertain = true;
      let uncertain = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"outcome_unknown"});
      self
        .previous
        .insert(event_id.into(), (source_node, uncertain));
    } else {
      self.previous.insert(event_id.into(), (source_node, result));
    }
  }
  pub fn result(&self, request: &Value, run_id: &str, revision: &str) -> Value {
    let event_id = request["event_id"].as_str().unwrap_or("");
    if request["schema_version"] != 1
      || request["core"] != "eos"
      || request["op"] != "result"
      || request["run_id"] != run_id
      || request["revision"] != revision
      || !valid_id(event_id, 64)
      || request.as_object().is_none_or(|fields| fields.len() != 6)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_result_request"});
    }
    self
      .previous
      .get(event_id)
      .map(|(_, result)| result.clone())
      .unwrap_or_else(
        || json!({"schema_version":1,"core":"eos","ok":false,"error":"event_not_found"}),
      )
  }
  pub fn activate(&mut self, request: &Value, run_id: &str, revision: &str) -> Value {
    let event_id = request["event_id"].as_str().unwrap_or("");
    let source_node = request["source_node"].as_str().unwrap_or("");
    if request["schema_version"] != 1
      || request["op"] != "activate"
      || request["core"] != "eos"
      || request["run_id"] != run_id
      || request["revision"] != revision
      || request["kind"] != "activate"
      || !valid_id(event_id, 64)
      || !valid_button_source(source_node)
      || request.as_object().is_none_or(|fields| fields.len() != 8)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"invalid_activation"});
    }
    if STOP.load(Ordering::SeqCst) {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"stopping"});
    }
    let (state, _) = self.monitor_state();
    if state != "ready" {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":if state == "stale" { "monitor_stale" } else { "monitor_unavailable" }});
    }
    if let Some((previous_source, result)) = self.previous.get(event_id) {
      return if previous_source == source_node {
        result.clone()
      } else {
        json!({"schema_version":1,"core":"eos","ok":false,"error":"event_conflict"})
      };
    }
    if self.previous.len() >= MAX_EVENTS_PER_RUN {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"event_capacity"});
    }
    // The built-in skill emits a typed six-joint delta. Dispatcher is the only movement gate.
    if DEMO_DELTA
      .iter()
      .any(|value| !value.is_finite() || value.abs() > 0.25)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"delta_invalid"});
    }
    let mut target = self.joints;
    for (joint, delta) in target.iter_mut().zip(DEMO_DELTA) {
      *joint += delta;
    }
    if target
      .iter()
      .any(|joint| !joint.is_finite() || joint.abs() > self.joint_limit_rad)
    {
      return json!({"schema_version":1,"core":"eos","ok":false,"error":"joint_limit"});
    }
    self.joints = target;
    self.sample();
    let result = json!({"schema_version":1,"core":"eos","ok":true,"run_id":run_id,"revision":revision,"event_id":event_id,"command_id":format!("arm-{event_id}"),"status":"succeeded","joint_positions_rad":self.joints});
    self
      .previous
      .insert(event_id.into(), (source_node.into(), result.clone()));
    result
  }
}
