#[path = "../src/domain/graph/edge_admission.rs"]
mod edge_admission;

use edge_admission::{admit, admit_partial};
use serde_json::json;

#[test]
fn current_graph_admits_matching_typed_edge() {
  let graph = json!({"version":"0.2.0","cores":{"isos":{"layers":{"eos_process":{"groups":{"source":{"nodes":{"a":{"id":"a","node_type":"isos.eos_process.source@v0.2"}}},"sink":{"nodes":{"b":{"id":"b","node_type":"isos.eos_process.sink@v0.2","inputs":{"data":{"node":"isos/eos_process/source/a","output":"data"}}}}}}}}}}});
  let mut catalog = json!({"types":[
    {"id":"isos.eos_process.source@v0.2","core":"isos","layer":"eos_process","group":"source","inputs":[],"outputs":[{"name":"data","data_type":"topology_link","transport":"none","delivery_mode":"declaration"}]},
    {"id":"isos.eos_process.sink@v0.2","core":"isos","layer":"eos_process","group":"sink","inputs":[{"name":"data","data_type":"topology_link","transport":"none","delivery_mode":"declaration"}],"outputs":[]}
  ],"data_types":[{"id":"topology_link","version":"1.0","purpose":"Static topology","media_type":"application/json","codec":"json_v1","schema":{"type":"object"},"max_payload_bytes":1}]});
  let edges = admit(&graph, &catalog).unwrap();
  assert_eq!(edges.len(), 1);
  assert_eq!(edges[0].source, "isos/eos_process/source/a");
  assert_eq!(edges[0].route_class, "declaration");
  catalog["types"][1]["inputs"][0]["data_type"] = json!("other");
  assert_eq!(
    admit(&graph, &catalog).unwrap_err().code,
    "edge_type_mismatch"
  );
}

#[test]
fn unsupported_graph_version_is_rejected() {
  let graph = json!({"version":"0.1.0","cores":{}});
  let catalog = json!({"types":[],"data_types":[]});
  assert_eq!(
    admit(&graph, &catalog).unwrap_err().code,
    "unsupported_graph_version"
  );
}

fn bridge_route() -> (serde_json::Value, serde_json::Value) {
  let graph = json!({"version":"0.2.0","cores":{"isos":{"layers":{"ui":{"groups":{"source":{"nodes":{"front":{"id":"front","node_type":"isos.ui.source@v0.2"}}}}}}},"ei":{"layers":{"io":{"groups":{"sink":{"nodes":{"preview":{"id":"preview","node_type":"ei.io.sink@v0.2","inputs":{"frame":{"node":"isos/ui/source/front","output":"frame"}}}}}}}}}}});
  let protocol = json!({"owner":"isos","id":"frame-http","version":"1.0"});
  let port = json!({"name":"frame","data_type":"camera_frame","transport":"http","delivery_mode":"stream","data_class":"image","contract":protocol});
  let catalog = json!({"types":[
    {"id":"isos.ui.source@v0.2","core":"isos","layer":"ui","group":"source","capabilities":["bridge"],"bridge":{"adapter":"http","inputs":[],"outputs":["frame"]},"inputs":[],"outputs":[port]},
    {"id":"ei.io.sink@v0.2","core":"ei","layer":"io","group":"sink","capabilities":["bridge"],"bridge":{"adapter":"http","inputs":["frame"],"outputs":[]},"inputs":[port],"outputs":[]}
  ],"bridge_adapters":[{"core":"isos","adapter":"http","node_type":"isos.ui.source@v0.2"},{"core":"ei","adapter":"http","node_type":"ei.io.sink@v0.2"}],"port_routes":[{"source_type":"isos.ui.source@v0.2","output":"frame","target_type":"ei.io.sink@v0.2","input":"frame","transport":"http","delivery_mode":"stream","data_type":"camera_frame","contract":protocol}],"data_types":[{"id":"camera_frame","version":"1.0","purpose":"Camera frame","media_type":"application/json","codec":"json_v1","schema":{"type":"object","properties":{"width":{"type":"integer","x-unit":"px"}}},"max_payload_bytes":1024}]});
  (graph, catalog)
}

#[test]
fn live_route_requires_exact_payload_channel_and_protocol() {
  let (graph, catalog) = bridge_route();
  assert_eq!(
    admit(&graph, &catalog).unwrap()[0].route_class,
    "owner_bridge"
  );
  let mut changed = catalog.clone();
  changed["types"][1]["inputs"][0]["contract"]["version"] = json!("2.0");
  assert_eq!(admit(&graph, &changed).unwrap_err().code, "edge_protocol");
  changed = catalog.clone();
  changed["types"][1]["inputs"][0]["transport"] = json!("unix_socket");
  assert_eq!(admit(&graph, &changed).unwrap_err().code, "edge_transport");
  changed = catalog.clone();
  changed["types"][1]["inputs"][0]["data_type"] = json!("other");
  assert_eq!(
    admit(&graph, &changed).unwrap_err().code,
    "edge_type_mismatch"
  );
  changed = catalog.clone();
  changed["types"][1]["inputs"][0]["delivery_mode"] = json!("event");
  assert_eq!(
    admit(&graph, &changed).unwrap_err().code,
    "edge_delivery_mode"
  );
}

#[test]
fn catalog_rejects_duplicate_and_incomplete_payloads() {
  let (graph, mut catalog) = bridge_route();
  let repeated = catalog["data_types"][0].clone();
  catalog["data_types"].as_array_mut().unwrap().push(repeated);
  assert_eq!(
    admit(&graph, &catalog).unwrap_err().code,
    "data_type_duplicate"
  );
  catalog["data_types"].as_array_mut().unwrap().pop();
  catalog["data_types"][0]
    .as_object_mut()
    .unwrap()
    .remove("codec");
  assert_eq!(
    admit(&graph, &catalog).unwrap_err().code,
    "data_type_invalid"
  );
}

#[test]
fn matching_payload_without_owner_route_is_rejected() {
  let (graph, mut catalog) = bridge_route();
  catalog["bridge_adapters"] = json!([]);
  assert_eq!(
    admit(&graph, &catalog).unwrap_err().code,
    "edge_bridge_undeclared"
  );
  let (_, mut catalog) = bridge_route();
  catalog["port_routes"] = json!([]);
  assert_eq!(
    admit(&graph, &catalog).unwrap_err().code,
    "edge_bridge_undeclared"
  );
}

#[test]
fn connection_preview_allows_other_required_inputs_to_be_unconnected() {
  let (graph, mut catalog) = bridge_route();
  let other = json!({"name":"other","data_type":"camera_frame","transport":"http","delivery_mode":"stream","data_class":"image","contract":{"owner":"isos","id":"frame-http","version":"1.0"},"required":true});
  catalog["types"][1]["inputs"]
    .as_array_mut()
    .unwrap()
    .push(other);
  assert_eq!(
    admit(&graph, &catalog).unwrap_err().code,
    "edge_required_input"
  );
  assert_eq!(admit_partial(&graph, &catalog).unwrap().len(), 1);
}
