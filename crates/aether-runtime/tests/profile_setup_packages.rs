use std::{
  fs,
  io::Write,
  process::{Command, Stdio},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn setup(binary: &str, request: &Value) -> Value {
  let mut child = Command::new(binary)
    .arg("setup")
    .arg("--json")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();
  child
    .stdin
    .take()
    .unwrap()
    .write_all(&serde_json::to_vec(request).unwrap())
    .unwrap();
  let output = child.wait_with_output().unwrap();
  assert!(output.status.success());
  serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn one_shot_setup_loads_selected_package_sidecar_before_graph_admission() {
  let dir = tempfile::tempdir().unwrap();
  let store = dir.path().join("store");
  let payload = dir.path().join("payload");
  fs::create_dir(&store).unwrap();
  fs::create_dir(&payload).unwrap();
  fs::create_dir(payload.join("lib")).unwrap();
  let library = payload.join("lib/camera.so");
  fs::write(&library, b"catalog fixture").unwrap();
  let library_digest = format!("{:x}", Sha256::digest(fs::read(&library).unwrap()));
  fs::write(
    store.join("receipt.json"),
    serde_json::to_vec(&json!({"local_link":payload})).unwrap(),
  )
  .unwrap();
  fs::write(
    store.join("node-types.json"),
    serde_json::to_vec(&json!({
      "package":{"id":"vendor/camera-driver","version":"0.1.2"},
      "libraries":[{"path":"lib/camera.so","sha256":library_digest}],
      "node_types":[{"id":"eos.hardware.camera.device@v0.2","core":"eos",
        "layer":"hardware","group":"camera","kind":"camera","lifecycle":"adapter",
        "description":"Vendor camera","capabilities":["rgb8"],"settings":[],"inputs":[],
        "outputs":[{"name":"frame","data_type":"image_frame@v0.2","required":false,
          "description":"Camera frame","transport":"in_process","delivery_mode":"event",
          "data_class":"observation","contract":{"owner":"eos","id":"image-local","version":"1"}}],
        "implementation":{"kind":"native_plugin","abi":"isos_node_v2","library":"lib/camera.so"}}],
      "adapter_families":[{"id":"vendor-socket","device_kinds":["camera","base","manipulator"],
        "transport":"unix_socket","description":"Vendor camera adapter","supported_primitives":[]}]
    }))
    .unwrap(),
  )
  .unwrap();
  let mut request = json!({
    "schema_version":3,"core":"eos","op":"validate","revision":"a",
    "endpoints":{},"packages":[{"id":"vendor/camera-driver","version":"0.1.2","path":store,"payload_root":payload}],
    "graph":{"version":"0.2.1","robot":{"id":"robot","name":"Robot"},
      "cores":{"eos":{"layers":{"hardware":{"groups":{"camera":{"nodes":{
        "front":{"id":"front","node_type":"eos.hardware.camera.device@v0.2",
          "package":{"id":"vendor/camera-driver","version":"0.1.2"},
          "label":"Front camera","description":"Front image source","frame_id":"front_optical",
          "capabilities":["rgb8"],"adapter":{"type":"vendor-socket",
            "package":{"id":"vendor/camera-driver","version":"0.1.2"},
            "settings":{"service_instance":"front"}},
          "health_timing":{"poll_interval_ms":100,"stale_after_ms":500},
          "observation":{"max_rate_hz":30,"max_payload_bytes":12000000},"movement":null}
      }}}}}}}}
  });
  let binary = env!("CARGO_BIN_EXE_aether-profile");
  let reply = setup(binary, &request);
  assert_eq!(reply["diagnostics"][0]["code"], "unsupported_execution");
  assert!(
    reply["diagnostics"][0]["message"]
      .as_str()
      .unwrap()
      .contains("adapter library unavailable for vendor-socket"),
    "{reply}"
  );

  request["packages"][0]["id"] = json!("wrong/package");
  let reply = setup(binary, &request);
  assert_eq!(reply["diagnostics"][0]["code"], "package_identity_mismatch");
}
