use aether_node_profile::{NodeTypeContribution, NodeTypeRegistry, Profile};
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
  json!({"version":"0.2.1","robot":{"id":"robot","name":"Robot"},"cores":{"eos":{"layers":{
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
fn node_description_may_be_empty_or_omitted() {
  let registry = NodeTypeRegistry::builtin();
  let mut value = graph();
  let nodes = &mut value["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"];
  nodes["arm"]["description"] = json!("");
  nodes["arm_observer"]
    .as_object_mut()
    .unwrap()
    .remove("description");
  let profile = Profile::from_json_with_registry(&value.to_string(), &registry).unwrap();
  assert_eq!(
    profile.node("eos/hardware/mock/arm").unwrap().description,
    ""
  );
  assert_eq!(
    profile
      .node("eos/hardware/mock/arm_observer")
      .unwrap()
      .description,
    ""
  );
}
#[test]
fn configured_input_rate_round_trips_and_rejects_invalid_ranges() {
  let mut value = graph();
  let input = &mut value["cores"]["eos"]["layers"]["interface"]["groups"]["egress"]["nodes"]
    ["arm_status"]["inputs"]["observation"];
  *input = json!({"sources":{"node":"eos/skill/demo/arm_status","output":"observation"},"desired_rate_hz":{"min":5.0,"max":20.0}});
  let registry = NodeTypeRegistry::builtin();
  let profile = Profile::from_json_with_registry(&value.to_string(), &registry).unwrap();
  let configured = profile
    .node("eos/interface/egress/arm_status")
    .unwrap()
    .inputs
    .get("observation")
    .unwrap();
  assert_eq!(configured.desired_rate_hz().unwrap().max, Some(20.0));
  assert!(serde_json::to_value(&profile)
    .unwrap()
    .to_string()
    .contains("desired_rate_hz"));
  value["cores"]["eos"]["layers"]["interface"]["groups"]["egress"]["nodes"]["arm_status"]
    ["inputs"]["observation"]["desired_rate_hz"]["min"] = json!(21.0);
  assert!(Profile::from_json_with_registry(&value.to_string(), &registry).is_err());
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
      "eos.dataset.export.manifest@v0.2",
      "eos.dispatcher.arm.delta@v0.2",
      "eos.dispatcher.arm.monitor@v0.2",
      "eos.dispatcher.command.binding@v0.2",
      "eos.evidence.episode.recorder@v0.2",
      "eos.hardware.mock.arm@v0.2",
      "eos.hardware.mock.arm_observer@v0.2",
      "eos.hardware.mock.arm_output@v0.2",
      "eos.hardware.mock.camera@v0.2",
      "eos.interface.egress.arm_status@v0.2",
      "eos.interface.egress.vision_flow@v0.2",
      "eos.interface.ingress.trigger@v0.2",
      "eos.perception.physical.features@v0.2",
      "eos.safety.device.estop@v0.2",
      "eos.safety.device.guard@v0.2",
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
  assert!(profile.demo_activation_chain_complete());
  assert!(profile.demo_monitor_chain_complete());
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

#[test]
fn mock_arm_rejects_position_limit_that_runtime_cannot_honor() {
  let mut value = graph();
  value["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm"]["movement"]
    ["limits"]["max_joint_position_rad"] = json!(3.15);
  let profile = Profile::from_json(&value.to_string()).unwrap();
  let error = profile.validate_demo_chain().unwrap_err();
  assert!(error.to_string().contains("exceeds 3.14 rad safety bound"));
}

#[test]
fn disconnected_demo_activation_is_valid_but_cannot_activate() {
  let mut value = graph();
  value["cores"]["eos"]["layers"]["skill"]["groups"]["demo"]["nodes"]["joint_delta"]["inputs"] =
    json!({});
  let profile = Profile::from_json(&value.to_string()).unwrap();
  profile.validate_demo_chain().unwrap();
  assert!(!profile.demo_activation_chain_complete());
  assert!(profile.demo_monitor_chain_complete());
}

#[test]
fn fully_disconnected_demo_keeps_observer_without_command_route() {
  let mut value = graph();
  for layer in value["cores"]["eos"]["layers"]
    .as_object_mut()
    .unwrap()
    .values_mut()
  {
    for group in layer["groups"].as_object_mut().unwrap().values_mut() {
      for node in group["nodes"].as_object_mut().unwrap().values_mut() {
        if node.get("inputs").is_some() {
          node["inputs"] = json!({});
        }
      }
    }
  }
  let profile = Profile::from_json(&value.to_string()).unwrap();
  profile.validate_demo_chain().unwrap();
  assert!(!profile.demo_activation_chain_complete());
  assert!(!profile.demo_monitor_chain_complete());
}

#[test]
fn non_mock_hardware_requires_package_registration() {
  let value = json!({
    "version":"0.2.1","robot":{"id":"robot","name":"Robot"},
    "cores":{"eos":{"layers":{"hardware":{"groups":{"camera":{"nodes":{
      "front":{
        "id":"front","node_type":"eos.hardware.camera.device@v0.2",
        "label":"Front camera","description":"Front image source",
        "frame_id":"front_optical","adapter":{"type":"unknown"},
        "capabilities":["rgb8"],
        "health_timing":{"poll_interval_ms":100,"stale_after_ms":500},
        "observation":{"max_rate_hz":30,"max_payload_bytes":12000000},
        "movement":null
      }
    }}}}}}}
  });
  let error =
    Profile::from_json_with_registry(&value.to_string(), &NodeTypeRegistry::builtin()).unwrap_err();
  assert!(error.to_string().contains("unknown node type"));
}

#[test]
fn package_adapter_family_requires_its_exact_pin() {
  let mut registry = NodeTypeRegistry::builtin();
  let contribution: NodeTypeContribution = serde_json::from_value(json!({
    "package":{"id":"vendor/driver","version":"1.0.0"},
    "libraries":[{"path":"lib/driver.so","sha256":"a".repeat(64)}],
    "node_types":[{
      "id":"eos.hardware.camera.device@v0.2","core":"eos","layer":"hardware",
      "group":"camera","kind":"camera","lifecycle":"adapter",
      "description":"Vendor camera","capabilities":["rgb8"],"settings":[],"inputs":[],
      "outputs":[{"name":"frame","data_type":"image_frame@v0.2","required":false,
        "description":"Camera frame","frequency_hz":30.0,"transport":"in_process","delivery_mode":"event",
        "data_class":"image","contract":{"owner":"eos","id":"image-local","version":"1"}}],
      "implementation":{"kind":"native_plugin","abi":"isos_node_v2","library":"lib/driver.so"}
    }],
    "adapter_families":[{
      "id":"vendor-socket","device_kinds":["camera"],
      "transport":"unix_socket","description":"Vendor camera adapter",
      "supported_primitives":[]
    }]
  }))
  .unwrap();
  registry.register_contribution(contribution).unwrap();
  assert_eq!(
    registry
      .get("eos.hardware.camera.device@v0.2")
      .unwrap()
      .outputs[0]
      .frequency_hz,
    Some(30.0)
  );
  assert_eq!(
    registry
      .get("eos.interface.egress.vision_flow@v0.2")
      .unwrap()
      .outputs[0]
      .frequency_hz,
    None
  );
  let graph = json!({
    "version":"0.2.1","robot":{"id":"robot","name":"Robot"},
    "cores":{"eos":{"layers":{"hardware":{"groups":{"camera":{"nodes":{
      "front":{
        "id":"front","node_type":"eos.hardware.camera.device@v0.2",
        "package":{"id":"vendor/driver","version":"1.0.0"},
        "label":"Front camera","description":"Front image source",
        "frame_id":"front_optical","capabilities":["rgb8"],
        "adapter":{"type":"vendor-socket","package":{"id":"other/driver","version":"1.0.0"}},
        "health_timing":{"poll_interval_ms":100,"stale_after_ms":500},
        "observation":{"max_rate_hz":30,"max_payload_bytes":12000000},
        "movement":null
      }
    }}}}}}}
  });
  let mut valid = graph.clone();
  valid["cores"]["eos"]["layers"]["hardware"]["groups"]["camera"]["nodes"]["front"]["adapter"]
    ["package"] = json!({"id":"vendor/driver","version":"1.0.0"});
  valid["cores"]["eos"]["layers"]["interface"]["groups"]["egress"]["nodes"]["vision"] = json!({"id":"vision","node_type":"eos.interface.egress.vision_flow@v0.2",
      "label":"Vision","description":"Camera transfer","observation":null,"movement":null,
      "inputs":{"frame":{"sources":{"node":"eos/hardware/camera/front","output":"frame"},
        "desired_rate_hz":{"max":24.0}}}});
  let profile = Profile::from_json_with_registry(&valid.to_string(), &registry).unwrap();
  assert_eq!(
    registry
      .get("eos.interface.egress.vision_flow@v0.2")
      .unwrap()
      .inputs[0]
      .data_class,
    Some(aether_node_profile::DataClass::Image)
  );
  assert!(registry.catalog_port_routes().iter().any(|route| {
    route["source_type"] == "eos.hardware.camera.device@v0.2"
      && route["target_type"] == "eos.interface.egress.vision_flow@v0.2"
  }));
  let route = profile.eos_edge_contract(&registry).unwrap();
  assert_eq!(route.len(), 1);
  assert_eq!(route[0].class, aether_node_profile::EosEdgeClass::EosLocal);
  assert_eq!(route[0].frequency_hz, Some(24.0));
  valid["cores"]["eos"]["layers"]["interface"]["groups"]["egress"]["nodes"]["vision"]["inputs"]
    ["frame"]["sources"]["frequency_hz"] = json!(10.0);
  assert!(Profile::from_json_with_registry(&valid.to_string(), &registry).is_err());
  let error = Profile::from_json_with_registry(&graph.to_string(), &registry).unwrap_err();
  assert!(error
    .to_string()
    .contains("adapter family provenance or kind mismatch"));
}

#[test]
fn package_can_register_its_own_layer_and_group() {
  let mut registry = NodeTypeRegistry::builtin();
  let contribution: NodeTypeContribution = serde_json::from_value(json!({
    "package":{"id":"vendor/skills","version":"1.0.0"},
    "libraries":[{"path":"lib/filter.so","sha256":"b".repeat(64)}],
    "node_types":[{
      "id":"eos.perception.vendor.filter@v0.2","core":"eos",
      "layer":"perception","group":"vendor","kind":"software",
      "lifecycle":"internal","description":"Vendor filter",
      "capabilities":[],"settings":[],"inputs":[],"outputs":[],
      "implementation":{"kind":"native_plugin","abi":"isos_node_v2",
        "library":"lib/filter.so"}
    }]
  }))
  .unwrap();
  registry.register_contribution(contribution).unwrap();
  let graph = json!({
    "version":"0.2.1","robot":{"id":"robot","name":"Robot"},
    "cores":{"eos":{"layers":{"perception":{"groups":{"vendor":{"nodes":{
      "filter":{"id":"filter","node_type":"eos.perception.vendor.filter@v0.2",
        "package":{"id":"vendor/skills","version":"1.0.0"},
        "label":"Filter","description":"Vendor filter"}
    }}}}}}}
  });
  assert!(Profile::from_json_with_registry(&graph.to_string(), &registry).is_ok());
  let mut unpinned = graph;
  unpinned["cores"]["eos"]["layers"]["perception"]["groups"]["vendor"]["nodes"]["filter"]
    .as_object_mut()
    .unwrap()
    .remove("package");
  assert!(Profile::from_json_with_registry(&unpinned.to_string(), &registry).is_err());
}
