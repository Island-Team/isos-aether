//! Reusable validation for EOS camera frames admitted to a graph Vision transfer.
use crate::NodeTypeRegistry;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

pub fn validate_frame(frame: &Value, registry: &NodeTypeRegistry) -> Result<(), String> {
  registry
    .validate_data_payload("image_frame@v0.2", frame)
    .map_err(|error| error.to_string())?;
  let width = frame["width_px"].as_u64().ok_or("missing width")?;
  let height = frame["height_px"].as_u64().ok_or("missing height")?;
  if width > 8192 || height > 8192 {
    return Err("frame dimensions exceed bound".into());
  }
  let raw = STANDARD
    .decode(frame["data"].as_str().ok_or("missing data")?)
    .map_err(|_| "invalid frame base64")?;
  if raw.is_empty() {
    return Err("frame bytes are empty".into());
  }
  match frame["encoding"].as_str() {
    Some("rgb8") => {
      let expected = width.checked_mul(height).and_then(|n| n.checked_mul(3));
      if expected != Some(raw.len() as u64) {
        return Err("RGB frame dimensions mismatch".into());
      }
    }
    Some("jpeg") => {
      if raw.len() < 4 || !raw.starts_with(&[0xff, 0xd8]) || !raw.ends_with(&[0xff, 0xd9]) {
        return Err("invalid JPEG markers".into());
      }
      if jpeg_dimensions(&raw) != Some((width, height)) {
        return Err("JPEG dimensions mismatch".into());
      }
    }
    _ => return Err("unsupported frame encoding".into()),
  }
  Ok(())
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u64, u64)> {
  let mut at = 2;
  while at + 4 <= bytes.len() {
    if bytes[at] != 0xff {
      return None;
    }
    while at < bytes.len() && bytes[at] == 0xff {
      at += 1;
    }
    let marker = *bytes.get(at)?;
    at += 1;
    if marker == 0xd9 || marker == 0xda {
      return None;
    }
    if matches!(marker, 0x01 | 0xd0..=0xd7) {
      continue;
    }
    let length = u16::from_be_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]) as usize;
    if length < 2 || at.checked_add(length)? > bytes.len() {
      return None;
    }
    if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
      if length < 7 {
        return None;
      }
      return Some((
        u16::from_be_bytes([bytes[at + 5], bytes[at + 6]]) as u64,
        u16::from_be_bytes([bytes[at + 3], bytes[at + 4]]) as u64,
      ));
    }
    at += length;
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn admits_large_rgb_frame_within_registered_payload_budget() {
    let registry = NodeTypeRegistry::builtin();
    let raw = vec![0u8; 2560 * 1600 * 3];
    let frame = json!({"sample":{"source_time_ns":1,"clock":"simulation","sequence":1,
      "frame_id":"robocasa-camera"},"width_px":2560,"height_px":1600,
      "encoding":"rgb8","data":STANDARD.encode(&raw)});
    assert!(validate_frame(&frame, &registry).is_ok());
    let mut invalid = frame;
    invalid["width_px"] = json!(2561);
    assert!(validate_frame(&invalid, &registry).is_err());
  }

  #[test]
  fn checks_camera_bytes_and_dimensions() {
    let registry = NodeTypeRegistry::builtin();
    let mut frame = json!({"sample":{"source_time_ns":1,"clock":"simulation","sequence":1,
      "frame_id":"cam-1"},"width_px":2,"height_px":1,"encoding":"rgb8",
      "data":STANDARD.encode([1,2,3,4,5,6])});
    assert!(validate_frame(&frame, &registry).is_ok());
    frame["width_px"] = json!(3);
    assert!(validate_frame(&frame, &registry).is_err());
    frame["width_px"] = json!(2);
    frame["data"] = json!("not base64");
    assert!(validate_frame(&frame, &registry).is_err());
    frame["encoding"] = json!("jpeg");
    frame["data"] =
      json!(STANDARD.encode([0xff, 0xd8, 0xff, 0xc0, 0, 7, 8, 0, 1, 0, 2, 0xff, 0xd9]));
    assert!(validate_frame(&frame, &registry).is_ok());
    frame["height_px"] = json!(2);
    assert!(validate_frame(&frame, &registry).is_err());
  }
}
