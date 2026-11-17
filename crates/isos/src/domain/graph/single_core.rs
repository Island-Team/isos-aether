//! Read-only owner views for the three-core graph.

use super::graph_contract::Diagnostic;
use serde_json::{Value, json};
use std::collections::BTreeMap;

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

/// Return only the layers owned by the requested process.
pub fn owner_projection(graph: &Value, role: &str) -> Result<Value, Diagnostic> {
  if graph["version"] != "0.2.1" {
    return Err(diagnostic(
      "/version",
      "unsupported_graph_version",
      "owner projection requires graph 0.2.1",
    ));
  }
  match role {
    "eos" | "ei" => (),
    _ => {
      return Err(diagnostic(
        "/owner",
        "unsupported_owner",
        format!("unknown owner {role}"),
      ));
    }
  }
  let layers = graph
    .pointer(&format!("/cores/{role}/layers"))
    .and_then(Value::as_object)
    .ok_or_else(|| {
      diagnostic(
        format!("/cores/{role}/layers"),
        "invalid_graph",
        "owner layers must be an object",
      )
    })?;
  Ok(json!({"version":graph["version"],"robot":graph["robot"],"cores":{(role):{"layers":layers}}}))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn owner_projection_contains_only_its_own_core() {
    let graph = json!({"version":"0.2.1","robot":{"id":"r","name":"R"},"cores":{
      "isos":{"layers":{"ui":{"groups":{}}}},
      "ei":{"layers":{"io":{"groups":{}}}},
      "eos":{"layers":{"hardware":{"groups":{}}}}
    }});
    let ei = owner_projection(&graph, "ei").unwrap();
    let eos = owner_projection(&graph, "eos").unwrap();
    assert_eq!(
      ei["cores"].as_object().unwrap().keys().collect::<Vec<_>>(),
      vec!["ei"]
    );
    assert_eq!(
      eos["cores"].as_object().unwrap().keys().collect::<Vec<_>>(),
      vec!["eos"]
    );
    assert!(ei.pointer("/cores/ei/layers/io").is_some());
    assert!(eos.pointer("/cores/eos/layers/hardware").is_some());
    assert_eq!(ei["robot"], graph["robot"]);
    assert_eq!(
      owner_projection(&graph, "other").unwrap_err().code,
      "unsupported_owner"
    );
  }

  #[test]
  fn missing_owner_core_is_rejected() {
    let graph =
      json!({"version":"0.2.1","robot":{"id":"r","name":"R"},"cores":{"isos":{"layers":{}}}});
    assert_eq!(
      owner_projection(&graph, "ei").unwrap_err().path,
      "/cores/ei/layers"
    );
  }

  #[test]
  fn owner_projection_preserves_every_node_value_and_connection() {
    let graph = json!({"version":"0.2.1","robot":{"id":"r","name":"R"},"cores":{
      "isos":{"layers":{"ui":{"groups":{"monitor":{"nodes":{"view":{"id":"view","node_type":"isos.ui.vision.monitor@v0.2","settings":{"note":"keep this"},"inputs":{"frame":{"node":"eos/interface/egress/out","output":"frame"}}}}}}}}},
      "ei":{"layers":{"io":{"groups":{"input":{"nodes":{"first":{"id":"first","node_type":"ei.io.input.arm_status@v0.2","settings":{"forward_rate_hz":37},"inputs":{"observation":{"node":"eos/interface/egress/arm","output":"observation"}}}}}}}}},
      "eos":{"layers":{"hardware":{"groups":{"camera":{"nodes":{"front":{"id":"front","node_type":"eos.hardware.camera.device@v0.2","settings":{"width_px":1280,"height_px":720,"fps":60},"adapter":{"settings":{"frequency_hz":60,"device_id":"cam"}},"observation":{"max_rate_hz":60,"max_payload_bytes":12000000}}}}}},"interface":{"groups":{"egress":{"nodes":{"out":{"id":"out","node_type":"eos.interface.egress.vision_flow@v0.2","inputs":{"frame":{"node":"eos/hardware/camera/front","output":"frame"}}}}}}}}}
    }});
    for role in ["ei", "eos"] {
      let projected = owner_projection(&graph, role).unwrap();
      assert_eq!(
        projected["cores"][role]["layers"],
        graph["cores"][role]["layers"]
      );
    }
  }
}
