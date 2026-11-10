//! Generic registered node declaration and instance-setting admission.

use anyhow::{Result, ensure};
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::LazyLock;

static TYPE_SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
  let schema: Value = serde_json::from_str(include_str!(
    "../../../../../share/isos/contracts/node/node-type.schema.json"
  ))
  .expect("embedded node type IDL is JSON");
  jsonschema::validator_for(&schema).expect("embedded node type IDL is valid")
});

static INSTANCE_SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
  let schema: Value = serde_json::from_str(include_str!(
    "../../../../../share/isos/contracts/node/node-instance.schema.json"
  ))
  .expect("embedded node instance IDL is JSON");
  jsonschema::validator_for(&schema).expect("embedded node instance IDL is valid")
});

fn idl_valid(validator: &jsonschema::Validator, value: &Value) -> Result<()> {
  if let Err(error) = validator.validate(value) {
    anyhow::bail!("node IDL validation failed: {error}");
  }
  Ok(())
}

pub fn validate_type_idl(value: &Value) -> Result<()> {
  idl_valid(&TYPE_SCHEMA, value)
}

fn nonempty(value: &Value) -> bool {
  value.as_str().is_some_and(|s| !s.trim().is_empty())
}

fn endpoint_matches(value: &Value) -> bool {
  let Some(endpoint) = value.as_object() else {
    return false;
  };
  match endpoint.get("transport").and_then(Value::as_str) {
    Some("tcp") => {
      endpoint.len() == 3
        && endpoint.get("host").is_some_and(nonempty)
        && endpoint
          .get("port")
          .and_then(Value::as_u64)
          .is_some_and(|port| (1..=65535).contains(&port))
    }
    Some("uds") => {
      endpoint.len() == 2
        && endpoint
          .get("path")
          .and_then(Value::as_str)
          .is_some_and(|path| std::path::Path::new(path).is_absolute())
    }
    _ => false,
  }
}

fn value_matches(value: &Value, kind: &str) -> bool {
  match kind {
    "string" => value.is_string(),
    "number" => value.as_f64().is_some_and(f64::is_finite),
    "integer" => value.as_i64().is_some(),
    "boolean" => value.is_boolean(),
    "array" => value.is_array(),
    "object" => value.is_object(),
    "object_array" => value
      .as_array()
      .is_some_and(|items| items.iter().all(Value::is_object)),
    // Existing catalog names remain valid aliases of the generic value kinds.
    "string_array" => value
      .as_array()
      .is_some_and(|a| a.iter().all(Value::is_string)),
    "string_table" => value
      .as_object()
      .is_some_and(|o| o.values().all(Value::is_string)),
    "positive_integer" => value.as_u64().is_some_and(|n| n > 0),
    "positive_number" => value.as_f64().is_some_and(|n| n.is_finite() && n > 0.0),
    "nonnegative_integer" => value.as_u64().is_some(),
    "endpoint" => endpoint_matches(value),
    _ => false,
  }
}

fn valid_setting_type(kind: &str) -> bool {
  matches!(
    kind,
    "string"
      | "number"
      | "integer"
      | "boolean"
      | "array"
      | "object"
      | "object_array"
      | "string_array"
      | "string_table"
      | "positive_integer"
      | "positive_number"
      | "nonnegative_integer"
      | "endpoint"
  )
}

fn selector_valid(selector: &Value) -> bool {
  let Some(selector) = selector.as_object() else {
    return false;
  };
  selector.get("provider").is_some_and(nonempty)
    && selector.get("resource").is_some_and(nonempty)
    && selector.get("package_from").is_none_or(nonempty)
    && selector.get("filter").is_none_or(Value::is_object)
    && selector.keys().all(|key| {
      matches!(
        key.as_str(),
        "provider" | "resource" | "package_from" | "filter"
      )
    })
}

