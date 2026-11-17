use std::collections::{BTreeMap, BTreeSet};

use aether_node_profile::{
  isolated::{DatasetManifest, EmergencyStopService, EpisodeRecorder, IsolatedEstop, IsolatedNode},
  NodeTypeRegistry, ProfileError,
};
use serde_json::json;

fn settings(frame: &str) -> BTreeMap<String, toml::Value> {
  BTreeMap::from([("frame_id".into(), toml::Value::String(frame.into()))])
}

fn state(time: u64, position: f64) -> serde_json::Value {
  json!({"sample":{"source_time_ns":time,"clock":"monotonic","sequence":1,"frame_id":"arm"},
    "joint_names":["joint_1"],"position_rad":[position],"velocity_rad_s":[0.0]})
}

#[test]
fn mock_camera_emits_bounded_typed_frame_then_closes_input() {
  let registry = NodeTypeRegistry::builtin();
  let ty = registry.get("eos.hardware.mock.camera@v0.2").unwrap();
  assert!(ty.inputs.iter().all(|port| !port.required));
  let mut node = IsolatedNode::start(&ty.id, "run_1", &settings("camera")).unwrap();
  let input = json!({"source_time_ns":100,"sequence":1,"frame_id":"camera","clock":"monotonic"});
  let outputs = node.input("sample", &input, 200).unwrap();
  assert_eq!(outputs[0].data_type, "image_frame@v0.2");
  assert_eq!(outputs[0].payload["width_px"], 2);
  assert_eq!(outputs[0].payload["sample"]["clock"], "monotonic");
  assert_eq!(node.read(), outputs.first());
  assert_eq!(node.health().state, "ready");
  assert_eq!(node.health_at(1_000_000_101).state, "degraded");
  assert!(node.input("sample", &input, 1_000_000_101).is_err());
  assert!(node.stop());
  assert_eq!(node.health().state, "unavailable");
  assert!(node.input("sample", &input, 201).is_err());
}

#[test]
fn physical_features_and_guard_reject_bad_provenance_and_recover() {
  let mut features = IsolatedNode::start(
    "eos.perception.physical.features@v0.2",
    "run_1",
    &settings("arm"),
  )
  .unwrap();
  let mut guard =
    IsolatedNode::start("eos.safety.device.guard@v0.2", "run_1", &settings("arm")).unwrap();
  let feature = features.input("state", &state(100, 3.05), 200).unwrap();
  assert_eq!(feature[0].payload["state"], "detected");
  let status = guard.input("state", &state(100, 3.2), 200).unwrap();
  assert_eq!(status[0].payload["state"], "inhibited");
  assert!(guard.input("state", &state(100, 0.0), 300).is_err());
  let mut wrong_frame = state(200, 0.0);
  wrong_frame["sample"]["frame_id"] = json!("other");
  assert!(guard.input("state", &wrong_frame, 300).is_err());
  let ready = guard.input("state", &state(200, 0.0), 300).unwrap();
  assert_eq!(ready[0].payload["state"], "ready");
  assert!(guard.stop());
  assert!(features.stop());
}

#[test]
fn primitive_binding_only_validates_exact_target_and_unit() {
  let mut settings = settings("arm");
  settings.insert(
    "target_node".into(),
    toml::Value::String("eos/hardware/mock/arm".into()),
  );
  let mut node =
    IsolatedNode::start("eos.dispatcher.command.binding@v0.2", "run_1", &settings).unwrap();
  let request = json!({"source_time_ns":100,"frame_id":"arm","target_node":"eos/hardware/mock/arm",
    "primitive":"arm.delta@1.0.0","mode":"joint_positions","unit":"rad",
    "joint_deltas_rad":[0.01,0.0,0.0,0.0,0.0,0.0]});
  let output = node.input("request", &request, 200).unwrap();
  assert_eq!(output[0].payload["valid"], true);
  let mut wrong = request.clone();
  wrong["unit"] = json!("degree");
  assert!(node.input("request", &wrong, 201).is_err());
  wrong = request.clone();
  wrong["target_node"] = json!("eos/hardware/mock/other");
  assert!(node.input("request", &wrong, 201).is_err());
  assert!(node.stop());
}

