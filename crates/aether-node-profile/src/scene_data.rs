//! Strict, bounded JSON metadata for EOS physical scene sources.
use serde_json::{json, Map, Value};

use crate::{NodeTypeRegistry, ProfileError};

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

const MAX_AGE_NS: u64 = 1_000_000_000;

pub(crate) fn is_scene_type(id: &str) -> bool {
  IDS.contains(&id)
}

fn identity(max: usize) -> Value {
  json!({"type":"string","minLength":1,"maxLength":max,"pattern":"^[A-Za-z0-9_.-]+$"})
}

fn digest() -> Value {
  json!({"type":"string","pattern":"^[0-9a-f]{64}$"})
}

fn vector3(unit: &str) -> Value {
  json!({"type":"array","minItems":3,"maxItems":3,"items":{"type":"number","minimum":-1000000,"maximum":1000000},"x-unit":unit})
}

fn descriptor(sample: &Value, extra: &[(&str, Value)]) -> Value {
  let mut properties = Map::new();
  properties.insert("sample".into(), sample.clone());
  properties.insert("calibration_id".into(), identity(128));
  properties.insert(
    "validity".into(),
    json!({"enum":["valid","invalid","unknown"]}),
  );
  let mut required = vec!["sample", "calibration_id", "validity"];
  for (name, value) in extra {
    properties.insert((*name).into(), value.clone());
    required.push(name);
  }
  json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}

fn reference(sample: &Value, extra: &[(&str, Value)]) -> Value {
  let mut fields = vec![
    ("artifact_id", identity(128)),
    ("sha256", digest()),
    (
      "byte_length",
      json!({"type":"integer","minimum":1,"maximum":100000000,"x-unit":"bytes"}),
    ),
  ];
  fields.extend_from_slice(extra);
  descriptor(sample, &fields)
}