pub fn validate_type(node: &Value) -> Result<()> {
  validate_type_idl(node)?;
  let id = node["id"].as_str().unwrap_or("<missing>");
  if let Some(implementation) = node.get("implementation") {
    match implementation["kind"].as_str() {
      Some("built_in") => ensure!(
        implementation.get("abi").is_none(),
        "{id}: built-in type cannot select an ABI"
      ),
      Some("native_plugin") => ensure!(
        node["layer"] == "eos_hardware" && implementation["abi"] == "aether_adapter_v1",
        "{id}: native plugin requires the EOS hardware adapter ABI"
      ),
      Some("managed_process") => ensure!(
        node["layer"] == "process" || node["layer"] == "eos_process",
        "{id}: managed process requires a process owner layer"
      ),
      _ => anyhow::bail!("{id}: implementation kind is unsupported"),
    }
  }
  ensure!(
    node["service_inputs"]
      .as_object()
      .is_none_or(|inputs| inputs.is_empty())
      && node["service_outputs"]
        .as_object()
        .is_none_or(|outputs| outputs.is_empty()),
    "{id}: package service declarations are unavailable in this reader"
  );
  ensure!(
    nonempty(&node["description"]),
    "{id}: description is required"
  );
  for field in ["core", "layer", "group", "kind", "lifecycle"] {
    ensure!(
      nonempty(&node[field]),
      "{id}: {field} must be a nonblank string"
    );
  }
  ensure!(
    node["capabilities"]
      .as_array()
      .is_some_and(|items| items.iter().all(nonempty)),
    "{id}: capabilities must be strings"
  );
  let settings = node["settings"]
    .as_array()
    .ok_or_else(|| anyhow::anyhow!("{id}: settings must be an array"))?;
  let mut names = BTreeSet::new();
  for setting in settings {
    let name = setting["name"].as_str().unwrap_or("");
    let kind = setting["data_type"].as_str().unwrap_or("");
    ensure!(
      !name.is_empty() && names.insert(name),
      "{id}: duplicate or missing setting name"
    );
    ensure!(
      valid_setting_type(kind),
      "{id}.{name}: unsupported setting type"
    );
    ensure!(
      setting["required"].is_boolean(),
      "{id}.{name}: required must be boolean"
    );
    ensure!(
      nonempty(&setting["description"]),
      "{id}.{name}: description is required"
    );
    if let Some(default) = setting.get("default") {
      ensure!(
        value_matches(default, kind),
        "{id}.{name}: default has wrong type"
      );
      if let Some(number) = default.as_f64() {
        ensure!(
          setting["min"].as_f64().is_none_or(|min| number >= min),
          "{id}.{name}: default is below min"
        );
        ensure!(
          setting["max"].as_f64().is_none_or(|max| number <= max),
          "{id}.{name}: default exceeds max"
        );
      }
    }
    if let Some(unit) = setting.get("unit") {
      ensure!(nonempty(unit), "{id}.{name}: unit must be nonblank");
    }
    if let Some(min) = setting.get("min") {
      ensure!(
        min.as_f64().is_some_and(f64::is_finite),
        "{id}.{name}: min must be finite"
      );
    }
    if let Some(max) = setting.get("max") {
      ensure!(
        max.as_f64().is_some_and(f64::is_finite),
        "{id}.{name}: max must be finite"
      );
    }
    if let (Some(min), Some(max)) = (setting["min"].as_f64(), setting["max"].as_f64()) {
      ensure!(min <= max, "{id}.{name}: min exceeds max");
    }
    if let Some(choices) = setting.get("choices") {
      ensure!(
        choices.as_array().is_some_and(
          |values| !values.is_empty() && values.iter().all(|value| value_matches(value, kind))
        ),
        "{id}.{name}: choices have wrong type"
      );
      if let Some(default) = setting.get("default") {
        ensure!(
          choices
            .as_array()
            .is_some_and(|values| values.contains(default)),
          "{id}.{name}: default is not a declared choice"
        );
      }
    }
    if let Some(selector) = setting.get("selector") {
      ensure!(
        selector_valid(selector),
        "{id}.{name}: invalid selector declaration"
      );
    }
  }
  for direction in ["inputs", "outputs"] {
    let ports = node[direction]
      .as_array()
      .ok_or_else(|| anyhow::anyhow!("{id}: {direction} must be an array"))?;
    let mut names = BTreeSet::new();
    for port in ports {
      let name = port["name"].as_str().unwrap_or("");
      ensure!(
        !name.is_empty() && names.insert(name),
        "{id}: duplicate or missing {direction} port"
      );
      ensure!(
        nonempty(&port["data_type"]),
        "{id}.{name}: port data_type required"
      );
      ensure!(
        port["required"].is_boolean(),
        "{id}.{name}: port required must be boolean"
      );
      ensure!(
        nonempty(&port["description"]),
        "{id}.{name}: port description is required"
      );
      let transport = port["transport"].as_str().unwrap_or("in_process");
      ensure!(
        matches!(
          transport,
          "none" | "in_process" | "unix_socket" | "websocket" | "ros2" | "http"
        ),
        "{id}.{name}: invalid port transport"
      );
      let mode = port["delivery_mode"].as_str().unwrap_or("message");
      ensure!(
        matches!(mode, "declaration" | "message" | "event" | "stream"),
        "{id}.{name}: invalid delivery_mode"
      );
      ensure!(
        (transport == "none") == (mode == "declaration"),
        "{id}.{name}: declaration ports require no transport"
      );
      if mode == "declaration" {
        ensure!(
          port.get("contract").is_none(),
          "{id}.{name}: static port cannot declare a live contract"
        );
      } else {
        ensure!(
          port.get("transport").is_some(),
          "{id}.{name}: live port requires an explicit transport"
        );
        let contract = port
          .get("contract")
          .ok_or_else(|| anyhow::anyhow!("{id}.{name}: live port requires a protocol contract"))?;
        ensure!(
          ["owner", "id", "version"]
            .iter()
            .all(|field| contract.get(*field).is_some_and(nonempty)),
          "{id}.{name}: live port has an invalid protocol contract"
        );
      }
      let cardinality = port["cardinality"].as_str().unwrap_or("one");
      ensure!(
        matches!(cardinality, "one" | "many"),
        "{id}.{name}: invalid cardinality"
      );
      if let Some(rate) = port.get("expected_rate_hz") {
        ensure!(
          rate.as_f64().is_some_and(|n| n.is_finite() && n > 0.0),
          "{id}.{name}: expected_rate_hz must be positive"
        );
      }
      if let Some(rate) = port.get("max_rate_hz") {
        ensure!(
          rate.as_f64().is_some_and(|n| n.is_finite() && n > 0.0),
          "{id}.{name}: max_rate_hz must be positive"
        );
      }
    }
  }
  if let Some(bridge) = node.get("bridge") {
    ensure!(
      node["capabilities"]
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item == "bridge")),
      "{id}: bridge declaration requires bridge capability"
    );
    let adapter = bridge["adapter"].as_str().unwrap_or("");
    ensure!(
      matches!(adapter, "http" | "websocket" | "unix_socket" | "ros2"),
      "{id}: bridge adapter is invalid"
    );
    for direction in ["inputs", "outputs"] {
      let names = bridge[direction]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("{id}: bridge {direction} must be an array"))?;
      for name in names {
        ensure!(
          name
            .as_str()
            .is_some_and(|name| node[direction].as_array().is_some_and(|ports| ports
              .iter()
              .any(|port| port["name"] == name && port["transport"] == adapter))),
          "{id}: bridge {direction} refers to an undeclared adapter port"
        );
      }
    }
  }
  Ok(())
}

