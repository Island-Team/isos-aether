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
  if graph["version"] != "0.2.0" {
    return Err(diagnostic(
      "/version",
      "unsupported_graph_version",
      "owner projection requires graph 0.2.0",
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
  Ok(json!({"version":"0.2.0","robot":graph["robot"],"cores":{(role):{"layers":layers}}}))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn owner_projection_contains_only_its_own_core() {
    let graph = json!({"version":"0.2.0","robot":{"id":"r","name":"R"},"cores":{
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
      json!({"version":"0.2.0","robot":{"id":"r","name":"R"},"cores":{"isos":{"layers":{}}}});
    assert_eq!(
      owner_projection(&graph, "ei").unwrap_err().path,
      "/cores/ei/layers"
    );
  }
}
