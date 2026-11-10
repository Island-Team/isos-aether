use std::{
  fs,
  io::{Read, Write},
  os::unix::net::UnixStream,
  path::Path,
  process::Command,
  thread,
  time::Duration,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
fn wait_socket(path: &Path) {
  for _ in 0..100 {
    if path.exists() {
      return;
    }
    thread::sleep(Duration::from_millis(20));
  }
  panic!("socket absent: {}", path.display());
}
fn owner_request(path: &Path, op: &str, request: Value) -> Value {
  let bytes = serde_json::to_vec(&request).unwrap();
  let mut stream = UnixStream::connect(path).unwrap();
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
fn node(id: &str, ty: &str, input: Option<(&str, &str, &str)>) -> Value {
  let mut value = json!({"id":id,"node_type":ty,"label":id,"description":"Demo node","observation":null,"movement":null});
  if let Some((port, source, output)) = input {
    value["inputs"][port] = json!({"node":source,"output":output});
  }
  if id == "arm" {
    value["frame_id"] = json!("mock_arm");
    value["joint_names"] =
      json!(["joint_1", "joint_2", "joint_3", "joint_4", "joint_5", "joint_6"]);
    value["adapter"] = json!({"type":"mock-arm","settings":{}});
    value["capabilities"] = json!(["joint_positions"]);
    value["health_timing"] = json!({"poll_interval_ms":100,"stale_after_ms":500});
    value["observation"] = json!({"max_rate_hz":10,"max_payload_bytes":1024});
    value["movement"] = json!({"modes":["joint_positions"],"limits":{"max_joint_position_rad":314.0 / 100.0,"max_joint_speed_rad_s":1}});
  }
  value
}
fn graph() -> Value {
  json!({"version":"0.2.0","robot":{"id":"robot","name":"Robot"},"cores":{"eos":{"layers":{
  "interface":{"groups":{"ingress":{"nodes":{"trigger":node("trigger","eos.interface.ingress.trigger@v0.2",Some(("activation","ei/governance/safety/gate","activation")))}} ,"egress":{"nodes":{"arm_status":node("arm_status","eos.interface.egress.arm_status@v0.2",Some(("observation","eos/skill/demo/arm_status","observation")))}}}},
  "skill":{"groups":{"demo":{"nodes":{"joint_delta":node("joint_delta","eos.skill.demo.joint_delta@v0.2",Some(("activation","eos/interface/ingress/trigger","activation"))),"arm_status":node("arm_status","eos.skill.demo.arm_status@v0.2",Some(("observation","eos/dispatcher/arm/monitor","observation")))}}}},
  "dispatcher":{"groups":{"arm":{"nodes":{"delta":node("delta","eos.dispatcher.arm.delta@v0.2",Some(("delta","eos/skill/demo/joint_delta","delta"))),"monitor":node("monitor","eos.dispatcher.arm.monitor@v0.2",Some(("observation","eos/hardware/mock/arm_observer","observation")))}}}},
  "hardware":{"groups":{"mock":{"nodes":{"arm":node("arm","eos.hardware.mock.arm@v0.2",Some(("delta","eos/dispatcher/arm/delta","delta"))),"arm_observer":node("arm_observer","eos.hardware.mock.arm_observer@v0.2",None)}}}}
  }}}})
}
#[test]
fn owner_runs_real_worker_and_activation_moves_mock_once() {
  let dir = tempfile::tempdir().unwrap();
  let socket = dir.path().join("owner.sock");
  let mut owner = Command::new(env!("CARGO_BIN_EXE_aether-profile"))
    .env("EOS_LOG_DIR", dir.path())
    .arg("owner")
    .arg("--socket")
    .arg(&socket)
    .spawn()
    .unwrap();
  wait_socket(&socket);
  let revision = "a".repeat(64);
  let digest = format!(
    "{:x}",
    Sha256::digest(fs::read(env!("CARGO_BIN_EXE_aether-runtime")).unwrap())
  );
  let options = owner_request(
    &socket,
    "options",
    json!({"schema_version":1,"core":"eos","op":"options","revision":revision,"graph":graph(),"endpoints":{},"packages":[]}),
  );
  assert_eq!(options["ok"], true, "{options}");
  let adapter = &options["catalog"]["setting_options"][0];
  assert_eq!(adapter["node_path"], "eos/hardware/mock/arm");
  assert_eq!(adapter["setting"], "adapter.type");
  assert_eq!(adapter["selected"], "mock-arm");
  assert_eq!(adapter["options"][0]["available"], true);
  let mut configured_graph = graph();
  configured_graph["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm"]
    ["settings"] = json!({"sample_interval_ms":20,"freshness_timeout_ms":80});
  let run = owner_request(
    &socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run","run_id":"run_1","revision":revision,"graph":configured_graph,"launch":{"sha256":digest,"args":[],"env":{}}}),
  );
  assert_eq!(run["ok"], true, "{run}");
  let status = owner_request(
    &socket,
    "status",
    json!({"schema_version":1,"core":"eos","op":"status","run_id":"run_1","revision":revision}),
  );
  assert_eq!(status["state"], "ready");
  assert_eq!(status["mock_hardware"], "ready");
  let monitor = || {
    owner_request(
      &socket,
      "monitor",
      json!({"schema_version":1,"core":"eos","op":"monitor","run_id":"run_1","revision":revision}),
    )
  };
  let initial = monitor();
  assert_eq!(initial["ok"], true, "{initial}");
  assert_eq!(initial["observation"]["state"], "ready");
  assert_eq!(
    initial["observation"]["joint_positions_rad"],
    json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
  );
  assert_eq!(
    initial["observation"]["source_node"],
    "eos/hardware/mock/arm_observer"
  );
  assert_eq!(
    initial["observation"]["joint_names"],
    json!(["joint_1", "joint_2", "joint_3", "joint_4", "joint_5", "joint_6"])
  );
  assert!(
    initial["observation"]["sampled_at_monotonic_ns"]
      .as_u64()
      .unwrap()
      > 0
  );
  thread::sleep(Duration::from_millis(35));
  let sampled = monitor();
  assert!(
    sampled["observation"]["sequence"].as_u64().unwrap()
      > initial["observation"]["sequence"].as_u64().unwrap()
  );
  let activation = |event: &str| json!({"schema_version":1,"core":"eos","op":"activate","run_id":"run_1","revision":revision,"source_node":"isos/ui/button/trigger","event_id":event,"kind":"activate"});
  let result = |event: &str| {
    owner_request(
      &socket,
      "result",
      json!({"schema_version":1,"core":"eos","op":"result","run_id":"run_1","revision":revision,"event_id":event}),
    )
  };
  let await_result = |event: &str| {
    for _ in 0..100 {
      let reply = result(event);
      if !matches!(reply["status"].as_str(), Some("accepted" | "running")) {
        return reply;
      }
      thread::sleep(Duration::from_millis(10));
    }
    panic!("result timed out for {event}");
  };
  let first = owner_request(&socket, "activate", activation("click-1"));
  assert_eq!(first["status"], "accepted", "{first}");
  assert!(first.get("joint_positions_rad").is_none());
  assert_eq!(
    owner_request(&socket, "activate", activation("click-1")),
    first
  );
  let completed = await_result("click-1");
  assert_eq!(completed["status"], "succeeded", "{completed}");
  let moved = monitor();
  assert_eq!(
    moved["observation"]["joint_positions_rad"],
    json!([0.05, -0.04, 0.03, -0.02, 0.01, -0.01])
  );
  assert!(
    moved["observation"]["sequence"].as_u64().unwrap()
      > sampled["observation"]["sequence"].as_u64().unwrap()
  );
  assert_eq!(
    completed["joint_positions_rad"],
    json!([0.05, -0.04, 0.03, -0.02, 0.01, -0.01])
  );
  assert_eq!(
    owner_request(&socket, "activate", activation("click-1")),
    first
  );
  let second = owner_request(&socket, "activate", activation("click-2"));
  assert_eq!(second["status"], "accepted");
  assert_eq!(
    await_result("click-2")["joint_positions_rad"],
    json!([0.1, -0.08, 0.06, -0.04, 0.02, -0.02])
  );
  let mut other_button = activation("click-3");
  other_button["source_node"] = json!("isos/ui/button/other");
  let third = owner_request(&socket, "activate", other_button);
  assert_eq!(third["status"], "accepted");
  assert!(
    (await_result("click-3")["joint_positions_rad"][0]
      .as_f64()
      .unwrap()
      - 0.15)
      .abs()
      < 1e-12
  );
  assert_eq!(result("missing")["error"], "event_not_found");
  let mut invalid_button = activation("click-4");
  invalid_button["source_node"] = json!("isos/ui/button/invalid.dot");
  assert_eq!(
    owner_request(&socket, "activate", invalid_button)["error"],
    "invalid_activation"
  );
  let mut stale = activation("click-4");
  stale["revision"] = json!("b".repeat(64));
  assert_eq!(
    owner_request(&socket, "activate", stale)["error"],
    "stale_or_missing_run"
  );
  let stale_monitor = owner_request(
    &socket,
    "monitor",
    json!({"schema_version":1,"core":"eos","op":"monitor","run_id":"run_1","revision":"b".repeat(64)}),
  );
  assert_eq!(stale_monitor["error"], "stale_or_missing_run");
  let stop = owner_request(
    &socket,
    "stop",
    json!({"schema_version":1,"core":"eos","op":"stop","run_id":"run_1","revision":revision}),
  );
  assert_eq!(stop["state"], "stopped");
  assert_eq!(monitor()["error"], "stale_or_missing_run");
  assert_eq!(
    owner_request(&socket, "activate", activation("click-3"))["error"],
    "stale_or_missing_run"
  );
  let mut incomplete = graph();
  incomplete["cores"]["eos"]["layers"]["skill"]["groups"]["demo"]["nodes"]["joint_delta"] =
    Value::Null;
  let invalid_run = owner_request(
    &socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run","run_id":"run_2","revision":revision,"graph":incomplete,"launch":{"sha256":digest,"args":[],"env":{}}}),
  );
  assert_eq!(invalid_run["error"], "invalid_eos_graph");
  let mut missing_monitor = graph();
  missing_monitor["cores"]["eos"]["layers"]["dispatcher"]["groups"]["arm"]["nodes"]["monitor"] =
    Value::Null;
  let invalid_monitor_run = owner_request(
    &socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run","run_id":"run_2","revision":revision,"graph":missing_monitor,"launch":{"sha256":digest,"args":[],"env":{}}}),
  );
  assert_eq!(invalid_monitor_run["error"], "invalid_eos_graph");
  let mut legacy = graph();
  let hardware = &mut legacy["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"];
  let mut output = hardware
    .as_object_mut()
    .unwrap()
    .remove("arm_observer")
    .unwrap();
  output["id"] = json!("arm_output");
  output["node_type"] = json!("eos.hardware.mock.arm_output@v0.2");
  output["inputs"]["observation"] = json!({"node":"eos/hardware/mock/arm","output":"observation"});
  hardware["arm_output"] = output;
  legacy["cores"]["eos"]["layers"]["dispatcher"]["groups"]["arm"]["nodes"]["monitor"]["inputs"]
    ["observation"]["node"] = json!("eos/hardware/mock/arm_output");
  let legacy_run = owner_request(
    &socket,
    "run",
    json!({"schema_version":1,"core":"eos","op":"run","run_id":"run_legacy","revision":revision,"graph":legacy,"launch":{"sha256":digest,"args":[],"env":{}}}),
  );
  assert_eq!(legacy_run["ok"], true, "{legacy_run}");
  let legacy_monitor = owner_request(
    &socket,
    "monitor",
    json!({"schema_version":1,"core":"eos","op":"monitor","run_id":"run_legacy","revision":revision}),
  );
  assert_eq!(
    legacy_monitor["observation"]["source_node"],
    "eos/hardware/mock/arm"
  );
  let legacy_stop = owner_request(
    &socket,
    "stop",
    json!({"schema_version":1,"core":"eos","op":"stop","run_id":"run_legacy","revision":revision}),
  );
  assert_eq!(legacy_stop["state"], "stopped");
  owner.kill().unwrap();
  owner.wait().unwrap();
}