/// Admit only exact, owner-declared service identities. Endpoint addresses are
/// resolved after the saved graph is fixed and never enter this declaration.
fn validate_service_bindings(node: &Value, definition: &Value) -> Result<()> {
  let Some(bindings) = node.get("service_bindings") else {
    return Ok(());
  };
  let bindings = bindings
    .as_object()
    .ok_or_else(|| anyhow::anyhow!("service_bindings must be an object"))?;
  let inputs = definition["service_inputs"]
    .as_object()
    .ok_or_else(|| anyhow::anyhow!("type declares no service inputs"))?;
  for (name, binding) in bindings {
    let declared = inputs
      .get(name)
      .ok_or_else(|| anyhow::anyhow!("unknown service binding {name}"))?;
    for field in ["owner", "service", "version"] {
      ensure!(
        binding[field] == declared[field] && nonempty(&binding[field]),
        "service binding {name} has wrong {field}"
      );
    }
    ensure!(
      declared["protocol"] == "grpc" && declared["carrier"] == "tcp",
      "service binding {name} uses an unavailable protocol/carrier"
    );
    ensure!(
      node["node_type"] == "isos.ei_io.gateway@v0.2"
        && name == "gateway"
        && binding["owner"] == "eos"
        && binding["service"] == "island.eos.v1.GatewayService"
        && binding["version"] == "1.0.0"
        && binding.get("node").is_none(),
      "service binding {name} is not a supported core service"
    );
    ensure!(
      node["settings"].get(name).is_none(),
      "service binding {name} conflicts with an endpoint setting"
    );
  }
  Ok(())
}

