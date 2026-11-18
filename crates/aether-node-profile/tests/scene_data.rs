use aether_node_profile::{scene_data::validate_scene_payload, NodeTypeRegistry};
use serde_json::{json, Value};

const IDS: &[&str] = &[
  "depth_frame_ref@v0.2",
  "segmentation_mask_ref@v0.2",
  "point_cloud_ref@v0.2",
  "physical_detection@v0.2",
  "imu_sample@v0.2",
  "force_torque_sample@v0.2",
  "tactile_sample@v0.2",
  "contact_event@v0.2",
  "audio_chunk_ref@v0.2",
];

fn example(id: &str) -> Value {
  let mut value = json!({"sample":{"source_time_ns":100,"clock":"monotonic","sequence":7,
    "frame_id":"sensor_frame","source_id":"sensor_1"},"calibration_id":"cal_1","validity":"valid"});
  let fields = match id {
    "depth_frame_ref@v0.2" => json!({"artifact_id":"depth_1","sha256":"0".repeat(64),
      "byte_length":8,"width_px":2,"height_px":2,"encoding":"u16le",
      "metres_per_unit":0.001,"uncertainty_m":0.01}),
    "segmentation_mask_ref@v0.2" => json!({"artifact_id":"mask_1","sha256":"0".repeat(64),
      "byte_length":4,"width_px":2,"height_px":2,"source_image_id":"image_1",
      "mask_kind":"binary","encoding":"u8"}),
    "point_cloud_ref@v0.2" => json!({"artifact_id":"cloud_1","sha256":"0".repeat(64),
      "byte_length":24,"point_count":2,"unit":"m","encoding":"xyz_f32_le","uncertainty_m":0.01}),
    "physical_detection@v0.2" => json!({"detection_id":"det_1","kind":"obstacle",
      "position_m":[1.0,0.0,0.0],"confidence":0.9,"uncertainty_m":0.1}),
    "imu_sample@v0.2" => json!({"acceleration_m_s2":[0.0,0.0,9.8],
      "angular_velocity_rad_s":[0.0,0.0,0.01],"uncertainty_m_s2":0.1}),
    "force_torque_sample@v0.2" => json!({"force_n":[1.0,0.0,0.0],
      "torque_nm":[0.0,0.1,0.0],"uncertainty_n":0.05}),
    "tactile_sample@v0.2" => json!({"sensor_id":"pad_1","pressure_pa":[100.0,110.0],
      "uncertainty_pa":5.0}),
    "contact_event@v0.2" => json!({"event_id":"contact_1","body_a":"finger_1",
      "body_b":"object_1","state":"began","force_n":1.5}),
    "audio_chunk_ref@v0.2" => json!({"artifact_id":"audio_1","sha256":"0".repeat(64),
      "byte_length":320,"encoding":"pcm_s16le","sample_rate_hz":16000,
      "channels":1,"duration_ms":10}),
    _ => panic!("unknown fixture"),
  };
  value
    .as_object_mut()
    .unwrap()
    .extend(fields.as_object().unwrap().clone());
  value
}

#[test]
fn scene_descriptors_have_valid_examples_and_reject_missing_identity_frame_calibration_and_age() {
  let registry = NodeTypeRegistry::builtin();
  for id in IDS {
    let descriptor = registry.data_type(id).unwrap();
    assert_eq!(descriptor.codec, "json_v1");
    assert_eq!(descriptor.max_payload_bytes, 4096);
    let valid = example(id);
    validate_scene_payload(&registry, id, &valid, 200, "sensor_frame", "cal_1").unwrap();
    let mut missing = valid.clone();
    missing["sample"]
      .as_object_mut()
      .unwrap()
      .remove("source_id");
    assert!(
      validate_scene_payload(&registry, id, &missing, 200, "sensor_frame", "cal_1").is_err(),
      "{id}"
    );
    assert!(
      validate_scene_payload(&registry, id, &valid, 200, "wrong_frame", "cal_1").is_err(),
      "{id}"
    );
    assert!(
      validate_scene_payload(&registry, id, &valid, 200, "sensor_frame", "wrong_cal").is_err(),
      "{id}"
    );
    assert!(
      validate_scene_payload(
        &registry,
        id,
        &valid,
        1_000_000_101,
        "sensor_frame",
        "cal_1"
      )
      .is_err(),
      "{id}"
    );
    assert!(
      validate_scene_payload(&registry, id, &valid, 99, "sensor_frame", "cal_1").is_err(),
      "{id}"
    );
    let mut oversized = valid.clone();
    oversized["unbounded"] = json!("x".repeat(5000));
    assert!(
      validate_scene_payload(&registry, id, &oversized, 200, "sensor_frame", "cal_1").is_err(),
      "{id}"
    );
  }
}

#[test]
fn scene_descriptors_reject_wrong_units_dimensions_encoding_and_unsafe_references() {
  let registry = NodeTypeRegistry::builtin();
  for (id, field, value) in [
    ("depth_frame_ref@v0.2", "metres_per_unit", json!(0.0)),
    ("segmentation_mask_ref@v0.2", "width_px", json!(0)),
    ("point_cloud_ref@v0.2", "unit", json!("mm")),
    ("physical_detection@v0.2", "confidence", json!(1.1)),
    ("imu_sample@v0.2", "acceleration_m_s2", json!([0.0, 0.0])),
    ("force_torque_sample@v0.2", "torque_nm", json!([0.0, 0.0])),
    ("tactile_sample@v0.2", "pressure_pa", json!([])),
    ("contact_event@v0.2", "state", json!("grasped")),
    ("audio_chunk_ref@v0.2", "encoding", json!("mp3")),
  ] {
    let mut invalid = example(id);
    invalid[field] = value;
    assert!(
      validate_scene_payload(&registry, id, &invalid, 200, "sensor_frame", "cal_1").is_err(),
      "{id}"
    );
  }
  let mut path = example("depth_frame_ref@v0.2");
  path["artifact_id"] = json!("../../private");
  assert!(validate_scene_payload(
    &registry,
    "depth_frame_ref@v0.2",
    &path,
    200,
    "sensor_frame",
    "cal_1"
  )
  .is_err());
  let mut bad_digest = example("audio_chunk_ref@v0.2");
  bad_digest["sha256"] = json!("A".repeat(64));
  assert!(validate_scene_payload(
    &registry,
    "audio_chunk_ref@v0.2",
    &bad_digest,
    200,
    "sensor_frame",
    "cal_1"
  )
  .is_err());
  for id in [
    "depth_frame_ref@v0.2",
    "segmentation_mask_ref@v0.2",
    "point_cloud_ref@v0.2",
  ] {
    let mut wrong_length = example(id);
    wrong_length["byte_length"] = json!(1);
    assert!(
      validate_scene_payload(&registry, id, &wrong_length, 200, "sensor_frame", "cal_1").is_err(),
      "{id}"
    );
  }
}