#[test]
fn configuration_and_payload_boundaries_reject_invalid_input() {
  let mut bad = settings("camera");
  bad.insert("socket".into(), toml::Value::String("/tmp/private".into()));
  assert!(IsolatedNode::start("eos.hardware.mock.camera@v0.2", "run_1", &bad).is_err());
  assert!(IsolatedNode::start("eos.safety.device.estop@v0.2", "run_1", &settings("arm")).is_err());
  let registry = NodeTypeRegistry::builtin();
  assert!(registry
    .validate_data_payload(
      "sample_request@v0.2",
      &json!({"source_time_ns":1,"sequence":1})
    )
    .is_err());
  assert!(registry
    .validate_data_payload(
      "sample_request@v0.2",
      &json!({"source_time_ns":1,"sequence":1,"frame_id":"f","extra":true})
    )
    .is_err());
}

struct StopFixture {
  calls: usize,
}
impl EmergencyStopService for StopFixture {
  fn request_stop(&mut self, run_id: &str, reason: &str) -> Result<String, ProfileError> {
    assert_eq!((run_id, reason), ("run_1", "operator"));
    self.calls += 1;
    Ok("proof_1".into())
  }
}

#[test]
fn estop_delegates_to_service_and_keeps_latch_after_local_stop() {
  let mut node = IsolatedEstop::start("run_1", StopFixture { calls: 0 }).unwrap();
  let request = json!({"run_id":"run_1","reason":"operator","source_time_ns":100});
  let latch = node.input("stop", &request, 200).unwrap();
  assert_eq!(latch.payload["proof_ref"], "proof_1");
  assert!(node.input("stop", &request, 201).is_err());
  assert!(node.stop());
  assert_eq!(node.read().unwrap().payload["latched"], true);
}

fn record(id: &str) -> serde_json::Value {
  json!({"run_id":"run_1","record_id":id,"source_time_ns":100,
    "kind":"sample","source_ref":"arm_sample_1"})
}

#[test]
fn episode_and_manifest_preserve_provenance_and_visible_gaps() {
  let temp = tempfile::tempdir().unwrap();
  let mut recorder = EpisodeRecorder::start(temp.path(), "run_1", "ep_1", 128).unwrap();
  assert!(recorder.input("record", &record("one")).unwrap().is_empty());
  assert!(recorder.input("record", &record("one")).is_err());
  let gap = recorder.input("record", &record("two")).unwrap();
  assert_eq!(gap[0].data_type, "gap_record@v0.2");
  assert_eq!(gap[0].payload["lost_count"], 1);
  assert_eq!(recorder.health().state, "degraded");
  let episode = recorder.stop().unwrap();
  assert_eq!(episode.payload["record_count"], 1);
  assert!(recorder.input("record", &record("three")).is_err());
  let mut manifest =
    DatasetManifest::start(temp.path(), "run_1", BTreeSet::from(["ep_1".into()])).unwrap();
  let request = json!({"run_id":"run_1","episodes":[{
    "episode_id":"ep_1","record_count":episode.payload["record_count"],
    "sha256":episode.payload["sha256"]}],
    "split_policy_id":"split_1","label_policy_id":"labels_1"});
  let first = manifest.input("request", &request).unwrap();
  let second = manifest.input("request", &request).unwrap();
  assert_eq!(first, second);
  assert_eq!(first.payload["episode_count"], 1);
  assert!(manifest.stop());
  assert!(manifest.input("request", &request).is_err());
  let mut unauthorized =
    DatasetManifest::start(temp.path(), "run_1", BTreeSet::from(["other".into()])).unwrap();
  assert!(unauthorized.input("request", &request).is_err());
  let mut changed = request.clone();
  changed["episodes"][0]["sha256"] = json!("0".repeat(64));
  let mut verifier =
    DatasetManifest::start(temp.path(), "run_1", BTreeSet::from(["ep_1".into()])).unwrap();
  assert!(verifier.input("request", &changed).is_err());
}