pub(crate) fn schema(id: &str, common_sample: &Value) -> Option<Value> {
  if !is_scene_type(id) {
    return None;
  }
  let mut sample = common_sample.clone();
  sample["required"] = json!([
    "source_time_ns",
    "clock",
    "sequence",
    "frame_id",
    "source_id"
  ]);
  sample["properties"]["clock"] = json!({"const":"monotonic"});
  sample["properties"]["source_id"] = identity(128);
  let dimensions = [
    (
      "width_px",
      json!({"type":"integer","minimum":1,"maximum":8192,"x-unit":"px"}),
    ),
    (
      "height_px",
      json!({"type":"integer","minimum":1,"maximum":8192,"x-unit":"px"}),
    ),
  ];
  Some(match id {
    "depth_frame_ref@v0.2" => reference(
      &sample,
      &[
        dimensions[0].clone(),
        dimensions[1].clone(),
        ("encoding", json!({"const":"u16le"})),
        (
          "metres_per_unit",
          json!({"type":"number","exclusiveMinimum":0,"maximum":1,"x-unit":"m/unit"}),
        ),
        (
          "uncertainty_m",
          json!({"type":"number","minimum":0,"maximum":100,"x-unit":"m"}),
        ),
      ],
    ),
    "segmentation_mask_ref@v0.2" => reference(
      &sample,
      &[
        dimensions[0].clone(),
        dimensions[1].clone(),
        ("source_image_id", identity(128)),
        ("mask_kind", json!({"const":"binary"})),
        ("encoding", json!({"const":"u8"})),
      ],
    ),
    "point_cloud_ref@v0.2" => reference(
      &sample,
      &[
        (
          "point_count",
          json!({"type":"integer","minimum":1,"maximum":10000000}),
        ),
        ("unit", json!({"const":"m"})),
        ("encoding", json!({"const":"xyz_f32_le"})),
        (
          "uncertainty_m",
          json!({"type":"number","minimum":0,"maximum":100,"x-unit":"m"}),
        ),
      ],
    ),
    "physical_detection@v0.2" => descriptor(
      &sample,
      &[
        ("detection_id", identity(128)),
        (
          "kind",
          json!({"enum":["obstacle","person_proximity","unknown"]}),
        ),
        ("position_m", vector3("m")),
        (
          "confidence",
          json!({"type":"number","minimum":0,"maximum":1}),
        ),
        (
          "uncertainty_m",
          json!({"type":"number","minimum":0,"maximum":100,"x-unit":"m"}),
        ),
      ],
    ),
    "imu_sample@v0.2" => descriptor(
      &sample,
      &[
        ("acceleration_m_s2", vector3("m/s2")),
        ("angular_velocity_rad_s", vector3("rad/s")),
        (
          "uncertainty_m_s2",
          json!({"type":"number","minimum":0,"maximum":100,"x-unit":"m/s2"}),
        ),
      ],
    ),
    "force_torque_sample@v0.2" => descriptor(
      &sample,
      &[
        ("force_n", vector3("N")),
        ("torque_nm", vector3("N.m")),
        (
          "uncertainty_n",
          json!({"type":"number","minimum":0,"maximum":100000,"x-unit":"N"}),
        ),
      ],
    ),
    "tactile_sample@v0.2" => descriptor(
      &sample,
      &[
        ("sensor_id", identity(128)),
        (
          "pressure_pa",
          json!({"type":"array","minItems":1,"maxItems":32,"items":{"type":"number","minimum":0,"maximum":10000000},"x-unit":"Pa"}),
        ),
        (
          "uncertainty_pa",
          json!({"type":"number","minimum":0,"maximum":10000000,"x-unit":"Pa"}),
        ),
      ],
    ),
    "contact_event@v0.2" => descriptor(
      &sample,
      &[
        ("event_id", identity(128)),
        ("body_a", identity(128)),
        ("body_b", identity(128)),
        ("state", json!({"enum":["began","ended","unknown"]})),
        (
          "force_n",
          json!({"type":["number","null"],"minimum":0,"maximum":100000,"x-unit":"N"}),
        ),
      ],
    ),
    "audio_chunk_ref@v0.2" => reference(
      &sample,
      &[
        ("encoding", json!({"const":"pcm_s16le"})),
        (
          "sample_rate_hz",
          json!({"type":"integer","minimum":8000,"maximum":192000,"x-unit":"Hz"}),
        ),
        (
          "channels",
          json!({"type":"integer","minimum":1,"maximum":8}),
        ),
        (
          "duration_ms",
          json!({"type":"integer","minimum":1,"maximum":1000,"x-unit":"ms"}),
        ),
      ],
    ),
    _ => unreachable!("checked scene type"),
  })
}

/// Validate one physical sample against its exact descriptor and caller's run-local clock/frame.
pub fn validate_scene_payload(
  registry: &NodeTypeRegistry,
  id: &str,
  payload: &Value,
  now_ns: u64,
  frame_id: &str,
  calibration_id: &str,
) -> Result<(), ProfileError> {
  if !is_scene_type(id) {
    return Err(ProfileError("unknown EOS scene type".into()));
  }
  registry.validate_data_payload(id, payload)?;
  let time = payload["sample"]["source_time_ns"]
    .as_u64()
    .ok_or_else(|| ProfileError("scene source time missing".into()))?;
  if time > now_ns || now_ns - time > MAX_AGE_NS {
    return Err(ProfileError("scene sample stale or future".into()));
  }
  if payload["sample"]["frame_id"] != frame_id {
    return Err(ProfileError("scene frame mismatch".into()));
  }
  if payload["calibration_id"] != calibration_id {
    return Err(ProfileError("scene calibration mismatch".into()));
  }
  let exact_bytes = match id {
    "depth_frame_ref@v0.2" => {
      Some(payload["width_px"].as_u64().unwrap() * payload["height_px"].as_u64().unwrap() * 2)
    }
    "segmentation_mask_ref@v0.2" => {
      Some(payload["width_px"].as_u64().unwrap() * payload["height_px"].as_u64().unwrap())
    }
    "point_cloud_ref@v0.2" => Some(payload["point_count"].as_u64().unwrap() * 12),
    _ => None,
  };
  if exact_bytes.is_some_and(|bytes| payload["byte_length"].as_u64() != Some(bytes)) {
    return Err(ProfileError("scene artifact byte length mismatch".into()));
  }
  Ok(())
}
