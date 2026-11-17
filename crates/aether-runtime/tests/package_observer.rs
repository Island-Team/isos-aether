//! Mock package replay through the real EOS owner, worker, and native ABI.
use std::{
  fs,
  io::{Read, Write},
  os::unix::{
    fs::PermissionsExt,
    net::{UnixListener, UnixStream},
  },
  path::Path,
  process::Command,
  thread,
  time::Duration,
};
use aether_hal::NodeTypeRegistry;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn digest(path: &Path) -> String {
  format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn wait_socket(path: &Path) {
  for _ in 0..100 {
    if path.exists() {
      return;
    }
    thread::sleep(Duration::from_millis(20));
  }
  panic!("socket absent: {}", path.display());
}
fn exchange(socket: &Path, op: &str, body: Value) -> Value {
  let bytes = serde_json::to_vec(&body).unwrap();
  let mut stream = UnixStream::connect(socket).unwrap();
  write!(
    stream,
    "POST /v1/{op} HTTP/1.1\r\nHost: owner\r\nContent-Length: {}\r\n\r\n",
    bytes.len()
  )
  .unwrap();
  stream.write_all(&bytes).unwrap();
  let mut reply = Vec::new();
  stream.read_to_end(&mut reply).unwrap();
  let split = reply
    .windows(4)
    .position(|part| part == b"\r\n\r\n")
    .unwrap()
    + 4;
  serde_json::from_slice(&reply[split..]).unwrap()
}

#[test]
fn verified_observer_runs_samples_rejects_stale_and_stops_child() {
  let dir = tempfile::tempdir().unwrap();
  let root = dir.path();
  let store = root.join("store");
  let lib = root.join("lib");
  fs::create_dir_all(&store).unwrap();
  fs::create_dir_all(&lib).unwrap();
  let script = root.join("mock-process");
  fs::write(&script, b"#!/usr/bin/python3\nimport socket,sys,time\ns=socket.socket(socket.AF_UNIX)\ns.bind(sys.argv[2])\ns.listen(1)\nwhile True: time.sleep(1)\n").unwrap();
  fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
  let scene = root.join("scene.xml");
  fs::write(&scene, b"<scene/>").unwrap();
  let source = root.join("observer.c");
  let library = lib.join("observer.so");
  let source_code = r#"
#include "isos_node_v1.h"
#include <stdlib.h>
struct isos_node_handle { const isos_node_host_v1 *host; };
static int32_t create(const uint8_t *config,size_t len,const isos_node_host_v1 *host,isos_node_handle **out) { (void)config;(void)len;*out=malloc(sizeof(**out));(*out)->host=host;return 0; }
static int32_t start(isos_node_handle *n) { return n?0:-1; }
static int32_t input(isos_node_handle *n,const uint8_t *bytes,size_t len) { (void)bytes;(void)len;const uint8_t out[]="{\"port\":\"frame\",\"payload\":{\"sample\":{\"source_time_ns\":1,\"clock\":\"simulation\",\"sequence\":1,\"frame_id\":\"camera\"},\"width_px\":1,\"height_px\":1,\"encoding\":\"rgb8\",\"data\":\"AA==\"}}";return n->host->emit(n->host->context,out,sizeof(out)-1); }
static int32_t health(isos_node_handle *n) { return n?1:-1; }
static int32_t stop(isos_node_handle *n) { return n?0:-1; }
static void destroy(isos_node_handle *n) { free(n); }
static const isos_node_api_v1 api={1,sizeof(isos_node_api_v1),create,start,input,health,stop,destroy};
const isos_node_api_v1 *isos_node_v1(void) { return &api; }
"#;
  let source_code = source_code
    .replace(r#"\"AA==\""#, &format!(r#"\"{}\""#, "A".repeat(131072)))
    .replace(r#"\"width_px\":1"#, r#"\"width_px\":256"#)
    .replace(r#"\"height_px\":1"#, r#"\"height_px\":128"#);
  fs::write(&source, source_code).unwrap();
  let header = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/node");
  let output = Command::new("cc")
    .args(["-shared", "-fPIC", "-I"])
    .arg(header)
    .arg(&source)
    .arg("-o")
    .arg(&library)
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
  let package = json!({"id":"vendor/mock","version":"0.2.0"});
  let mut process_type = serde_json::to_value(
    NodeTypeRegistry::builtin()
      .get("eos.skill.demo.joint_delta@v0.2")
      .unwrap(),
  )
  .unwrap();
  process_type["id"] = json!("eos.process.mock.service@v0.2");
  process_type["layer"] = json!("process");
  process_type["group"] = json!("mock");
  process_type["lifecycle"] = json!("process");
  process_type["capabilities"] = json!(["process"]);
  process_type["inputs"] = json!([]);
  process_type["outputs"] = json!([]);
  process_type["settings"] = json!([]);
  process_type["implementation"] = json!({"kind":"managed_process","interface":"mock-v1","version":"1","lifecycle_adapter":"mock-v1"});
  let camera_type = json!({
    "id":"eos.hardware.camera.device@v0.2","core":"eos","layer":"hardware",
    "group":"camera","kind":"camera","lifecycle":"adapter",
    "description":"Package camera observer","capabilities":["rgb8"],
    "settings":[],"inputs":[],
    "outputs":[{"name":"frame","data_type":"image_frame@v0.2","required":false,
      "description":"Observed camera frame","transport":"in_process","delivery_mode":"event",
      "data_class":"observation","contract":{"owner":"eos","id":"image-local","version":"1"}}],
    "implementation":{"kind":"native_plugin","abi":"isos_node_v1","library":"lib/observer.so"}
  });
  let sidecar = store.join("node-types.json");
  fs::write(&sidecar, json!({"package":package,"libraries":[{"path":"lib/observer.so","sha256":digest(&library)}],
    "adapters":[{"family":"mock-v1","library":"lib/observer.so","sha256":digest(&library)}],
    "adapter_families":[{"id":"mock-v1","device_kinds":["camera"],"transport":"unix_socket","description":"Mock observer"}],
    "node_types":[process_type,camera_type]}).to_string()).unwrap();
  let second_store = root.join("second-store");
  fs::create_dir_all(second_store.join("lib")).unwrap();
  let second_library = second_store.join("lib/pass.so");
  fs::copy(&library, &second_library).unwrap();
  let second_package = json!({"id":"vendor/vision","version":"0.2.0"});
  let frame_port = json!({"name":"frame","data_type":"image_frame@v0.2","required":false,
    "description":"Observed frame","transport":"in_process","delivery_mode":"event",
    "data_class":"observation","contract":{"owner":"eos","id":"image-local","version":"1"}});
  let route = json!({"source_type":"eos.hardware.camera.device@v0.2","output":"frame",
    "target_type":"eos.skill.package.frame_pass@v0.2","input":"frame",
    "transport":"in_process","delivery_mode":"event","data_type":"image_frame@v0.2",
    "contract":{"owner":"eos","id":"image-local","version":"1"}});
  let second_sidecar = second_store.join("node-types.json");
  fs::write(
    &second_sidecar,
    json!({"package":second_package,
    "libraries":[{"path":"lib/pass.so","sha256":digest(&second_library)}],
    "node_types":[{"id":"eos.skill.package.frame_pass@v0.2","core":"eos","layer":"skill",
      "group":"package","kind":"software","lifecycle":"internal","description":"Pass package frame",
      "capabilities":[],"settings":[],"inputs":[frame_port],
      "outputs":[{"name":"frame","data_type":"image_frame@v0.2","required":false,
        "description":"Passed frame","transport":"in_process","delivery_mode":"event",
        "data_class":"observation","contract":{"owner":"eos","id":"image-local","version":"1"}}],
      "implementation":{"kind":"native_plugin","abi":"isos_node_v1","library":"lib/pass.so"}}],
    "port_routes":[route]})
    .to_string(),
  )
  .unwrap();
  fs::write(
    second_store.join("receipt.json"),
    json!({"local_link":second_store,"payload_sha256":"b".repeat(64),
      "sidecar_sha256":digest(&second_sidecar)})
    .to_string(),
  )
  .unwrap();
  fs::write(
    store.join("receipt.json"),
    json!({"local_link":root,"payload_sha256":"b".repeat(64),"sidecar_sha256":digest(&sidecar)})
      .to_string(),
  )
  .unwrap();
  let release = store.join("release.json");
  fs::write(
    &release,
    json!({"contract":"isos.package@v1","schema_version":"0.1.0","id":"vendor/mock","kind":"backend","version":"0.2.0","source_commit":"a".repeat(40),
      "assets":[{"name":"mock-package.tar.gz","url":"https://github.com/vendor/mock/releases/download/v0.2.0/mock-package.tar.gz","sha256":"d".repeat(64),"size_bytes":1,"os":"linux","arch":"x86_64"}]}).to_string(),
  )
  .unwrap();
  let handoff = root.join("handoff.json");
  fs::write(&handoff, json!({"schema_version":1,"node_path":"eos/process/mock/sim","node_type":"eos.process.mock.service@v0.2",
    "package":package,"entrypoint":script,"entrypoint_sha256":digest(&script),"args":["--socket","${socket}","--scene","${resource.scene.path}"],
    "resources":{"scene":{"kind":"scene","id":"one","path":scene,"sha256":digest(&scene),"metadata":null}},
    "lifecycle_adapter":"mock-v1","release_sha256":digest(&release),"payload_sha256":"b".repeat(64)}).to_string()).unwrap();
  let owner_socket = root.join("owner.sock");
  let mut owner = Command::new(env!("CARGO_BIN_EXE_aether-profile"))
    .env("EOS_LOG_DIR", root)
    .args(["owner", "--socket"])
    .arg(&owner_socket)
    .arg("--node-types")
    .arg(&sidecar)
    .arg("--node-types")
    .arg(&second_sidecar)
    .arg("--adapter-library")
    .arg(format!(
      "mock-v1={}@{}",
      library.display(),
      digest(&library)
    ))
    .arg("--package-process")
    .arg(&handoff)
    .spawn()
    .unwrap();
  wait_socket(&owner_socket);
  let revision = "a".repeat(64);
  let process_node = json!({"id":"sim","node_type":"eos.process.mock.service@v0.2","package":package,"label":"Mock service","description":"Mock child","capabilities":["process"],"observation":null,"movement":null});
  let camera_node = json!({"id":"front","node_type":"eos.hardware.camera.device@v0.2","package":package,"label":"Camera","description":"Mock camera","frame_id":"camera","capabilities":["rgb8"],"adapter":{"type":"mock-v1","package":package,"settings":{"socket_path":"${socket}","device_id":"cam-0"}},"health_timing":{"poll_interval_ms":100,"stale_after_ms":500},"observation":{"max_rate_hz":30,"max_payload_bytes":12000000},"movement":null});
  let transfer_node = json!({"id":"front","node_type":"eos.interface.egress.vision_flow@v0.2","label":"Front vision","description":"Camera transfer",
    "inputs":{"frame":{"node":"eos/hardware/camera/front","output":"frame"}}});
  let mut graph = json!({"version":"0.2.1","robot":{"id":"robot","name":"Robot"},"cores":{"eos":{"layers":{
    "process":{"groups":{"mock":{"nodes":{"sim":process_node}}}},
    "hardware":{"groups":{"camera":{"nodes":{"front":camera_node}}}},
    "interface":{"groups":{"egress":{"nodes":{"front":transfer_node}}}}
  }}}});
  let packages = json!([{"id":"vendor/mock","version":"0.2.0","path":store,"payload_root":root}]);
  let registry = NodeTypeRegistry::with_contribution_files(std::slice::from_ref(&sidecar)).unwrap();
  aether_hal::Profile::from_json_with_registry(&graph.to_string(), &registry).unwrap();
  let missing_frequency = exchange(
    &owner_socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run","run_id":"run_1","revision":revision,"graph":graph,"packages":packages,
    "launch":{"sha256":digest(Path::new(env!("CARGO_BIN_EXE_aether-runtime"))),"args":[],"env":{}}}),
  );
  assert_eq!(
    missing_frequency["error"], "source_frequency_hz_required",
    "{missing_frequency}"
  );
  assert!(missing_frequency["diagnostic"]
    .as_str()
    .unwrap()
    .contains("eos/hardware/camera/front"));
  graph["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"]["nodes"]["front"]["adapter"]
    ["settings"]["frequency_hz"] = json!(20.0);
  let native_graph = graph.clone();
  let run = exchange(
    &owner_socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run","run_id":"run_1","revision":revision,"graph":graph,"packages":packages,
    "launch":{"sha256":digest(Path::new(env!("CARGO_BIN_EXE_aether-runtime"))),"args":[],"env":{}}}),
  );
  assert_eq!(run["ok"], true, "{run}");
  let status = exchange(
    &owner_socket,
    "status",
    json!({"schema_version":1,"core":"eos","op":"status","run_id":"run_1","revision":revision}),
  );
  assert_eq!(status["state"], "ready", "{status}");
  let stale = exchange(
    &owner_socket,
    "observe",
    json!({"schema_version":1,"core":"eos","op":"observe","run_id":"run_1","revision":"c".repeat(64),"node":"eos/hardware/camera/front"}),
  );
  assert_eq!(stale["ok"], false);
  let sample = exchange(
    &owner_socket,
    "observe",
    json!({"schema_version":1,"core":"eos","op":"observe","run_id":"run_1","revision":revision,"node":"eos/hardware/camera/front"}),
  );
  assert_eq!(sample["ok"], true, "{sample}");
  assert_eq!(sample["outputs"][0]["output"]["port"], "frame");
  assert_eq!(
    sample["outputs"][0]["output"]["payload"]["data"]
      .as_str()
      .unwrap()
      .len(),
    131072
  );
  let transferred = exchange(
    &owner_socket,
    "vision_frame",
    json!({"schema_version":1,"core":"eos","op":"vision_frame","run_id":"run_1",
      "revision":revision,"transfer_path":"eos/interface/egress/front",
      "source_path":"eos/hardware/camera/front"}),
  );
  assert_eq!(transferred["ok"], true, "{transferred}");
  assert_eq!(transferred["frame"]["sample"]["frame_id"], "camera");
  let pooled = exchange(
    &owner_socket,
    "latest_output",
    json!({"schema_version":1,"core":"eos","op":"latest_output","run_id":"run_1",
      "revision":revision,"transfer_path":"eos/interface/egress/front",
      "source_path":"eos/hardware/camera/front","node":"eos/interface/egress/front",
      "port":"frame","after_sequence":0,"wait_ms":0}),
  );
  assert_eq!(pooled["ok"], true, "{pooled}");
  assert_eq!(pooled["sample"]["node"], "eos/interface/egress/front");
  assert_eq!(
    pooled["sample"]["output"]["payload"]["sample"]["frame_id"],
    "camera"
  );
  assert!(pooled["sample"]["sequence"].as_u64().is_some());
  let wrong_source = exchange(
    &owner_socket,
    "vision_frame",
    json!({"schema_version":1,"core":"eos","op":"vision_frame","run_id":"run_1",
      "revision":revision,"transfer_path":"eos/interface/egress/front",
      "source_path":"eos/hardware/camera/other"}),
  );
  assert_eq!(wrong_source["error"], "vision_route_unavailable");
  let stopped = exchange(
    &owner_socket,
    "stop",
    json!({"schema_version":1,"core":"eos","op":"stop","run_id":"run_1","revision":revision}),
  );
  assert_eq!(stopped["state"], "stopped", "{stopped}");
  assert!(!root.join("eos-package.sock").exists());

  let demo_source = fs::read_to_string(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/graph/demo-only.nodes.json"
  ))
  .unwrap();
  let demo: Value = serde_json::from_str(&demo_source).unwrap();
  let mut mixed = json!({"version":"0.2.1","robot":{"id":"robot","name":"Robot"},
    "cores":{"eos":demo["cores"]["eos"]}});
  mixed["cores"]["eos"]["layers"]["process"] =
    native_graph["cores"]["eos"]["layers"]["process"].clone();
  mixed["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"] =
    native_graph["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"].clone();
  mixed["cores"]["eos"]["layers"]["interface"]["groups"]["egress"]["nodes"]["front"] = native_graph
    ["cores"]["eos"]["layers"]["interface"]["groups"]["egress"]["nodes"]["front"]
    .clone();
  mixed["cores"]["eos"]["layers"]["skill"]["groups"]["package"] = json!({"nodes":{
    "frame_pass":{"id":"frame_pass","node_type":"eos.skill.package.frame_pass@v0.2",
      "package":second_package,"label":"Frame pass","description":"Second package node",
      "inputs":{"frame":{"node":"eos/hardware/camera/front","output":"frame"}}}}});
  let mixed_revision = "c".repeat(64);
  let packages_mixed = json!([{"id":"vendor/mock","version":"0.2.0","path":store,"payload_root":root},
    {"id":"vendor/vision","version":"0.2.0","path":second_store,"payload_root":second_store}]);
  let mixed_registry =
    NodeTypeRegistry::with_contribution_files(&[sidecar.clone(), second_sidecar.clone()]).unwrap();
  aether_hal::Profile::from_json_with_registry(&mixed.to_string(), &mixed_registry).unwrap();
  let mixed_run = exchange(
    &owner_socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run",
    "run_id":"run_2","revision":mixed_revision,"graph":mixed,"packages":packages_mixed,
    "launch":{"sha256":digest(Path::new(env!("CARGO_BIN_EXE_aether-runtime"))),"args":[],"env":{}}}),
  );
  assert_eq!(mixed_run["ok"], true, "{mixed_run}");
  assert!(mixed_run["mock_worker_pid"].is_number(), "{mixed_run}");
  let mixed_status = exchange(
    &owner_socket,
    "status",
    json!({"schema_version":1,"core":"eos",
    "op":"status","run_id":"run_2","revision":mixed_revision}),
  );
  assert_eq!(mixed_status["state"], "ready", "{mixed_status}");
  assert_eq!(mixed_status["mock_hardware"], "ready");
  let monitor = exchange(
    &owner_socket,
    "monitor",
    json!({"schema_version":1,"core":"eos",
    "op":"monitor","run_id":"run_2","revision":mixed_revision}),
  );
  assert_eq!(monitor["ok"], true, "{monitor}");
  let camera = exchange(
    &owner_socket,
    "observe",
    json!({"schema_version":1,"core":"eos",
    "op":"observe","run_id":"run_2","revision":mixed_revision,
    "node":"eos/hardware/camera/front"}),
  );
  assert_eq!(camera["ok"], true, "{camera}");
  assert!(
    camera["outputs"]
      .as_array()
      .unwrap()
      .iter()
      .any(|output| output["node"] == "eos/skill/package/frame_pass"
        && output["output"]["port"] == "frame"),
    "{camera}"
  );
  let activation = exchange(
    &owner_socket,
    "activate",
    json!({"schema_version":1,"core":"eos",
    "op":"activate","run_id":"run_2","revision":mixed_revision,
    "source_node":"isos/ui/button/trigger","event_id":"mixed-click-1","kind":"activate"}),
  );
  assert_eq!(activation["status"], "accepted", "{activation}");
  let mixed_stop = exchange(
    &owner_socket,
    "stop",
    json!({"schema_version":1,"core":"eos",
    "op":"stop","run_id":"run_2","revision":mixed_revision}),
  );
  assert_eq!(mixed_stop["state"], "stopped", "{mixed_stop}");
  assert!(!root.join("eos-package.sock").exists());
  assert!(!root.join("eos-mock-run.sock").exists());
  owner.kill().unwrap();
  owner.wait().unwrap();
}

