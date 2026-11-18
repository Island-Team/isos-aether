//! Complete graph-declared camera to EOS transfer to IsOS monitor paths.

use super::edge_admission::Edge;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Binding {
  pub monitor_path: String,
  pub transfer_path: String,
  pub source_path: String,
  pub label: String,
  pub interval_ms: Option<u64>,
  #[serde(skip_serializing)]
  pub single_frame: bool,
}

fn node<'a>(graph: &'a Value, path: &str) -> Option<&'a Value> {
  let [core, layer, group, id] = <[&str; 4]>::try_from(path.split('/').collect::<Vec<_>>()).ok()?;
  graph.pointer(&format!(
    "/cores/{core}/layers/{layer}/groups/{group}/nodes/{id}"
  ))
}

fn source(input: &Value) -> &Value {
  input.get("sources").unwrap_or(input)
}

pub fn bindings(graph: &Value, _catalog: &Value, edges: &[Edge]) -> Result<Vec<Binding>, String> {
  let mut result = Vec::new();
  let Some(monitors) = graph
    .pointer("/cores/isos/layers/ui/groups/vision/nodes")
    .and_then(Value::as_object)
  else {
    return Ok(result);
  };
  for (id, monitor) in monitors {
    if monitor["node_type"] != "isos.ui.vision.monitor@v0.2" {
      continue;
    }
    let monitor_path = format!("isos/ui/vision/{id}");
    let monitor_input = source(&monitor["inputs"]["frame"]);
    let transfer_path = monitor_input["node"]
      .as_str()
      .ok_or_else(|| format!("{monitor_path}: vision monitor requires a transfer frame input"))?;
    let transfer = node(graph, transfer_path)
      .ok_or_else(|| format!("{monitor_path}: transfer node is missing"))?;
    if transfer["node_type"] != "eos.interface.egress.vision_flow@v0.2"
      || monitor_input["output"] != "frame"
      || !edges.iter().any(|edge| {
        edge.source == transfer_path
          && edge.target == monitor_path
          && edge.output == "frame"
          && edge.input == "frame"
          && edge.routed
      })
    {
      return Err(format!(
        "{monitor_path}: vision monitor requires a routed EOS transfer"
      ));
    }
    let transfer_input = source(&transfer["inputs"]["frame"]);
    let source_path = transfer_input["node"]
      .as_str()
      .ok_or_else(|| format!("{transfer_path}: vision transfer requires a camera frame input"))?;
    node(graph, source_path).ok_or_else(|| format!("{transfer_path}: camera source is missing"))?;
    if !source_path.starts_with("eos/")
      || transfer_input["output"] != "frame"
      || !edges.iter().any(|edge| {
        edge.source == source_path
          && edge.target == transfer_path
          && edge.output == "frame"
          && edge.input == "frame"
          && edge.data_type == "image_frame@v0.2"
          && edge.routed
      })
    {
      return Err(format!(
        "{transfer_path}: vision transfer requires a routed camera frame"
      ));
    }
    let edge_rate = edges
      .iter()
      .filter(|edge| {
        (edge.source == source_path && edge.target == transfer_path)
          || (edge.source == transfer_path && edge.target == monitor_path)
      })
      .filter_map(|edge| edge.frequency_hz)
      .reduce(f64::min);
    // Only a downstream delivery cap controls this interval. The package's
    // optional official frequency does not schedule or throttle capture.
    let interval_ms = edge_rate.and_then(|rate| {
      let period_ms = (1000.0 / rate).ceil();
      (period_ms.is_finite() && period_ms <= 31_536_000_000.0).then(|| period_ms.max(1.0) as u64)
    });
    let single_frame = edge_rate.is_some() && interval_ms.is_none();
    result.push(Binding {
      monitor_path,
      transfer_path: transfer_path.into(),
      source_path: source_path.into(),
      label: monitor["label"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or(id)
        .into(),
      interval_ms,
      single_frame,
    });
  }
  Ok(result)
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn edge(source: &str, target: &str) -> Edge {
    Edge {
      path: String::new(),
      source: source.into(),
      output: "frame".into(),
      target: target.into(),
      input: "frame".into(),
      route_class: "owner_bridge",
      fixed: false,
      qos_eligible: false,
      routed: true,
      priority: 0,
      frequency_hz: None,
      data_type: "image_frame@v0.2".into(),
      transport: "unix_socket".into(),
      delivery_mode: "stream".into(),
    }
  }

  #[test]
  fn requires_complete_routed_camera_transfer_monitor_path() {
    let graph = json!({"cores":{"eos":{"layers":{"hardware":{"groups":{"camera":{"nodes":{"front":{"node_type":"eos.hardware.camera.device@v0.2"}}}}},
      "interface":{"groups":{"egress":{"nodes":{"out":{"node_type":"eos.interface.egress.vision_flow@v0.2","inputs":{"frame":{"node":"eos/hardware/camera/front","output":"frame"}}}}}}}}},
      "isos":{"layers":{"ui":{"groups":{"vision":{"nodes":{"view":{"node_type":"isos.ui.vision.monitor@v0.2","label":"Front","inputs":{"frame":{"node":"eos/interface/egress/out","output":"frame"}}}}}}}}}}});
    let edges = vec![
      edge("eos/hardware/camera/front", "eos/interface/egress/out"),
      edge("eos/interface/egress/out", "isos/ui/vision/view"),
    ];
    let catalog = json!({"types":[{"id":"eos.hardware.camera.device@v0.2","outputs":[{"name":"frame","frequency_hz":30.0}]}]});
    let bound = bindings(&graph, &catalog, &edges).unwrap();
    assert_eq!(bound[0].label, "Front");
    assert_eq!(bound[0].source_path, "eos/hardware/camera/front");
    assert_eq!(bound[0].interval_ms, None);
    assert!(!bound[0].single_frame);
    assert_eq!(
      bindings(&graph, &json!({"types":[]}), &edges).unwrap(),
      bound
    );
    let mut capped = edges.clone();
    capped[1].frequency_hz = Some(1.0);
    assert_eq!(
      bindings(&graph, &catalog, &capped).unwrap()[0].interval_ms,
      Some(1000)
    );
    capped[1].frequency_hz = Some(f64::MIN_POSITIVE);
    assert_eq!(
      bindings(&graph, &catalog, &capped).unwrap()[0].interval_ms,
      None
    );
    assert!(bindings(&graph, &catalog, &capped).unwrap()[0].single_frame);
    assert!(bindings(&graph, &catalog, &edges[1..]).is_err());
  }
}
