//! Bounded owner setup for the EOS-only graph projection.
use std::{
  error::Error,
  io::{Read, Write},
};
use aether_hal::{NodeTypeRegistry, Profile};
use serde_json::{json, Value};
const MAX_REQUEST_BYTES: u64 = 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
pub fn run(registry: &NodeTypeRegistry) -> Result<(), Box<dyn Error>> {
  let mut input = Vec::new();
  std::io::stdin()
    .take(MAX_REQUEST_BYTES + 1)
    .read_to_end(&mut input)?;
  let reply = if input.len() as u64 > MAX_REQUEST_BYTES {
    failure(
      "request",
      "request_too_large",
      "setup request exceeds 1 MiB",
    )
  } else {
    match serde_json::from_slice::<Value>(&input) {
      Ok(request) => handle(&request, registry),
      Err(_) => failure("request", "invalid_json", "setup request is not JSON"),
    }
  };
  let mut bytes = serde_json::to_vec(&reply)?;
  if bytes.len() > MAX_RESPONSE_BYTES {
    bytes = serde_json::to_vec(&failure(
      "response",
      "response_too_large",
      "setup reply exceeds 4 MiB",
    ))?;
  }
  std::io::stdout().write_all(&bytes)?;
  std::io::stdout().write_all(b"\n")?;
  Ok(())
}
fn failure(path: &str, code: &str, message: &str) -> Value {
  json!({"schema_version":2,"ok":false,"diagnostics":[{"path":path,"code":code,"message":message}],"files":[]})
}
pub(crate) fn handle(request: &Value, registry: &NodeTypeRegistry) -> Value {
  if request["schema_version"] != 2 || request["core"] != "eos" {
    return failure("request", "invalid_envelope", "expected EOS setup schema 2");
  }
  let op = request["op"].as_str().unwrap_or("");
  match op {
    "identity" => {
      return json!({"schema_version":2,"ok":true,"diagnostics":[],"files":[],"identity":{
      "core":"eos","product_version":env!("CARGO_PKG_VERSION"),"graph_version":"0.2.0","node_type_version":"0.2",
      "supported_graph_versions":["0.2.0"],"supported_node_type_versions":["0.2"],"supported_node_types":aether_hal::builtin_reader_node_types(),
      "features":super::PROFILE_FEATURES,"setup_schema_version":2}});
    }
    "catalog" => {
      return json!({"schema_version":2,"ok":true,"diagnostics":[],"files":[],"catalog":{
      "types":registry.all().collect::<Vec<_>>(),"data_types":registry.data_types().collect::<Vec<_>>(),
      "adapter_families":registry.catalog_adapter_families(),"primitive_specs":aether_hal::PRIMITIVE_SPECS.iter().filter(|spec|spec.name=="arm.delta").collect::<Vec<_>>()}});
    }
    "options" | "validate" | "render" => {}
    _ => return failure("op", "invalid_operation", "unknown setup operation"),
  }
  let revision = request["revision"].as_str().unwrap_or("");
  if revision.is_empty() || revision.len() > 128 || !revision.is_ascii() {
    return failure(
      "revision",
      "invalid_revision",
      "missing or invalid graph revision",
    );
  }
  if !request["endpoints"].is_object() || request["packages"] != json!([]) {
    return failure(
      "request",
      "invalid_inputs",
      "expected endpoints object and no packages",
    );
  }
  let graph = &request["graph"];
  if graph["version"] != "0.2.0"
    || graph["cores"]
      .as_object()
      .is_none_or(|cores| cores.len() != 1 || !cores.contains_key("eos"))
  {
    return failure(
      "graph/cores",
      "invalid_graph",
      "expected EOS-only graph 0.2.0 projection",
    );
  }
  let source = match serde_json::to_string(graph) {
    Ok(source) => source,
    Err(_) => return failure("graph", "invalid_graph", "graph is not JSON"),
  };
  let profile = match Profile::from_json_with_registry(&source, registry) {
    Ok(profile) => profile,
    Err(error) => return failure("graph", "invalid_graph", &error.to_string()),
  };
  if let Err(error) = profile.validate_demo_chain() {
    return failure("graph", "invalid_graph", &error.to_string());
  }
  if op == "options" {
    let arm = profile
      .all_nodes()
      .find(|node| node.node_type == "eos.hardware.mock.arm@v0.2")
      .expect("validated demo arm");
    return json!({"schema_version":2,"ok":true,"diagnostics":[],"files":[],"catalog":{"setting_options":[{
      "node_path":arm.path(),"setting":"adapter.type","selected":arm.adapter.adapter_type,
      "options":[{"value":"mock-arm","label":"Built-in mock arm","description":"Six-joint in-process demonstration adapter","available":true,"reason_code":null,"reason":null}]
    }]}});
  }
  if op == "validate" {
    return json!({"schema_version":2,"ok":true,"diagnostics":[],"files":[]});
  }
  json!({"schema_version":2,"ok":true,"diagnostics":[],"files":[{"path":"eos/graph.json","content":source}]})
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn catalog_has_nine_demo_nodes_and_three_data_types() {
    let reply = handle(
      &json!({"schema_version":2,"core":"eos","op":"catalog"}),
      &NodeTypeRegistry::builtin(),
    );
    assert_eq!(reply["catalog"]["types"].as_array().unwrap().len(), 9);
    assert_eq!(reply["catalog"]["data_types"].as_array().unwrap().len(), 3);
    let arm = reply["catalog"]["types"]
      .as_array()
      .unwrap()
      .iter()
      .find(|node| node["id"] == "eos.hardware.mock.arm@v0.2")
      .unwrap();
    assert_eq!(arm["settings"][0]["default"], 10);
    assert_eq!(arm["settings"][1]["default"], 50);
  }
  #[test]
  fn old_projection_is_rejected() {
    let reply = handle(
      &json!({"schema_version":2,"core":"eos","op":"validate","revision":"a","endpoints":{},"packages":[],
      "graph":{"version":"0.2.0","cores":{"isos":{"layers":{}}}}}),
      &NodeTypeRegistry::builtin(),
    );
    assert_eq!(reply["ok"], false);
    assert_eq!(reply["diagnostics"][0]["code"], "invalid_graph");
  }
}
