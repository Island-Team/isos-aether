use aether_node_profile::{NodeTypeRegistry, Profile};
use serde_json::{json, Value};
fn node(id: &str, node_type: &str, input: Option<(&str, &str, &str)>) -> Value {
  let mut value = json!({"id":id,"node_type":node_type,"label":id,"description":"Demo node","observation":null,"movement":null});
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
fn demo_projection_has_eight_typed_nodes_and_six_local_edges() {
  let registry = NodeTypeRegistry::builtin();
  let profile = Profile::from_json_with_registry(&graph().to_string(), &registry).unwrap();
  assert_eq!(profile.all_nodes().count(), 8);
  assert_eq!(profile.hardware_nodes().count(), 0); // EOS projection owns its own hardware layer.
  assert_eq!(profile.eos_edge_contract(&registry).unwrap().len(), 6);
}
#[test]
fn legacy_arm_output_route_remains_valid() {
  let mut value = graph();
  let hardware = &mut value["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"];
  let mut source = hardware
    .as_object_mut()
    .unwrap()
    .remove("arm_observer")
    .unwrap();
  source["id"] = json!("arm_output");
  source["node_type"] = json!("eos.hardware.mock.arm_output@v0.2");
  source["inputs"]["observation"] = json!({"node":"eos/hardware/mock/arm","output":"observation"});
  hardware["arm_output"] = source;
  value["cores"]["eos"]["layers"]["dispatcher"]["groups"]["arm"]["nodes"]["monitor"]["inputs"]
    ["observation"]["node"] = json!("eos/hardware/mock/arm_output");
  let registry = NodeTypeRegistry::builtin();
  let profile = Profile::from_json_with_registry(&value.to_string(), &registry).unwrap();
  assert_eq!(profile.eos_edge_contract(&registry).unwrap().len(), 7);
  assert!(profile.validate_demo_chain().is_ok());
}
#[test]
fn observer_rejects_command_chain_input() {
  let mut value = graph();
  value["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm_observer"]
    ["inputs"]["observation"] = json!({"node":"eos/hardware/mock/arm","output":"observation"});
  assert!(Profile::from_json(&value.to_string()).is_err());
}
#[test]
fn stale_types_and_wrong_placement_are_rejected() {
  let mut value = graph();
  value["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm"]["node_type"] =
    json!("eos.hardware.mock.health@v0.2");
  assert!(Profile::from_json(&value.to_string()).is_err());
  let ids = aether_node_profile::builtin_reader_node_types();
  assert_eq!(
    ids,
    vec![
      "eos.dispatcher.arm.delta@v0.2",
      "eos.dispatcher.arm.monitor@v0.2",
      "eos.hardware.mock.arm@v0.2",
      "eos.hardware.mock.arm_observer@v0.2",
      "eos.hardware.mock.arm_output@v0.2",
      "eos.interface.egress.arm_status@v0.2",
      "eos.interface.ingress.trigger@v0.2",
      "eos.skill.demo.arm_status@v0.2",
      "eos.skill.demo.joint_delta@v0.2"
    ]
  );
}

#[test]
fn mock_arm_sampling_settings_are_bounded_and_coherent() {
  let mut value = graph();
  let set = |value: &mut Value, settings: Value| {
    value["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm"]["settings"] =
      settings;
  };
  set(
    &mut value,
    json!({"sample_interval_ms":20,"freshness_timeout_ms":80}),
  );
  let profile = Profile::from_json(&value.to_string()).unwrap();
  assert!(profile.validate_demo_chain().is_ok());
  set(
    &mut value,
    json!({"sample_interval_ms":0,"freshness_timeout_ms":80}),
  );
  assert!(Profile::from_json(&value.to_string()).is_err());
  set(
    &mut value,
    json!({"sample_interval_ms":1001,"freshness_timeout_ms":5000}),
  );
  assert!(Profile::from_json(&value.to_string()).is_err());
  set(
    &mut value,
    json!({"sample_interval_ms":20,"freshness_timeout_ms":30}),
  );
  assert!(Profile::from_json(&value.to_string())
    .unwrap()
    .validate_demo_chain()
    .is_err());
}
