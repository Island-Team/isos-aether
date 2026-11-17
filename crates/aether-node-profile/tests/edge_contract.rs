use aether_node_profile::{DataClass, NodeTypeRegistry, PortTransport};
use serde_json::json;

#[test]
fn general_pose_and_normalized_command_reject_invalid_values() {
  let registry = NodeTypeRegistry::builtin();
  let sample = json!({"source_time_ns":1,"clock":"simulation","sequence":1,"frame_id":"map"});
  let pose =
    json!({"sample":sample,"position_m":[0.0,0.0,0.0],"orientation_xyzw":[0.0,0.0,0.0,1.0]});
  registry
    .validate_data_payload("pose_state@v0.2", &pose)
    .unwrap();
  let mut invalid = pose.clone();
  invalid["orientation_xyzw"] = json!([0.0, 0.0, 0.0, 0.5]);
  assert!(registry
    .validate_data_payload("pose_state@v0.2", &invalid)
    .is_err());
  assert!(registry
    .validate_data_payload(
      "normalized_axes_command@v0.2",
      &json!({"axes":{"forward":1.1},"lease_ms":100})
    )
    .is_err());
  registry
    .validate_data_payload(
      "normalized_axes_command@v0.2",
      &json!({"axes":{"forward":1.0},"lease_ms":100}),
    )
    .unwrap();
}
#[test]
fn demo_catalog_ports_have_live_contracts() {
  let registry = NodeTypeRegistry::builtin();
  let ingress = registry.get("eos.interface.ingress.trigger@v0.2").unwrap();
  assert_eq!(ingress.inputs[0].transport, Some(PortTransport::UnixSocket));
  assert_eq!(ingress.inputs[0].contract.as_ref().unwrap().owner, "ei");
  let skill = registry.get("eos.skill.demo.joint_delta@v0.2").unwrap();
  assert_eq!(skill.outputs[0].data_type.id(), "arm_joint_delta@v0.2");
  let arm = registry.get("eos.hardware.mock.arm@v0.2").unwrap();
  assert_eq!(arm.inputs[0].data_type.id(), "arm_joint_delta@v0.2");
  assert_eq!(arm.settings.len(), 2);
  assert_eq!(arm.settings[0].name, "sample_interval_ms");
  assert_eq!(
    arm.settings[0].default.as_ref().unwrap().as_integer(),
    Some(10)
  );
  assert_eq!(arm.settings[1].name, "freshness_timeout_ms");
  assert_eq!(
    arm.settings[1].default.as_ref().unwrap().as_integer(),
    Some(50)
  );
  assert_eq!(arm.outputs[0].data_type.id(), "arm_monitor_state@v0.2");
  let path = [
    "eos.hardware.mock.arm_observer@v0.2",
    "eos.dispatcher.arm.monitor@v0.2",
    "eos.skill.demo.arm_status@v0.2",
    "eos.interface.egress.arm_status@v0.2",
  ];
  let source = registry.get(path[0]).unwrap();
  assert!(source.inputs.is_empty());
  assert_eq!(source.outputs[0].data_type.id(), "arm_monitor_state@v0.2");
  let legacy = registry.get("eos.hardware.mock.arm_output@v0.2").unwrap();
  assert_eq!(legacy.inputs[0].data_type.id(), "arm_monitor_state@v0.2");
  for ty in path.iter().skip(1) {
    let node = registry.get(ty).unwrap();
    assert_eq!(node.inputs[0].data_type.id(), "arm_monitor_state@v0.2");
    assert_eq!(node.outputs[0].data_type.id(), "arm_monitor_state@v0.2");
    assert_eq!(node.inputs[0].data_class, Some(DataClass::Observation));
    assert_eq!(
      node.inputs[0].contract.as_ref().unwrap().id,
      "arm-monitor-local"
    );
  }
  let egress = registry.get(path[3]).unwrap();
  assert_eq!(egress.outputs[0].transport, Some(PortTransport::UnixSocket));
  assert_eq!(
    egress.outputs[0].contract.as_ref().unwrap().id,
    "arm-monitor-to-ei"
  );
  assert!(registry.data_type("arm_monitor_state@v0.2").is_some());
  assert!(registry.get("eos.hardware.mock.health@v0.2").is_none());
  assert_eq!(
    registry
      .adapter_family("mock-arm")
      .unwrap()
      .supported_primitives,
    vec!["arm.delta@1.0.0"]
  );
}

#[test]
fn general_hardware_payloads_remain_available_for_packages() {
  let registry = NodeTypeRegistry::builtin();
  for (node_type, input, output) in [
    ("eos.hardware.camera.device@v0.2", None, "image_frame@v0.2"),
    (
      "eos.hardware.base.device@v0.2",
      Some("base_twist_command@v0.2"),
      "base_state@v0.2",
    ),
    (
      "eos.hardware.manipulator.device@v0.2",
      Some("joint_position_command@v0.2"),
      "joint_state@v0.2",
    ),
  ] {
    assert!(registry.get(node_type).is_none());
    assert!(registry.data_type(output).is_some());
    if let Some(command) = input {
      assert!(registry.data_type(command).is_some());
    }
  }
  let image = registry.data_type("image_frame@v0.2").unwrap();
  assert_eq!(image.codec, "json_v1");
  assert_eq!(
    image.schema["properties"]["data"]["contentEncoding"],
    "base64"
  );
}

#[test]
fn general_base_state_schema_requires_source_clock_and_units() {
  let registry = NodeTypeRegistry::builtin();
  let descriptor = registry.data_type("base_state@v0.2").unwrap();
  let validator = jsonschema::validator_for(&descriptor.schema).unwrap();
  let valid = serde_json::json!({
    "sample":{"source_time_ns":1000000000,"clock":"simulation","sequence":7,"frame_id":"map"},
    "x_m":1.25,"y_m":0.5,"yaw_rad":0.1,
    "vx_m_s":0.2,"vy_m_s":0.0,"yaw_rate_rad_s":0.05
  });
  assert!(validator.is_valid(&valid));
  assert!(registry
    .validate_data_payload("base_state@v0.2", &valid)
    .is_ok());
  let mut monotonic_clock = valid.clone();
  monotonic_clock["sample"]["clock"] = serde_json::json!("monotonic");
  assert!(registry
    .validate_data_payload("base_state@v0.2", &monotonic_clock)
    .is_ok());
  let mut unknown_clock = valid.clone();
  unknown_clock["sample"]["clock"] = serde_json::json!("unqualified");
  assert!(!validator.is_valid(&unknown_clock));
  assert!(registry
    .validate_data_payload("base_state@v0.2", &unknown_clock)
    .is_err());
  let mut missing_sample = valid;
  missing_sample.as_object_mut().unwrap().remove("sample");
  assert!(!validator.is_valid(&missing_sample));
}
