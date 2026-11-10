use aether_node_profile::{DataClass, NodeTypeRegistry, PortTransport};
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