#[test]
fn v2_service_observation_reaches_separately_pinned_v1_software() {
  let dir = tempfile::tempdir().unwrap();
  let root = dir.path();
  let header = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/node");
  let source_store = root.join("source-package");
  let sink_store = root.join("sink-package");
  fs::create_dir_all(source_store.join("lib")).unwrap();
  fs::create_dir_all(sink_store.join("lib")).unwrap();
  let v2_source = root.join("source.c");
  let v2_library = source_store.join("lib/source.so");
  let service_socket = root.join("service.sock");
  let revision = "d".repeat(64);
  fs::write(&v2_source, format!(r#"
#include "isos_node.h"
#include <stdlib.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <string.h>
#include <unistd.h>
struct isos_node_v2_handle {{ const isos_node_host_v2 *host; char socket_path[108]; }};
static int32_t create(const uint8_t *c,size_t l,const isos_node_host_v2 *h,isos_node_v2_handle **o){{char *copy=malloc(l+1);if(!copy)return -1;memcpy(copy,c,l);copy[l]=0;char *path=strstr(copy,"\"socket_path\":\"");if(!path){{free(copy);return -1;}}path+=strlen("\"socket_path\":\"");char *end=strchr(path,'"');if(!end||end-path>=108){{free(copy);return -1;}}*o=calloc(1,sizeof(**o));if(!*o){{free(copy);return -1;}}(*o)->host=h;memcpy((*o)->socket_path,path,(size_t)(end-path));free(copy);return 0;}}
static int32_t start(isos_node_v2_handle *n){{return n?0:-1;}}
static int32_t observe(isos_node_v2_handle *n,const uint8_t *b,size_t l){{(void)b;(void)l;int fd=socket(AF_UNIX,SOCK_STREAM,0);if(fd<0)return -1;struct sockaddr_un addr={{0}};addr.sun_family=AF_UNIX;strncpy(addr.sun_path,n->socket_path,sizeof(addr.sun_path)-1);if(connect(fd,(struct sockaddr*)&addr,sizeof(addr))!=0){{close(fd);return -1;}}write(fd,"observe\n",8);char out[2048];int got=read(fd,out,sizeof(out));close(fd);if(got<=0)return -1;return n->host->emit(n->host->context,(const uint8_t*)out,(size_t)got);}}
static int32_t command(isos_node_v2_handle *n,const uint8_t *b,size_t l){{(void)n;(void)b;(void)l;return -1;}}
static int32_t health(isos_node_v2_handle *n){{return n?1:-1;}}
static int32_t stop(isos_node_v2_handle *n,const uint8_t *b,size_t l){{(void)b;(void)l;const uint8_t p[]="{{\"run_id\":\"run_3\",\"revision\":\"{revision}\",\"outcome\":\"stopped\"}}";return n->host->stop_proof(n->host->context,p,sizeof(p)-1);}}
static void destroy(isos_node_v2_handle *n){{free(n);}}
static const isos_node_api_v2 api={{2,sizeof(isos_node_api_v2),create,start,observe,command,health,stop,destroy}};
const isos_node_api_v2 *isos_node_v2(void){{return &api;}}
"#)).unwrap();
  let build = Command::new("cc")
    .args(["-shared", "-fPIC", "-I"])
    .arg(&header)
    .arg(&v2_source)
    .arg("-o")
    .arg(&v2_library)
    .output()
    .unwrap();
  assert!(
    build.status.success(),
    "{}",
    String::from_utf8_lossy(&build.stderr)
  );
  let v1_source = root.join("sink.c");
  let v1_library = sink_store.join("lib/sink.so");
  fs::write(&v1_source, r#"
#include "isos_node_v1.h"
#include <stdlib.h>
#include <string.h>
struct isos_node_handle { const isos_node_host_v1 *host; };
static int32_t create(const uint8_t *c,size_t l,const isos_node_host_v1 *h,isos_node_handle **o){(void)c;(void)l;*o=malloc(sizeof(**o));(*o)->host=h;return 0;}
static int32_t start(isos_node_handle *n){return n?0:-1;}
static int32_t input(isos_node_handle *n,const uint8_t *b,size_t l){char *copy=malloc(l+1);if(!copy)return -1;memcpy(copy,b,l);copy[l]=0;int valid=strstr(copy,"\"sequence\":73")!=NULL;free(copy);if(!valid)return -1;const uint8_t o[]="{\"port\":\"frame\",\"payload\":{\"sample\":{\"source_time_ns\":1,\"clock\":\"simulation\",\"sequence\":74,\"frame_id\":\"camera\"},\"width_px\":1,\"height_px\":1,\"encoding\":\"rgb8\",\"data\":\"AAAA\"}}";return n->host->emit(n->host->context,o,sizeof(o)-1);}
static int32_t health(isos_node_handle *n){return n?1:-1;}
static int32_t stop(isos_node_handle *n){return n?0:-1;}
static void destroy(isos_node_handle *n){free(n);}
static const isos_node_api_v1 api={1,sizeof(isos_node_api_v1),create,start,input,health,stop,destroy};
const isos_node_api_v1 *isos_node_v1(void){return &api;}
"#).unwrap();
  let build = Command::new("cc")
    .args(["-shared", "-fPIC", "-I"])
    .arg(&header)
    .arg(&v1_source)
    .arg("-o")
    .arg(&v1_library)
    .output()
    .unwrap();
  assert!(
    build.status.success(),
    "{}",
    String::from_utf8_lossy(&build.stderr)
  );
  let source_pin = json!({"id":"vendor/service-camera","version":"0.2.1"});
  let sink_pin = json!({"id":"vendor/software-sink","version":"0.2.1"});
  let frame_port = json!({"name":"frame","data_type":"image_frame@v0.2","required":false,
    "description":"Frame","transport":"in_process","delivery_mode":"event",
    "data_class":"observation","contract":{"owner":"eos","id":"image-local","version":"1"}});
  let source_type = "eos.hardware.camera.service_device@v0.2";
  let sink_type = "eos.skill.package.frame_sink@v0.2";
  let unsupported_sink_type = "eos.skill.package.frame_sink_v2@v0.2";
  let source_sidecar = source_store.join("node-types.json");
  fs::write(&source_sidecar, json!({"package":source_pin,
    "libraries":[{"path":"lib/source.so","sha256":digest(&v2_library)}],
    "adapters":[{"family":"service-v2","library":"lib/source.so","sha256":digest(&v2_library)}],
    "adapter_families":[{"id":"service-v2","device_kinds":["camera"],"transport":"unix_socket","description":"Service camera"}],
    "node_types":[{"id":source_type,"core":"eos","layer":"hardware","group":"camera",
      "kind":"camera","lifecycle":"adapter","description":"Service camera","capabilities":["rgb8"],
      "settings":[],"inputs":[],"outputs":[frame_port],
      "implementation":{"kind":"native_plugin","abi":"isos_node_v2","library":"lib/source.so"}}]})
    .to_string()).unwrap();
  let sink_sidecar = sink_store.join("node-types.json");
  let unsupported_sink_library = sink_store.join("lib/sink-v2.so");
  fs::copy(&v2_library, &unsupported_sink_library).unwrap();
  fs::write(&sink_sidecar, json!({"package":sink_pin,
    "libraries":[{"path":"lib/sink.so","sha256":digest(&v1_library)},
      {"path":"lib/sink-v2.so","sha256":digest(&unsupported_sink_library)}],
    "node_types":[{"id":sink_type,"core":"eos","layer":"skill","group":"package",
      "kind":"software","lifecycle":"internal","description":"Software sink","capabilities":[],
      "settings":[],"inputs":[frame_port],"outputs":[frame_port],
      "implementation":{"kind":"native_plugin","abi":"isos_node_v1","library":"lib/sink.so"}},
      {"id":unsupported_sink_type,"core":"eos","layer":"skill","group":"package",
      "kind":"software","lifecycle":"internal","description":"Unsupported v2 input","capabilities":[],
      "settings":[],"inputs":[frame_port],"outputs":[],
      "implementation":{"kind":"native_plugin","abi":"isos_node_v2","library":"lib/sink-v2.so"}}],
    "port_routes":[{"source_type":source_type,"output":"frame","target_type":sink_type,"input":"frame",
      "transport":"in_process","delivery_mode":"event","data_type":"image_frame@v0.2",
      "contract":{"owner":"eos","id":"image-local","version":"1"}},
      {"source_type":source_type,"output":"frame","target_type":unsupported_sink_type,"input":"frame",
      "transport":"in_process","delivery_mode":"event","data_type":"image_frame@v0.2",
      "contract":{"owner":"eos","id":"image-local","version":"1"}}]})
    .to_string()).unwrap();
  for (store, sidecar) in [
    (&source_store, &source_sidecar),
    (&sink_store, &sink_sidecar),
  ] {
    fs::write(
      store.join("receipt.json"),
      json!({"local_link":store,
      "payload_sha256":"b".repeat(64),"sidecar_sha256":digest(sidecar)})
      .to_string(),
    )
    .unwrap();
  }
  let owner_socket = root.join("owner-v2.sock");
  let mut owner = Command::new(env!("CARGO_BIN_EXE_aether-profile"))
    .env("EOS_LOG_DIR", root)
    .args(["owner", "--socket"])
    .arg(&owner_socket)
    .arg("--node-types")
    .arg(&source_sidecar)
    .arg("--node-types")
    .arg(&sink_sidecar)
    .spawn()
    .unwrap();
  wait_socket(&owner_socket);
  let camera = json!({"id":"front","node_type":source_type,
    "package":source_pin,"label":"Service camera","description":"V2 observer","frame_id":"camera",
    "capabilities":["rgb8"],"adapter":{"type":"service-v2","package":source_pin,
      "settings":{"service_instance":"camera-a","device_id":"cam-0"}},
    "health_timing":{"poll_interval_ms":100,"stale_after_ms":500},
    "observation":{"max_rate_hz":30,"max_payload_bytes":12000000},"movement":null});
  let sink = json!({"id":"sink","node_type":sink_type,
    "package":sink_pin,"label":"Sink","description":"V1 consumer",
    "inputs":{"frame":{"node":"eos/hardware/camera/front","output":"frame"}}});
  let graph = json!({"version":"0.2.1","robot":{"id":"robot","name":"Robot"},
  "cores":{"eos":{"layers":{
    "hardware":{"groups":{"camera":{"nodes":{"front":camera}}}},
    "skill":{"groups":{"package":{"nodes":{"sink":sink}}}}
  }}}});
  let registry =
    NodeTypeRegistry::with_contribution_files(&[source_sidecar.clone(), sink_sidecar.clone()])
      .unwrap();
  aether_hal::Profile::from_json_with_registry(&graph.to_string(), &registry).unwrap();
  let packages = json!([{"id":"vendor/service-camera","version":"0.2.1","path":source_store,"payload_root":source_store},
    {"id":"vendor/software-sink","version":"0.2.1","path":sink_store,"payload_root":sink_store}]);
  let generation = format!(
    "{:x}",
    Sha256::digest(format!("run_3:{revision}:vendor/service-camera@0.2.1/camera-a").as_bytes())
  );
  let endpoints = json!({"package_services":{"eos/hardware/camera/front":{
    "package":source_pin,"service_instance":"camera-a","socket_path":root.join("service.sock"),
    "service_generation":generation,"config_digest":"c".repeat(64)}}});
  let mut vision_graph = graph.clone();
  vision_graph["cores"]["eos"]["layers"]["interface"] = json!({"groups":{"egress":{"nodes":{"front":{
    "id":"front","node_type":"eos.interface.egress.vision_flow@v0.2","label":"Front vision",
    "description":"Camera transfer","inputs":{"frame":{"node":"eos/hardware/camera/front","output":"frame"}}
  }}}}});
  aether_hal::Profile::from_json_with_registry(&vision_graph.to_string(), &registry).unwrap();
  let missing_frequency = exchange(
    &owner_socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run",
    "run_id":"run_3","revision":revision,"graph":vision_graph,"packages":packages,"endpoints":endpoints,
    "launch":{"sha256":digest(Path::new(env!("CARGO_BIN_EXE_aether-runtime"))),"args":[],"env":{}}}),
  );
  assert_eq!(
    missing_frequency["error"], "source_frequency_hz_required",
    "{missing_frequency}"
  );
  assert!(missing_frequency["diagnostic"]
    .as_str()
    .unwrap()
    .contains("eos/hardware/camera/front"));
  assert!(missing_frequency["diagnostic"]
    .as_str()
    .unwrap()
    .contains("eos/interface/egress/front"));
  vision_graph["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"]["nodes"]["front"]
    ["adapter"]["settings"]["frequency_hz"] = json!(20.0);
  let valid_frequency = exchange(
    &owner_socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run",
    "run_id":"run_3","revision":revision,"graph":vision_graph,"packages":packages,"endpoints":endpoints,
    "launch":{"sha256":digest(Path::new(env!("CARGO_BIN_EXE_aether-runtime"))),"args":[],"env":{}}}),
  );
  assert_eq!(
    valid_frequency["error"], "service_not_ready",
    "{valid_frequency}"
  );
  let mut unsupported_graph = graph.clone();
  unsupported_graph["cores"]["eos"]["layers"]["skill"]["groups"]["package"]["nodes"]["sink"]
    ["node_type"] = json!(unsupported_sink_type);
  let unsupported = exchange(
    &owner_socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run",
    "run_id":"run_bad","revision":revision,"graph":unsupported_graph,"packages":packages,"endpoints":endpoints,
    "launch":{"sha256":digest(Path::new(env!("CARGO_BIN_EXE_aether-runtime"))),"args":[],"env":{}}}),
  );
  assert_eq!(
    unsupported["error"], "unsupported_execution",
    "{unsupported}"
  );
  assert!(
    unsupported["diagnostic"]
      .as_str()
      .unwrap()
      .starts_with("unsupported_v2_software_input"),
    "{unsupported}"
  );
  let not_ready = exchange(
    &owner_socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run",
    "run_id":"run_3","revision":revision,"graph":graph,"packages":packages,"endpoints":endpoints,
    "launch":{"sha256":digest(Path::new(env!("CARGO_BIN_EXE_aether-runtime"))),"args":[],"env":{}}}),
  );
  assert_eq!(not_ready["error"], "service_not_ready", "{not_ready}");
  let service = UnixListener::bind(&service_socket).unwrap();
  let health_generation = generation.clone();
  let service_thread = thread::spawn(move || {
    for index in 0..3 {
      let (mut client, _) = service.accept().unwrap();
      let mut header = [0u8; 8];
      client.read_exact(&mut header).unwrap();
      let size = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
      assert_eq!(u32::from_le_bytes(header[4..].try_into().unwrap()), 0);
      let mut body = vec![0u8; size];
      client.read_exact(&mut body).unwrap();
      let request: Value = serde_json::from_slice(&body).unwrap();
      assert_eq!(request["node"], "cam-0");
      assert_eq!(request["generation_id"], health_generation);
      let reply = serde_json::to_vec(&json!({"version":"2.0.0","id":request["id"],
        "ok":true,"accepted":true,"generation_id":if index == 0 { "stale" } else { &health_generation },
        "features":["strict-socket-v1"]}))
      .unwrap();
      header[..4].copy_from_slice(&(reply.len() as u32).to_le_bytes());
      client.write_all(&header).unwrap();
      client.write_all(&reply).unwrap();
    }
    let (mut client, _) = service.accept().unwrap();
    let mut request = [0u8; 8];
    client.read_exact(&mut request).unwrap();
    assert_eq!(&request, b"observe\n");
    client.write_all(br#"{"port":"frame","payload":{"sample":{"source_time_ns":1,"clock":"simulation","sequence":73,"frame_id":"camera"},"width_px":1,"height_px":1,"encoding":"rgb8","data":"AAAA"}}"#).unwrap();
  });
  let stale_health = exchange(
    &owner_socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run",
    "run_id":"run_3","revision":revision,"graph":graph,"packages":packages,"endpoints":endpoints,
    "launch":{"sha256":digest(Path::new(env!("CARGO_BIN_EXE_aether-runtime"))),"args":[],"env":{}}}),
  );
  assert_eq!(stale_health["error"], "service_not_ready", "{stale_health}");
  let run = exchange(
    &owner_socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run",
    "run_id":"run_3","revision":revision,"graph":graph,"packages":packages,"endpoints":endpoints,
    "launch":{"sha256":digest(Path::new(env!("CARGO_BIN_EXE_aether-runtime"))),"args":[],"env":{}}}),
  );
  assert_eq!(run["ok"], true, "{run}");
  let status = exchange(
    &owner_socket,
    "status",
    json!({"schema_version":1,"core":"eos","op":"status",
    "run_id":"run_3","revision":revision}),
  );
  assert_eq!(status["state"], "ready", "{status}");
  let stale = exchange(
    &owner_socket,
    "observe",
    json!({"schema_version":1,"core":"eos","op":"observe",
    "run_id":"run_3","revision":"a".repeat(64),"node":"eos/hardware/camera/front"}),
  );
  assert_eq!(stale["ok"], false, "{stale}");
  let observed = exchange(
    &owner_socket,
    "observe",
    json!({"schema_version":1,"core":"eos","op":"observe",
    "run_id":"run_3","revision":revision,"node":"eos/hardware/camera/front"}),
  );
  assert_eq!(observed["ok"], true, "{observed}");
  assert_eq!(
    observed["outputs"][0]["output"]["payload"]["sample"]["sequence"],
    73
  );
  assert!(
    observed["outputs"]
      .as_array()
      .unwrap()
      .iter()
      .any(|output| output["node"] == "eos/skill/package/sink"
        && output["output"]["port"] == "frame"
        && output["output"]["payload"]["sample"]["sequence"] == 74),
    "{observed}"
  );
  service_thread.join().unwrap();
  fs::remove_file(&service_socket).unwrap();
  let failed = exchange(
    &owner_socket,
    "status",
    json!({"schema_version":1,"core":"eos","op":"status",
    "run_id":"run_3","revision":revision}),
  );
  assert_eq!(failed["state"], "failed", "{failed}");
  let mut health_socket = UnixStream::connect(&owner_socket).unwrap();
  health_socket
    .write_all(b"GET /v1/health HTTP/1.1\r\nHost: owner\r\n\r\n")
    .unwrap();
  let mut health = String::new();
  health_socket.read_to_string(&mut health).unwrap();
  assert!(health.contains("\"state\":\"failed\""), "{health}");
  let lost = exchange(
    &owner_socket,
    "observe",
    json!({"schema_version":1,"core":"eos","op":"observe",
    "run_id":"run_3","revision":revision,"node":"eos/hardware/camera/front"}),
  );
  assert_eq!(lost["ok"], false, "{lost}");
  let stopped = exchange(
    &owner_socket,
    "stop",
    json!({"schema_version":1,"core":"eos","op":"stop",
    "run_id":"run_3","revision":revision}),
  );
  assert_eq!(stopped["state"], "stopped", "{stopped}");
  owner.kill().unwrap();
  owner.wait().unwrap();
}