/// Validate only declared settings. Owner-specific settings are never inferred.
pub fn validate_instance_settings(node: &Value, definition: &Value) -> Result<()> {
  if node.get("node_type").is_some() {
    idl_valid(&INSTANCE_SCHEMA, node)?;
  }
  validate_service_bindings(node, definition)?;
  let empty = serde_json::Map::new();
  let settings = node
    .get("settings")
    .map(|settings| {
      settings
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("settings must be an object"))
    })
    .transpose()?
    .unwrap_or(&empty);
  let declarations = definition["settings"]
    .as_array()
    .ok_or_else(|| anyhow::anyhow!("registered settings missing"))?;
  for (name, value) in settings {
    let declaration = declarations
      .iter()
      .find(|item| item["name"] == *name)
      .ok_or_else(|| anyhow::anyhow!("unknown setting {name}"))?;
    let kind = declaration["data_type"].as_str().unwrap_or("");
    ensure!(value_matches(value, kind), "setting {name} must be {kind}");
    ensure!(
      serde_json::to_vec(value)?.len() <= 16_384,
      "setting {name} exceeds 16 KiB"
    );
    if let Some(number) = value.as_f64() {
      if let Some(min) = declaration["min"].as_f64() {
        ensure!(number >= min, "setting {name} is below min");
      }
      if let Some(max) = declaration["max"].as_f64() {
        ensure!(number <= max, "setting {name} exceeds max");
      }
    }
    if let Some(choices) = declaration["choices"].as_array() {
      ensure!(
        choices.contains(value),
        "setting {name} is not a declared choice"
      );
    }
  }
  for declaration in declarations {
    let name = declaration["name"].as_str().unwrap_or("");
    if declaration["required"] == true
      && !settings.contains_key(name)
      && declaration.get("default").is_none()
      && !(definition["service_inputs"].get(name).is_some()
        && node["service_bindings"].get(name).is_some())
    {
      ensure!(false, "required setting {name} is missing");
    }
  }
  if let Some(inputs) = definition["service_inputs"].as_object() {
    for (name, input) in inputs {
      ensure!(
        input["required"] != true
          || node["service_bindings"].get(name).is_some()
          || settings.contains_key(name),
        "required service binding {name} is missing"
      );
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn generic_setting_kinds_and_selector_are_checked() {
    let definition = json!({"id":"x.gate@v0.2","core":"isos","layer":"io","group":"gate","description":"Synthetic gate",
      "kind":"software","lifecycle":"internal","capabilities":["bridge"],
      "settings":[
        {"name":"key","data_type":"string","required":true,"description":"Key", "selector":{"provider":"ui","resource":"keys"}},
        {"name":"speed","data_type":"number","required":true,"description":"Speed"},
        {"name":"enabled","data_type":"boolean","required":false,"description":"Armed"},
        {"name":"axes","data_type":"array","required":false,"description":"Axes"},
        {"name":"metadata","data_type":"object","required":false,"description":"Metadata"}],
      "inputs":[],"outputs":[{"name":"event","data_type":"key_state","transport":"http","delivery_mode":"stream","cardinality":"one","required":true,"description":"Input event","max_rate_hz":20.0,"contract":{"owner":"isos","id":"key-state","version":"1.0"}}]});
    validate_type(&definition).unwrap();
    validate_instance_settings(
      &json!({"settings":{"key":"w","speed":0.4,"enabled":true,"axes":[],"metadata":{}}}),
      &definition,
    )
    .unwrap();
    assert!(validate_instance_settings(&json!({}), &definition).is_err());
    assert!(
      validate_instance_settings(&json!({"settings":{"key":"w","speed":"fast"}}), &definition)
        .is_err()
    );
    assert!(
      validate_instance_settings(
        &json!({"settings":{"key":"w","speed":0.4,"other":1}}),
        &definition
      )
      .is_err()
    );
    let mut broken = definition.clone();
    broken["outputs"][0]["delivery_mode"] = json!("unknown");
    assert!(validate_type(&broken).is_err());
    broken = definition.clone();
    broken["outputs"][0]
      .as_object_mut()
      .unwrap()
      .remove("contract");
    assert!(validate_type(&broken).is_err());
    broken = definition.clone();
    broken["settings"][1]["min"] = json!(0.1);
    broken["settings"][1]["default"] = json!(0.0);
    assert!(validate_type(&broken).is_err());
    broken["settings"][1]["default"] = json!(0.2);
    broken["settings"][1]["choices"] = json!([0.1, 0.3]);
    assert!(validate_type(&broken).is_err());
    broken["settings"][1]["default"] = json!(0.3);
    validate_type(&broken).unwrap();
  }

  #[test]
  fn endpoint_and_positive_number_settings_are_strict() {
    let definition = json!({"settings":[
      {"name":"gateway","data_type":"endpoint","required":true},
      {"name":"timeout","data_type":"positive_number","required":true}
    ]});
    let valid = json!({"settings":{"gateway":{"transport":"tcp","host":"127.0.0.1","port":16200},"timeout":0.5}});
    validate_instance_settings(&valid, &definition).unwrap();
    for gateway in [
      json!({"transport":"tcp","host":"127.0.0.1","port":0}),
      json!({"transport":"tcp","host":"127.0.0.1","port":16200,"path":"/tmp/socket"}),
      json!({"transport":"uds","path":"relative/socket"}),
      json!({"transport":"http","host":"127.0.0.1","port":16200}),
    ] {
      let mut invalid = valid.clone();
      invalid["settings"]["gateway"] = gateway;
      assert!(validate_instance_settings(&invalid, &definition).is_err());
    }
    let mut invalid = valid;
    invalid["settings"]["timeout"] = json!(0);
    assert!(validate_instance_settings(&invalid, &definition).is_err());
  }

  #[test]
  fn logical_eos_service_binding_requires_exact_contract_and_supported_channel() {
    let mut definition = json!({
      "settings":[{"name":"gateway","data_type":"endpoint","required":true}],
      "service_inputs":{"gateway":{"owner":"eos","service":"island.eos.v1.GatewayService",
        "version":"1.0.0","protocol":"grpc","carrier":"tcp","required":true}}
    });
    let node = json!({"id":"link","node_type":"isos.ei_io.gateway@v0.2",
      "references":[],
      "service_bindings":{"gateway":{"owner":"eos","service":"island.eos.v1.GatewayService",
        "version":"1.0.0"}}});
    validate_instance_settings(&node, &definition).unwrap();
    let mut invalid = node.clone();
    invalid["service_bindings"]["gateway"]["version"] = json!("2.0.0");
    assert!(validate_instance_settings(&invalid, &definition).is_err());
    invalid = node.clone();
    invalid["settings"] = json!({"gateway":{"transport":"tcp","host":"127.0.0.1","port":16200}});
    assert!(validate_instance_settings(&invalid, &definition).is_err());
    definition["service_inputs"]["gateway"]["carrier"] = json!("unix_socket");
    assert!(validate_instance_settings(&node, &definition).is_err());
  }

  #[test]
  fn object_array_setting_accepts_objects_and_rejects_other_items() {
    let definition = json!({"settings":[
      {"name":"skill_executables","data_type":"object_array","required":false,
       "description":"Selected skill executables"}
    ]});
    validate_instance_settings(
      &json!({"settings":{"skill_executables":[{"package":{"id":"test/skill"}}]}}),
      &definition,
    )
    .unwrap();
    assert!(
      validate_instance_settings(
        &json!({"settings":{"skill_executables":["test/skill"]}}),
        &definition,
      )
      .is_err()
    );
  }
}
