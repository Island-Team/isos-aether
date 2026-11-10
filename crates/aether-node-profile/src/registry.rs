use std::{
  collections::{BTreeMap, BTreeSet},
  sync::LazyLock,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{valid_segment, NodeKind, NodeProfile, PackageRef, ProfileError};

static TYPE_IDL: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
  let source: serde_json::Value = serde_json::from_str(include_str!(
    "../../../share/isos/contracts/node/node-type.schema.json"
  ))
  .expect("embedded node type IDL is JSON");
  jsonschema::validator_for(&source).expect("embedded node type IDL is valid")
});

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
  Internal,
  Process,
  Adapter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortDataType {
  CameraFrame,
  BaseObservation,
  ManipulatorObservation,
  BaseMovement,
  ManipulatorMovement,
  ControlRequest,
  CommandOutcome,
  SkillOutcome,
  Health,
  Status,
  ObservationRef,
  ContextSnapshot,
  PlanProposal,
  VisionFrame,
  Custom(String),
}

impl PortDataType {
  pub fn id(&self) -> &str {
    match self {
      Self::CameraFrame => "camera_frame",
      Self::BaseObservation => "base_observation",
      Self::ManipulatorObservation => "manipulator_observation",
      Self::BaseMovement => "base_movement",
      Self::ManipulatorMovement => "manipulator_movement",
      Self::ControlRequest => "control_request",
      Self::CommandOutcome => "command_outcome",
      Self::SkillOutcome => "skill_outcome",
      Self::Health => "health",
      Self::Status => "status",
      Self::ObservationRef => "observation_ref",
      Self::ContextSnapshot => "context_snapshot",
      Self::PlanProposal => "plan_proposal",
      Self::VisionFrame => "vision_frame",
      Self::Custom(id) => id,
    }
  }
}

impl Serialize for PortDataType {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.id())
  }
}

impl<'de> Deserialize<'de> for PortDataType {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let id = String::deserialize(deserializer)?;
    Ok(match id.as_str() {
      "camera_frame" => Self::CameraFrame,
      "base_observation" => Self::BaseObservation,
      "manipulator_observation" => Self::ManipulatorObservation,
      "base_movement" => Self::BaseMovement,
      "manipulator_movement" => Self::ManipulatorMovement,
      "control_request" => Self::ControlRequest,
      "command_outcome" => Self::CommandOutcome,
      "skill_outcome" => Self::SkillOutcome,
      "health" => Self::Health,
      "status" => Self::Status,
      "observation_ref" => Self::ObservationRef,
      "context_snapshot" => Self::ContextSnapshot,
      "plan_proposal" => Self::PlanProposal,
      "vision_frame" => Self::VisionFrame,
      _ => Self::Custom(id),
    })
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortTransport {
  None,
  InProcess,
  UnixSocket,
  Http,
  Websocket,
  Ros2,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
  Declaration,
  #[default]
  Message,
  Event,
  Stream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataClass {
  Image,
  Observation,
  Command,
  Event,
  State,
  Opaque,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortCardinality {
  #[default]
  One,
  Many,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
  pub name: String,
  pub data_type: PortDataType,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub contract: Option<PortContractRef>,
  pub required: bool,
  #[serde(default)]
  pub cardinality: PortCardinality,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub expected_rate_hz: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub max_rate_hz: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub transport: Option<PortTransport>,
  #[serde(default)]
  pub delivery_mode: DeliveryMode,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub data_class: Option<DataClass>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortContractRef {
  pub owner: String,
  pub id: String,
  pub version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataTypeDescriptor {
  pub id: String,
  pub version: String,
  pub purpose: String,
  pub media_type: String,
  pub codec: String,
  pub schema: serde_json::Value,
  pub max_payload_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingType {
  String,
  PositiveInteger,
  NonnegativeInteger,
  StringArray,
  StringTable,
  ObjectArray,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Setting {
  pub name: String,
  pub data_type: SettingType,
  pub required: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub default: Option<toml::Value>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unit: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub min: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub max: Option<i64>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub choices: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub selector: Option<SettingSelector>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub items_schema: Option<serde_json::Value>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub min_items: Option<usize>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub max_items: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectorProvider {
  Package,
  Registry,
  Graph,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum PackageSource {
  #[serde(rename = "node.package")]
  NodePackage,
  #[serde(rename = "node.adapter.package")]
  NodeAdapterPackage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SettingSelector {
  pub provider: SelectorProvider,
  pub resource: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub package_from: Option<PackageSource>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeType {
  pub id: String,
  /// Operator-facing type description.
  pub description: String,
  pub core: String,
  pub layer: String,
  pub group: String,
  pub kind: NodeKind,
  pub lifecycle: Lifecycle,
  pub capabilities: Vec<String>,
  pub settings: Vec<Setting>,
  pub inputs: Vec<Port>,
  pub outputs: Vec<Port>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub implementation: Option<ImplementationDescriptor>,
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub service_inputs: BTreeMap<String, ServiceDescriptor>,
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub service_outputs: BTreeMap<String, ServiceDescriptor>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bridge: Option<BridgeDeclaration>,
  /// Required for registered actuator types beyond the built-in EOS base and arm.
  #[serde(default)]
  pub safety_adapter_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImplementationDescriptor {
  pub kind: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub abi: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDescriptor {
  pub owner: String,
  pub service: String,
  pub version: String,
  pub protocol: String,
  pub carrier: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub socket_protocol: Option<String>,
  #[serde(default)]
  pub required: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeDeclaration {
  pub adapter: String,
  pub inputs: Vec<String>,
  pub outputs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyAdapterType {
  pub id: String,
  pub kind: NodeKind,
  pub movement_modes: Vec<String>,
  pub supports_stop: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterFamilyDescriptor {
  pub id: String,
  pub device_kinds: Vec<NodeKind>,
  pub transport: PortTransport,
  pub description: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub settings_schema: Option<serde_json::Value>,
  #[serde(default)]
  pub supported_primitives: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CatalogAdapterFamily {
  #[serde(flatten)]
  pub descriptor: AdapterFamilyDescriptor,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub package: Option<PackageRef>,
  pub compiled: bool,
  /// Package families require a verified shared library at EOS launch.
  pub loadable: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PrimitiveSpec {
  pub name: &'static str,
  pub version: &'static str,
  pub device_class: &'static str,
  pub movement_type: &'static str,
  pub description: &'static str,
}

pub const PRIMITIVE_SPECS: &[PrimitiveSpec] = &[
  PrimitiveSpec {
    name: "base.velocity",
    version: "1.0.0",
    device_class: "base",
    movement_type: "body_velocity",
    description: "Body-frame base velocity",
  },
  PrimitiveSpec {
    name: "arm.delta",
    version: "1.0.0",
    device_class: "manipulator",
    movement_type: "joint_positions",
    description: "Six ordered relative joint angles in radians",
  },
  PrimitiveSpec {
    name: "arm.movej",
    version: "1.0.0",
    device_class: "manipulator",
    movement_type: "joint_positions",
    description: "Full joint-position target",
  },
];

#[derive(Clone, Debug, Default)]
pub struct NodeTypeRegistry {
  types: BTreeMap<String, NodeType>,
  safety_adapters: BTreeMap<String, SafetyAdapterType>,
  packages: BTreeMap<String, PackageRef>,
  data_types: BTreeMap<String, DataTypeDescriptor>,
  data_packages: BTreeMap<String, PackageRef>,
  codecs: BTreeSet<String>,
  adapter_families: BTreeMap<String, AdapterFamilyDescriptor>,
  family_packages: BTreeMap<String, PackageRef>,
}

/// Verified package contribution passed by IsOS, separate from graph instances.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeTypeContribution {
  pub package: PackageRef,
  #[serde(default)]
  pub node_types: Vec<NodeType>,
  #[serde(default)]
  pub safety_adapters: Vec<SafetyAdapterType>,
  #[serde(default)]
  pub data_types: Vec<DataTypeDescriptor>,
  #[serde(default)]
  pub adapter_families: Vec<AdapterFamilyDescriptor>,
}

impl NodeTypeRegistry {
  pub fn with_contribution_files(paths: &[std::path::PathBuf]) -> Result<Self, ProfileError> {
    let mut registry = Self::builtin();
    for path in paths {
      let bytes =
        std::fs::read(path).map_err(|e| ProfileError(format!("{}: {e}", path.display())))?;
      if bytes.len() > 1024 * 1024 {
        return Err(ProfileError(format!(
          "{}: node type contribution exceeds 1 MiB",
          path.display()
        )));
      }
      let contribution: NodeTypeContribution = serde_json::from_slice(&bytes)
        .map_err(|e| ProfileError(format!("{}: {e}", path.display())))?;
      registry.register_contribution(contribution)?;
    }
    Ok(registry)
  }
  pub fn builtin() -> Self {
    let mut registry = Self::default();
    registry.codecs.insert("json_v1".into());
    for id in [
      "button_activation@v0.2",
      "arm_joint_delta@v0.2",
      "arm_monitor_state@v0.2",
    ] {
      registry
        .register_builtin_data_type(DataTypeDescriptor {
          id: id.into(),
          version: "0.2".into(),
          purpose: builtin_data_purpose(id).into(),
          media_type: "application/json".into(),
          codec: "json_v1".into(),
          schema: builtin_data_schema(id),
          max_payload_bytes: 1024,
        })
        .expect("valid built-in data type");
    }
    registry
      .register_safety_adapter(SafetyAdapterType {
        id: "mock_arm".into(),
        kind: NodeKind::Manipulator,
        movement_modes: vec!["joint_positions".into()],
        supports_stop: true,
      })
      .expect("valid built-in safety adapter");
    registry.adapter_families.insert(
      "mock-arm".into(),
      AdapterFamilyDescriptor {
        id: "mock-arm".into(),
        device_kinds: vec![NodeKind::Manipulator],
        transport: PortTransport::InProcess,
        description: "Built-in six-joint mock arm".into(),
        settings_schema: None,
        supported_primitives: vec!["arm.delta@1.0.0".into()],
      },
    );
    for definition in builtin_types() {
      registry
        .register_builtin(definition)
        .expect("valid built-in node type");
    }
    registry
  }

  pub fn get(&self, id: &str) -> Option<&NodeType> {
    self.types.get(id)
  }

  pub fn all(&self) -> impl Iterator<Item = &NodeType> {
    self.types.values()
  }

  pub fn definitions(&self) -> impl Iterator<Item = &NodeType> {
    self.all()
  }

  pub fn package_for_type(&self, id: &str) -> Option<&PackageRef> {
    self.packages.get(id)
  }

  pub fn data_type(&self, id: &str) -> Option<&DataTypeDescriptor> {
    self.data_types.get(id)
  }

  pub fn data_types(&self) -> impl Iterator<Item = &DataTypeDescriptor> {
    self.data_types.values()
  }

  pub fn package_for_data_type(&self, id: &str) -> Option<&PackageRef> {
    self.data_packages.get(id)
  }

  pub fn adapter_family(&self, id: &str) -> Option<&AdapterFamilyDescriptor> {
    self.adapter_families.get(id)
  }

  pub fn package_for_adapter_family(&self, id: &str) -> Option<&PackageRef> {
    self.family_packages.get(id)
  }

  pub fn catalog_adapter_families(&self) -> Vec<CatalogAdapterFamily> {
    self
      .adapter_families
      .values()
      .map(|descriptor| {
        let package = self.family_packages.get(&descriptor.id).cloned();
        let compiled = matches!(
          (
            descriptor.id.as_str(),
            package
              .as_ref()
              .map(|p| (p.id.as_str(), p.version.as_str()))
          ),
          ("mock-cxx", None)
            | ("mock-arm", None)
            | ("mujoco-v2", Some(("island/mujoco-sim", "0.1.1")))
        );
        let loadable = !compiled
          && descriptor.transport == PortTransport::InProcess
          && self.family_packages.contains_key(&descriptor.id);
        CatalogAdapterFamily {
          descriptor: descriptor.clone(),
          package,
          compiled,
          loadable,
          reason: if loadable {
            Some("requires verified adapter library".into())
          } else if !compiled {
            Some("no EOS adapter implementation".into())
          } else {
            None
          },
        }
      })
      .collect()
  }

  pub fn register_codec(&mut self, id: &str) -> Result<(), ProfileError> {
    if !valid_segment(id) {
      return Err(ProfileError(format!("invalid codec ID: {id}")));
    }
    self.codecs.insert(id.into());
    Ok(())
  }

  pub fn register_contribution(
    &mut self,
    contribution: NodeTypeContribution,
  ) -> Result<(), ProfileError> {
    contribution.package.validate()?;
    let mut next = self.clone();
    for descriptor in contribution.data_types {
      let id = descriptor.id.clone();
      next.register_data_type(descriptor)?;
      next.data_packages.insert(id, contribution.package.clone());
    }
    for family in contribution.adapter_families {
      let mut kinds = BTreeSet::new();
      let mut primitives = BTreeSet::new();
      if !valid_segment(&family.id)
        || family.device_kinds.is_empty()
        || family.device_kinds.iter().any(|kind| {
          !matches!(
            kind,
            NodeKind::Camera | NodeKind::Base | NodeKind::Manipulator
          ) || !kinds.insert(*kind)
        })
        || family.description.trim().is_empty()
        || family
          .settings_schema
          .as_ref()
          .is_some_and(|schema| !schema.is_object())
        || family.supported_primitives.iter().any(|primitive| {
          !primitives.insert(primitive)
            || !PRIMITIVE_SPECS
              .iter()
              .any(|spec| format!("{}@{}", spec.name, spec.version) == *primitive)
        })
        || next.adapter_families.contains_key(&family.id)
      {
        return Err(ProfileError(format!(
          "invalid or duplicate adapter family: {}",
          family.id
        )));
      }
      next
        .family_packages
        .insert(family.id.clone(), contribution.package.clone());
      next.adapter_families.insert(family.id.clone(), family);
    }
    for adapter in contribution.safety_adapters {
      next.register_safety_adapter(adapter)?;
    }
    for definition in contribution.node_types {
      if definition.id == "isos.eos_process.mujoco@v0.2"
        && (contribution.package.id != "island/mujoco-sim"
          || contribution.package.version != "0.1.1"
          || definition.core != "isos"
          || definition.layer != "eos_process"
          || definition.group != "mujoco"
          || definition.kind != NodeKind::Software
          || definition.lifecycle != Lifecycle::Process)
      {
        return Err(ProfileError(format!(
          "unsupported MuJoCo type definition: {}",
          definition.id
        )));
      }
      if !definition.id.starts_with("isos.") || !definition.id.ends_with("@v0.2") {
        return Err(ProfileError(format!(
          "package node type must use the current graph contract: {}",
          definition.id
        )));
      }
      let id = definition.id.clone();
      next.register(definition)?;
      next.packages.insert(id, contribution.package.clone());
    }
    *self = next;
    Ok(())
  }

  pub fn register(&mut self, definition: NodeType) -> Result<(), ProfileError> {
    for port in definition.inputs.iter().chain(&definition.outputs) {
      if (port.delivery_mode == DeliveryMode::Declaration)
        != (port.transport == Some(PortTransport::None))
      {
        return Err(ProfileError(format!(
          "node type {} port {} has inconsistent static transport",
          definition.id, port.name
        )));
      }
      if port.delivery_mode == DeliveryMode::Declaration {
        if port.contract.is_some() {
          return Err(ProfileError(format!(
            "node type {} static port {} has a live contract",
            definition.id, port.name
          )));
        }
      } else if !port.contract.as_ref().is_some_and(|contract| {
        !contract.owner.trim().is_empty()
          && !contract.id.trim().is_empty()
          && !contract.version.trim().is_empty()
      }) {
        return Err(ProfileError(format!(
          "node type {} live port {} requires a protocol contract",
          definition.id, port.name
        )));
      }
      let descriptor = self.data_types.get(port.data_type.id()).ok_or_else(|| {
        ProfileError(format!(
          "node type {} uses unknown data type {}",
          definition.id,
          port.data_type.id()
        ))
      })?;
      if !self.codecs.contains(&descriptor.codec) {
        return Err(ProfileError(format!(
          "node type {} requires uncompiled codec {}",
          definition.id, descriptor.codec
        )));
      }
    }
    if matches!(definition.kind, NodeKind::Base | NodeKind::Manipulator) {
      let safety = definition
        .safety_adapter_type
        .as_deref()
        .and_then(|id| self.safety_adapters.get(id))
        .ok_or_else(|| {
          ProfileError(format!(
            "custom actuator type {} requires a registered safety adapter",
            definition.id
          ))
        })?;
      if safety.kind != definition.kind
        || !safety.supports_stop
        || !definition
          .capabilities
          .iter()
          .all(|mode| safety.movement_modes.contains(mode))
      {
        return Err(ProfileError(format!(
          "custom actuator type {} has incompatible safety adapter",
          definition.id
        )));
      }
    }
    self.register_builtin(definition)
  }

  pub fn register_data_type(&mut self, descriptor: DataTypeDescriptor) -> Result<(), ProfileError> {
    if !valid_contributed_data_id(&descriptor.id)
      || !descriptor
        .id
        .ends_with(&format!("@v{}", descriptor.version))
    {
      return Err(ProfileError(format!(
        "invalid contributed data type ID: {}",
        descriptor.id
      )));
    }
    self.register_builtin_data_type(descriptor)
  }

  fn register_builtin_data_type(
    &mut self,
    descriptor: DataTypeDescriptor,
  ) -> Result<(), ProfileError> {
    if descriptor.media_type.trim().is_empty()
      || descriptor.version.trim().is_empty()
      || descriptor.purpose.trim().is_empty()
      || descriptor.codec.trim().is_empty()
      || descriptor.max_payload_bytes == 0
      || !descriptor.schema.is_object()
      || jsonschema::validator_for(&descriptor.schema).is_err()
      || self.data_types.contains_key(&descriptor.id)
    {
      return Err(ProfileError(format!(
        "invalid or duplicate data type: {}",
        descriptor.id
      )));
    }
    self.data_types.insert(descriptor.id.clone(), descriptor);
    Ok(())
  }

  pub fn register_safety_adapter(
    &mut self,
    definition: SafetyAdapterType,
  ) -> Result<(), ProfileError> {
    if !valid_segment(&definition.id)
      || !matches!(definition.kind, NodeKind::Base | NodeKind::Manipulator)
      || !definition.supports_stop
      || definition.movement_modes.is_empty()
      || !definition
        .movement_modes
        .iter()
        .all(|mode| valid_segment(mode))
    {
      return Err(ProfileError(format!(
        "invalid safety adapter type: {}",
        definition.id
      )));
    }
    if self.safety_adapters.contains_key(&definition.id) {
      return Err(ProfileError(format!(
        "duplicate safety adapter type: {}",
        definition.id
      )));
    }
    self
      .safety_adapters
      .insert(definition.id.clone(), definition);
    Ok(())
  }

  fn register_builtin(&mut self, definition: NodeType) -> Result<(), ProfileError> {
    let value = serde_json::to_value(&definition)
      .map_err(|error| ProfileError(format!("node type serialization failed: {error}")))?;
    if let Err(error) = TYPE_IDL.validate(&value) {
      return Err(ProfileError(format!(
        "node type {} violates shared IDL: {error}",
        definition.id
      )));
    }
    if !matches!(definition.core.as_str(), "isos" | "eos")
      || !definition.id.starts_with(&format!("{}.", definition.core))
      || !definition.id.ends_with("@v0.2")
    {
      return Err(ProfileError(format!(
        "node type must use the current graph contract: {}",
        definition.id
      )));
    }
    if definition.description.trim().is_empty() {
      return Err(ProfileError(format!(
        "node type {} requires a nonblank description",
        definition.id
      )));
    }
    let expected_prefix = if definition.core == "isos" {
      format!("{}.{}.", definition.core, definition.layer)
    } else {
      format!(
        "{}.{}.{}.",
        definition.core, definition.layer, definition.group
      )
    };
    let type_name = definition
      .id
      .strip_prefix(&expected_prefix)
      .and_then(|name| name.split_once('@'));
    if !type_name.is_some_and(|(name, version)| valid_segment(name) && valid_type_version(version))
      || ![&definition.core, &definition.layer, &definition.group]
        .iter()
        .all(|s| valid_segment(s))
      || definition
        .inputs
        .iter()
        .chain(&definition.outputs)
        .any(|p| {
          !valid_segment(&p.name)
            || p.description.as_ref().is_some_and(|d| d.trim().is_empty())
            || p
              .expected_rate_hz
              .is_some_and(|r| !r.is_finite() || r <= 0.0)
            || p.max_rate_hz.is_some_and(|r| !r.is_finite() || r <= 0.0)
            || !self.data_types.contains_key(p.data_type.id())
            || p.description.is_none()
            || p.transport.is_none()
            || p.data_class.is_none()
        })
      || definition.settings.iter().any(|s| !valid_segment(&s.name))
      || definition.capabilities.iter().any(|c| !valid_segment(c))
      || definition.bridge.as_ref().is_some_and(|bridge| {
        !valid_segment(&bridge.adapter)
          || !definition
            .capabilities
            .iter()
            .any(|capability| capability == "bridge")
          || bridge
            .inputs
            .iter()
            .any(|name| !definition.inputs.iter().any(|port| &port.name == name))
          || bridge
            .outputs
            .iter()
            .any(|name| !definition.outputs.iter().any(|port| &port.name == name))
      })
      || (definition.kind != NodeKind::Software
        && !matches!(
          (
            definition.core.as_str(),
            definition.layer.as_str(),
            definition.lifecycle
          ),
          ("isos", "eos_hardware", Lifecycle::Adapter) | ("eos", "hardware", Lifecycle::Adapter)
        ))
      || (definition.core == "isos"
        && definition.layer == "eos_hardware"
        && definition.kind == NodeKind::Software)
      || (definition.kind == NodeKind::Software && definition.lifecycle == Lifecycle::Adapter)
    {
      return Err(ProfileError(format!(
        "invalid node type definition: {}",
        definition.id
      )));
    }
    let mut names = BTreeSet::new();
    if !definition.inputs.iter().all(|p| names.insert(&p.name)) {
      return Err(ProfileError(format!(
        "duplicate input on {}",
        definition.id
      )));
    }
    names.clear();
    if !definition.outputs.iter().all(|p| names.insert(&p.name)) {
      return Err(ProfileError(format!(
        "duplicate output on {}",
        definition.id
      )));
    }
    names.clear();
    if !definition.settings.iter().all(|s| names.insert(&s.name)) {
      return Err(ProfileError(format!(
        "duplicate setting on {}",
        definition.id
      )));
    }
    for setting in &definition.settings {
      if !valid_setting_descriptor(setting) {
        return Err(ProfileError(format!(
          "invalid setting {} on {}",
          setting.name, definition.id
        )));
      }
    }
    if self.types.contains_key(&definition.id) {
      return Err(ProfileError("duplicate node type registration".into()));
    }
    self.types.insert(definition.id.clone(), definition);
    Ok(())
  }

  pub(crate) fn validate_instance(
    &self,
    node: &NodeProfile,
    definition: &NodeType,
  ) -> Result<(), ProfileError> {
    for key in node.settings.keys() {
      if !definition.settings.iter().any(|s| &s.name == key) {
        return Err(ProfileError(format!(
          "node {} has unknown setting {key}",
          node.id
        )));
      }
    }
    for setting in &definition.settings {
      match node.settings.get(&setting.name) {
        None if setting.required => {
          return Err(ProfileError(format!(
            "node {} requires setting {}",
            node.id, setting.name
          )))
        }
        None => continue,
        Some(value)
          if matches!(setting.name.as_str(), "depends_on" | "args")
            && value.as_array().is_some_and(Vec::is_empty) => {}
        Some(value)
          if valid_setting(value, setting.data_type) && valid_setting_value(value, setting) => {}
        Some(_) => {
          return Err(ProfileError(format!(
            "node {} setting {} has invalid type or value",
            node.id, setting.name
          )))
        }
      }
    }
    if !node
      .capabilities
      .iter()
      .all(|c| definition.capabilities.contains(c))
    {
      return Err(ProfileError(format!(
        "node {} advertises capability outside its type",
        node.id
      )));
    }
    if is_eos_backend_type(&definition.id) {
      if let Some(path) = node.settings.get("socket").and_then(toml::Value::as_str) {
        if !path.starts_with('/') || path.len() >= 108 {
          return Err(ProfileError(format!(
            "node {} has invalid socket path",
            node.id
          )));
        }
      }
    }
    if matches!(definition.id.as_str(), "isos.eos_process.mujoco@v0.2")
      && node
        .package
        .as_ref()
        .is_none_or(|package| package.id != "island/mujoco-sim" || package.version != "0.1.1")
    {
      return Err(ProfileError(format!(
        "node {} requires island/mujoco-sim@0.1.1",
        node.id
      )));
    }
    if matches!(definition.id.as_str(), "isos.eos_process.mujoco@v0.2")
      && (node
        .settings
        .get("scene")
        .and_then(toml::Value::as_str)
        .is_none()
        || node
          .settings
          .get("socket")
          .and_then(toml::Value::as_str)
          .is_none())
    {
      return Err(ProfileError(format!(
        "node {} requires scene and socket",
        node.id
      )));
    }
    Ok(())
  }
}

pub fn is_eos_backend_type(id: &str) -> bool {
  id == "isos.eos_process.mujoco@v0.2"
}

fn valid_type_version(version: &str) -> bool {
  let Some((major, minor)) = version.strip_prefix('v').and_then(|v| v.split_once('.')) else {
    return false;
  };
  !major.is_empty()
    && !minor.is_empty()
    && major.bytes().all(|b| b.is_ascii_digit())
    && minor.bytes().all(|b| b.is_ascii_digit())
}

fn valid_contributed_data_id(id: &str) -> bool {
  let Some((name, version)) = id.rsplit_once('@') else {
    return false;
  };
  let mut segments = name.split('.');
  segments.clone().count() >= 2 && segments.all(valid_segment) && valid_type_version(version)
}

fn valid_setting(value: &toml::Value, kind: SettingType) -> bool {
  match kind {
    SettingType::String => value.as_str().is_some_and(|s| !s.trim().is_empty()),
    SettingType::PositiveInteger => value.as_integer().is_some_and(|n| n > 0),
    SettingType::NonnegativeInteger => value.as_integer().is_some_and(|n| n >= 0),
    SettingType::StringArray => value.as_array().is_some_and(|values| {
      !values.is_empty()
        && values
          .iter()
          .all(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))
    }),
    SettingType::StringTable => value.as_table().is_some_and(|table| {
      table
        .iter()
        .all(|(k, v)| valid_segment(k) && v.as_str().is_some())
    }),
    SettingType::ObjectArray => value
      .as_array()
      .is_some_and(|values| values.iter().all(|value| value.as_table().is_some())),
  }
}

fn valid_item_schema(schema: &serde_json::Value, depth: usize) -> bool {
  let Some(object) = schema.as_object() else {
    return false;
  };
  let kind = schema.get("type").and_then(serde_json::Value::as_str);
  if depth > 3 || !matches!(kind, Some("object" | "string")) {
    return false;
  }
  let allowed = [
    "type",
    "properties",
    "required",
    "additionalProperties",
    "minLength",
    "maxLength",
    "pattern",
  ];
  if object.keys().any(|key| !allowed.contains(&key.as_str())) {
    return false;
  }
  match kind {
    Some("object") => {
      if object.contains_key("minLength")
        || object.contains_key("maxLength")
        || object.contains_key("pattern")
      {
        return false;
      }
      let Some(properties) = object
        .get("properties")
        .and_then(serde_json::Value::as_object)
      else {
        return false;
      };
      if properties.len() > 64
        || properties
          .iter()
          .any(|(name, child)| name.is_empty() || !valid_item_schema(child, depth + 1))
      {
        return false;
      }
      if object
        .get("additionalProperties")
        .is_some_and(|value| !value.is_boolean())
      {
        return false;
      }
      let Some(required) = object.get("required") else {
        return true;
      };
      let Some(required) = required.as_array() else {
        return false;
      };
      let mut seen = BTreeSet::new();
      required.iter().all(|value| {
        value
          .as_str()
          .is_some_and(|name| properties.contains_key(name) && seen.insert(name))
      })
    }
    Some("string") => {
      if object.contains_key("properties")
        || object.contains_key("required")
        || object.contains_key("additionalProperties")
      {
        return false;
      }
      let min = object
        .get("minLength")
        .map(serde_json::Value::as_u64)
        .unwrap_or(Some(0));
      let max = object
        .get("maxLength")
        .map(serde_json::Value::as_u64)
        .unwrap_or(Some(4096));
      let Some((min, max)) = min.zip(max) else {
        return false;
      };
      if min > max || max > 4096 {
        return false;
      }
      object.get("pattern").is_none_or(|value| {
        value
          .as_str()
          .is_some_and(|pattern| pattern.len() <= 256 && regex::Regex::new(pattern).is_ok())
      })
    }
    _ => false,
  }
}

fn item_matches_schema(value: &serde_json::Value, schema: &serde_json::Value) -> bool {
  match schema["type"].as_str() {
    Some("object") => {
      let Some(object) = value.as_object() else {
        return false;
      };
      let properties = schema["properties"].as_object().expect("validated schema");
      if schema["required"].as_array().is_some_and(|required| {
        required
          .iter()
          .any(|key| key.as_str().is_none_or(|name| !object.contains_key(name)))
      }) {
        return false;
      }
      object.iter().all(|(name, child)| {
        properties.get(name).map_or(
          schema["additionalProperties"].as_bool() != Some(false),
          |child_schema| item_matches_schema(child, child_schema),
        )
      })
    }
    Some("string") => value.as_str().is_some_and(|text| {
      let length = text.chars().count() as u64;
      length >= schema["minLength"].as_u64().unwrap_or(0)
        && length <= schema["maxLength"].as_u64().unwrap_or(4096)
        && schema["pattern"].as_str().is_none_or(|pattern| {
          regex::Regex::new(pattern).is_ok_and(|pattern| pattern.is_match(text))
        })
    }),
    _ => false,
  }
}

fn valid_setting_descriptor(setting: &Setting) -> bool {
  if setting.default.as_ref().is_some_and(|value| {
    !valid_setting(value, setting.data_type) || !valid_setting_value(value, setting)
  }) {
    return false;
  }
  if setting
    .description
    .as_ref()
    .is_some_and(|value| value.trim().is_empty())
    || setting.description.is_none()
    || setting
      .unit
      .as_ref()
      .is_some_and(|value| value.trim().is_empty())
  {
    return false;
  }
  let numeric = matches!(
    setting.data_type,
    SettingType::PositiveInteger | SettingType::NonnegativeInteger
  );
  if (setting.min.is_some() || setting.max.is_some()) && !numeric {
    return false;
  }
  let object_array = setting.data_type == SettingType::ObjectArray;
  if object_array {
    if setting.items_schema.as_ref().is_none_or(|schema| {
      serde_json::to_vec(schema).map_or(true, |bytes| bytes.len() > 8192)
        || schema["type"].as_str() != Some("object")
        || !valid_item_schema(schema, 0)
    }) || setting
      .min_items
      .zip(setting.max_items)
      .is_some_and(|(min, max)| min > max)
      || setting.max_items.is_none_or(|max| max > 256)
    {
      return false;
    }
  } else if setting.items_schema.is_some()
    || setting.min_items.is_some()
    || setting.max_items.is_some()
  {
    return false;
  }
  if setting
    .min
    .into_iter()
    .chain(setting.max)
    .any(|value| value < 0)
    || setting
      .min
      .zip(setting.max)
      .is_some_and(|(min, max)| min > max)
  {
    return false;
  }
  let mut choices = BTreeSet::new();
  if !setting.choices.is_empty()
    && (!matches!(setting.data_type, SettingType::String)
      || setting.selector.is_some()
      || setting
        .choices
        .iter()
        .any(|value| value.trim().is_empty() || !choices.insert(value)))
  {
    return false;
  }
  setting.selector.as_ref().is_none_or(|selector| {
    matches!(setting.data_type, SettingType::String)
      && matches!(
        (
          selector.provider,
          selector.resource.as_str(),
          selector.package_from
        ),
        (SelectorProvider::Package, "scene" | "device", Some(_))
          | (SelectorProvider::Registry, "adapter_family", None)
          | (SelectorProvider::Graph, "node", None)
      )
  })
}

fn valid_setting_value(value: &toml::Value, setting: &Setting) -> bool {
  if setting.data_type == SettingType::ObjectArray {
    let Some(items) = value.as_array() else {
      return false;
    };
    return setting.min_items.is_none_or(|min| items.len() >= min)
      && setting.max_items.is_none_or(|max| items.len() <= max)
      && items.iter().all(|item| {
        serde_json::to_value(item).is_ok_and(|json| {
          setting
            .items_schema
            .as_ref()
            .is_some_and(|schema| item_matches_schema(&json, schema))
        })
      });
  }
  if let Some(integer) = value.as_integer() {
    if setting.min.is_some_and(|min| integer < min) || setting.max.is_some_and(|max| integer > max)
    {
      return false;
    }
  }
  setting.choices.is_empty()
    || value
      .as_str()
      .is_some_and(|selected| setting.choices.iter().any(|choice| choice == selected))
}

fn port(name: &str, data_type: PortDataType, required: bool) -> Port {
  Port {
    name: name.into(),
    data_type,
    contract: None,
    required,
    cardinality: PortCardinality::One,
    description: None,
    expected_rate_hz: None,
    max_rate_hz: None,
    transport: None,
    delivery_mode: DeliveryMode::Message,
    data_class: None,
  }
}

fn setting(name: &str, data_type: SettingType, required: bool) -> Setting {
  Setting {
    name: name.into(),
    data_type,
    required,
    default: None,
    description: None,
    unit: None,
    min: None,
    max: None,
    choices: Vec::new(),
    selector: None,
    items_schema: None,
    min_items: None,
    max_items: None,
  }
}

fn node_type(
  id: &str,
  placement: (&str, &str, &str),
  kind: NodeKind,
  lifecycle: Lifecycle,
  capabilities: &[&str],
  ports: (Vec<Port>, Vec<Port>),
) -> NodeType {
  let (core, layer, group) = placement;
  let (inputs, outputs) = ports;
  NodeType {
        id: id.into(),
        description: match id {
            "isos.eos_hardware.camera@v0.2" => "Captures camera frames and publishes health and status.",
            "isos.eos_hardware.base@v0.2" => "Receives admitted base commands and publishes pose, outcome, health, and status.",
            "isos.eos_hardware.manipulator@v0.2" => "Receives admitted arm commands and publishes joint observations, outcome, health, and status.",
            "isos.eos_dispatcher.control@v0.2" => "Admits control requests and emits typed base or arm commands and status.",
            "isos.eos_dispatcher.primitive_base@v0.2" => "Binds a declared base primitive to a typed base command output.",
            "isos.eos_dispatcher.primitive_manipulator@v0.2" => "Binds a declared arm primitive to a typed arm command output.",
            "isos.eos_interface.camera_gate@v0.2" => "Gates camera frames into a read-only vision stream.",
            _ => "",
        }.into(),
        core: core.into(),
        layer: layer.into(),
        group: group.into(),
        kind,
        lifecycle,
        capabilities: capabilities.iter().map(|c| (*c).into()).collect(),
        settings: Vec::new(),
        inputs,
        outputs,
        implementation: None,
        service_inputs: BTreeMap::new(),
        service_outputs: BTreeMap::new(),
        bridge: None,
        safety_adapter_type: None,
    }
}

fn builtin_types() -> Vec<NodeType> {
  fn live(
    name: &str,
    data: &str,
    transport: PortTransport,
    owner: &str,
    contract_id: &str,
    class: DataClass,
  ) -> Port {
    let mut value = port(name, PortDataType::Custom(data.into()), true);
    value.description = Some(format!("{data} delivery"));
    value.transport = Some(transport);
    value.delivery_mode = DeliveryMode::Event;
    value.data_class = Some(class);
    value.contract = Some(PortContractRef {
      owner: owner.into(),
      id: contract_id.into(),
      version: "1".into(),
    });
    value
  }
  let activation = "button_activation@v0.2";
  let delta = "arm_joint_delta@v0.2";
  let monitor = "arm_monitor_state@v0.2";
  let mut ingress = node_type(
    "eos.interface.ingress.trigger@v0.2",
    ("eos", "interface", "ingress"),
    NodeKind::Software,
    Lifecycle::Internal,
    &["bridge"],
    (
      vec![live(
        "activation",
        activation,
        PortTransport::UnixSocket,
        "ei",
        "activation-to-eos",
        DataClass::Event,
      )],
      vec![live(
        "activation",
        activation,
        PortTransport::InProcess,
        "eos",
        "activation-local",
        DataClass::Event,
      )],
    ),
  );
  ingress.description = "Receives run-fenced EI activation events".into();
  ingress.bridge = Some(BridgeDeclaration {
    adapter: "unix_socket".into(),
    inputs: vec!["activation".into()],
    outputs: vec![],
  });
  let mut skill = node_type(
    "eos.skill.demo.joint_delta@v0.2",
    ("eos", "skill", "demo"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![live(
        "activation",
        activation,
        PortTransport::InProcess,
        "eos",
        "activation-local",
        DataClass::Event,
      )],
      vec![live(
        "delta",
        delta,
        PortTransport::InProcess,
        "eos",
        "arm-delta",
        DataClass::Command,
      )],
    ),
  );
  skill.description = "Converts activation into the built-in six-joint demonstration delta".into();
  let mut dispatcher = node_type(
    "eos.dispatcher.arm.delta@v0.2",
    ("eos", "dispatcher", "arm"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![live(
        "delta",
        delta,
        PortTransport::InProcess,
        "eos",
        "arm-delta",
        DataClass::Command,
      )],
      vec![live(
        "delta",
        delta,
        PortTransport::InProcess,
        "eos",
        "arm-delta",
        DataClass::Command,
      )],
    ),
  );
  dispatcher.description = "Admits six-joint deltas before mock hardware movement".into();
  let mut hardware = node_type(
    "eos.hardware.mock.arm@v0.2",
    ("eos", "hardware", "mock"),
    NodeKind::Manipulator,
    Lifecycle::Adapter,
    &[],
    (
      vec![live(
        "delta",
        delta,
        PortTransport::InProcess,
        "eos",
        "arm-delta",
        DataClass::Command,
      )],
      vec![live(
        "observation",
        monitor,
        PortTransport::InProcess,
        "eos",
        "arm-monitor-local",
        DataClass::Observation,
      )],
    ),
  );
  hardware.description = "Holds six mock arm joint positions and applies admitted deltas".into();
  hardware.capabilities = vec!["joint_positions".into()];
  hardware.safety_adapter_type = Some("mock_arm".into());
  let mut sample_interval = setting("sample_interval_ms", SettingType::PositiveInteger, false);
  sample_interval.description = Some("Mock joint sampling period (10 ms = 100 Hz)".into());
  sample_interval.unit = Some("ms".into());
  sample_interval.min = Some(1);
  sample_interval.max = Some(1000);
  sample_interval.default = Some(toml::Value::Integer(10));
  let mut freshness_timeout = setting("freshness_timeout_ms", SettingType::PositiveInteger, false);
  freshness_timeout.description = Some("Maximum sample age before movement is blocked".into());
  freshness_timeout.unit = Some("ms".into());
  freshness_timeout.min = Some(2);
  freshness_timeout.max = Some(5000);
  freshness_timeout.default = Some(toml::Value::Integer(50));
  hardware.settings = vec![sample_interval, freshness_timeout];
  let mut output = node_type(
    "eos.hardware.mock.arm_output@v0.2",
    ("eos", "hardware", "mock"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![live(
        "observation",
        monitor,
        PortTransport::InProcess,
        "eos",
        "arm-monitor-local",
        DataClass::Observation,
      )],
      vec![live(
        "observation",
        monitor,
        PortTransport::InProcess,
        "eos",
        "arm-monitor-local",
        DataClass::Observation,
      )],
    ),
  );
  output.description = "Passes the mock arm observation to EOS monitoring".into();
  let mut observer = node_type(
    "eos.hardware.mock.arm_observer@v0.2",
    ("eos", "hardware", "mock"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![],
      vec![live(
        "observation",
        monitor,
        PortTransport::InProcess,
        "eos",
        "arm-monitor-local",
        DataClass::Observation,
      )],
    ),
  );
  observer.description =
    "Reads the sole mock arm sampler as an independent observation source".into();
  let mut monitor_dispatcher = node_type(
    "eos.dispatcher.arm.monitor@v0.2",
    ("eos", "dispatcher", "arm"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![live(
        "observation",
        monitor,
        PortTransport::InProcess,
        "eos",
        "arm-monitor-local",
        DataClass::Observation,
      )],
      vec![live(
        "observation",
        monitor,
        PortTransport::InProcess,
        "eos",
        "arm-monitor-local",
        DataClass::Observation,
      )],
    ),
  );
  monitor_dispatcher.description = "Checks mock arm observation freshness and validity".into();
  let mut status_skill = node_type(
    "eos.skill.demo.arm_status@v0.2",
    ("eos", "skill", "demo"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![live(
        "observation",
        monitor,
        PortTransport::InProcess,
        "eos",
        "arm-monitor-local",
        DataClass::Observation,
      )],
      vec![live(
        "observation",
        monitor,
        PortTransport::InProcess,
        "eos",
        "arm-monitor-local",
        DataClass::Observation,
      )],
    ),
  );
  status_skill.description = "Passes checked arm status toward EI".into();
  let mut egress = node_type(
    "eos.interface.egress.arm_status@v0.2",
    ("eos", "interface", "egress"),
    NodeKind::Software,
    Lifecycle::Internal,
    &["bridge"],
    (
      vec![live(
        "observation",
        monitor,
        PortTransport::InProcess,
        "eos",
        "arm-monitor-local",
        DataClass::Observation,
      )],
      vec![live(
        "observation",
        monitor,
        PortTransport::UnixSocket,
        "eos",
        "arm-monitor-to-ei",
        DataClass::Observation,
      )],
    ),
  );
  egress.description = "Publishes bounded, run-fenced arm status to EI".into();
  egress.bridge = Some(BridgeDeclaration {
    adapter: "unix_socket".into(),
    inputs: vec![],
    outputs: vec!["observation".into()],
  });
  vec![
    ingress,
    skill,
    dispatcher,
    hardware,
    output,
    observer,
    monitor_dispatcher,
    status_skill,
    egress,
  ]
}

fn builtin_data_purpose(id: &str) -> &'static str {
  match id {
    "button_activation@v0.2" => "General user activation event",
    "arm_joint_delta@v0.2" => "Six ordered relative arm joint angles in radians",
    "arm_monitor_state@v0.2" => "Run-fenced six-joint mock arm observation",
    "camera_frame" => "RGB-D camera sample with source provenance and calibration",
    "vision_frame" => "JPEG preview derived from a camera gate",
    "base_observation" => "Mobile base pose and velocity sample",
    "manipulator_observation" => "Joint and end effector sample",
    "base_movement" => "Admitted mobile base movement request",
    "manipulator_movement" => "Admitted manipulator movement request",
    "control_request" => "Typed request for the EOS command dispatcher",
    "command_outcome" => "Result of a dispatched hardware command",
    "skill_outcome" => "Result of a composed skill execution",
    "health" => "Adapter health and last successful observation",
    "status" => "Live node lifecycle and active command",
    "observation_ref" => "Reference to a source observation",
    "context_snapshot" => "Bounded EI context snapshot",
    "plan_proposal" => "EI proposed plan requiring admission",
    _ => unreachable!("built-in data type"),
  }
}

fn builtin_data_schema(id: &str) -> serde_json::Value {
  use serde_json::json;
  let properties = match id {
    "button_activation@v0.2" => {
      return json!({"type":"object","additionalProperties":false,"required":["event_id","kind"],"properties":{"event_id":{"type":"string","minLength":1,"maxLength":64,"pattern":"^[A-Za-z0-9_.-]+$"},"kind":{"const":"activate"}}})
    }
    "arm_monitor_state@v0.2" => {
      return json!({
        "type":"object","additionalProperties":false,
        "required":["run_id","revision","source_node","sequence","sampled_at_monotonic_ns","joint_names","joint_positions_rad","state","reason_code"],
        "properties":{
          "run_id":{"type":"string","minLength":1,"maxLength":128},
          "revision":{"type":"string","pattern":"^[0-9a-f]{64}$"},
          "source_node":{"type":"string","pattern":"^eos/hardware/mock/[A-Za-z0-9_-]+$"},
          "sequence":{"type":"integer","minimum":1},
          "sampled_at_monotonic_ns":{"type":"integer","minimum":0,"x-unit":"ns"},
          "joint_names":{"type":"array","minItems":6,"maxItems":6,"uniqueItems":true,"items":{"type":"string","minLength":1}},
          "joint_positions_rad":{"type":["array","null"],"minItems":6,"maxItems":6,"items":{"type":"number"},"x-unit":"rad"},
          "state":{"type":"string","enum":["ready","degraded","stale","stopped"]},
          "reason_code":{"type":["string","null"]}
        },
        "if":{"properties":{"state":{"const":"ready"}}},
        "then":{"properties":{"joint_positions_rad":{"type":"array"}}},
        "else":{"properties":{"joint_positions_rad":{"type":"null"}}}
      })
    }
    "arm_joint_delta@v0.2" => {
      return json!({"type":"object","additionalProperties":false,"required":["joint_deltas_rad"],"properties":{"joint_deltas_rad":{"type":"array","minItems":6,"maxItems":6,"items":{"type":"number","minimum":-0.25,"maximum":0.25},"x-unit":"rad"}}})
    }
    "camera_frame" => json!({
        "meta":{"type":"object","description":"Source timestamp, clock, receive time, sequence and frame ID"},
        "width_px":{"type":"integer","minimum":1,"x-unit":"px","description":"RGB width"},
        "height_px":{"type":"integer","minimum":1,"x-unit":"px","description":"RGB height"},
        "stride_bytes":{"type":"integer","minimum":1,"x-unit":"bytes/row","description":"RGB row stride"},
        "rgb8":{"type":"array","description":"Interleaved 8-bit RGB pixels"},
        "depth":{"type":["object","null"],"description":"Optional aligned u16 depth, metres per unit and intrinsics"}
    }),
    "vision_frame" => json!({
        "gate_path":{"type":"string","description":"Canonical EOS camera gate path"},
        "sequence":{"type":["integer","null"],"minimum":0,"description":"Source camera sequence"},
        "timestamp_unix_ms":{"type":["integer","null"],"x-unit":"ms","description":"Source Unix timestamp when available"},
        "width_px":{"type":"integer","minimum":1,"x-unit":"px","description":"JPEG pixel width"},
        "height_px":{"type":"integer","minimum":1,"x-unit":"px","description":"JPEG pixel height"},
        "source_rate_hz":{"type":"number","exclusiveMinimum":0,"x-unit":"Hz","description":"Declared source camera rate"},
        "jpeg":{"type":"string","contentEncoding":"binary","description":"JPEG image bytes, at most 8 MiB"}
    }),
    "base_observation" => json!({
        "meta":{"type":"object","description":"Source sample metadata"},
        "valid":{"type":"boolean","description":"Whether pose and velocity are valid"},
        "x_m":{"type":"number","x-unit":"m","description":"Base x position"},
        "y_m":{"type":"number","x-unit":"m","description":"Base y position"},
        "yaw_rad":{"type":"number","x-unit":"rad","description":"Base heading"},
        "x_m_s":{"type":"number","x-unit":"m/s","description":"Longitudinal velocity"},
        "y_m_s":{"type":"number","x-unit":"m/s","description":"Lateral velocity"},
        "yaw_rad_s":{"type":"number","x-unit":"rad/s","description":"Angular velocity"}
    }),
    "manipulator_observation" => json!({
        "meta":{"type":"object","description":"Source sample metadata"},
        "joint_names":{"type":"array","items":{"type":"string"},"description":"Ordered joint names"},
        "joint_positions_rad":{"type":"array","items":{"type":"number"},"x-unit":"rad","description":"Ordered joint angles"},
        "joint_velocities_rad_s":{"type":"array","items":{"type":"number"},"x-unit":"rad/s","description":"Ordered joint velocities"},
        "joints_valid":{"type":"boolean","description":"Whether joint values are valid"},
        "end_effector_pose":{"type":["object","null"],"description":"Optional Cartesian position, orientation and frame"},
        "end_effector_valid":{"type":"boolean","description":"Whether end effector pose is valid"}
    }),
    "base_movement" => json!({
        "body_velocity":{"type":"object","description":"x and y velocity in m/s, yaw rate in rad/s, lease in ms"},
        "relative_pose":{"type":"object","description":"Relative x/y metres, yaw radians and frame ID"}
    }),
    "manipulator_movement" => json!({
        "joint_positions":{"type":"object","description":"Joint names, target angles in rad, optional speed rad/s"},
        "end_effector_pose":{"type":"object","description":"Target Cartesian pose and optional speed m/s"}
    }),
    "health" => json!({
        "state":{"type":"string","enum":["healthy","degraded","unavailable"],"description":"Current adapter health"},
        "code":{"type":["string","null"],"description":"Optional machine code"},
        "message":{"type":["string","null"],"description":"Optional operator explanation"},
        "last_ok_monotonic_ns":{"type":["integer","null"],"minimum":0,"x-unit":"ns","description":"Last successful local sample time"}
    }),
    "status" => json!({
        "state":{"type":"string","enum":["starting","ready","stopped","faulted"],"description":"Node lifecycle state"},
        "active_command_id":{"type":["string","null"],"description":"Current admitted command ID"}
    }),
    "control_request" => {
      json!({"node_path":{"type":"string","description":"Target graph path"},"command_id":{"type":"string","description":"Idempotency key"},"deadline_monotonic_ns":{"type":"integer","minimum":0,"x-unit":"ns","description":"Local admission deadline"}})
    }
    "command_outcome" | "skill_outcome" => {
      json!({"state":{"type":"string","description":"Accepted, completed, cancelled or uncertain result"},"message":{"type":["string","null"],"description":"Optional result detail"}})
    }
    "observation_ref" => {
      json!({"source_node":{"type":"string","description":"Source graph path"},"port":{"type":"string","description":"Source output port"},"sequence":{"type":["integer","null"],"description":"Optional source sequence"}})
    }
    "context_snapshot" => {
      json!({"observations":{"type":"array","description":"Bounded referenced observations"},"created_monotonic_ns":{"type":"integer","minimum":0,"x-unit":"ns","description":"Snapshot creation time"}})
    }
    "plan_proposal" => {
      json!({"steps":{"type":"array","description":"Proposed typed steps"},"source_context":{"type":"string","description":"Context snapshot reference"}})
    }
    _ => unreachable!("built-in data type"),
  };
  json!({"type":"object","description":builtin_data_purpose(id),"properties":properties})
}

#[cfg(test)]
mod object_array_tests {
  use super::*;

  #[test]
  fn nested_items_are_checked_without_coercion_or_extra_fields() {
    let mut setting = setting("skill_executables", SettingType::ObjectArray, false);
    setting.description = Some("Exact package libraries".into());
    setting.items_schema = Some(serde_json::json!({
      "type":"object","additionalProperties":false,
      "required":["package","library"],
      "properties":{
        "package":{"type":"object","additionalProperties":false,
          "required":["id","version"],
          "properties":{"id":{"type":"string","minLength":1},
            "version":{"type":"string","minLength":1}}},
        "library":{"type":"string","minLength":1,"maxLength":256}
      }
    }));
    setting.min_items = Some(0);
    setting.max_items = Some(16);
    assert!(valid_setting_descriptor(&setting));
    let good = toml::Value::try_from(serde_json::json!([{
      "package":{"id":"island/example","version":"0.1.0"},"library":"lib/a.so"
    }]))
    .unwrap();
    assert!(valid_setting(&good, setting.data_type));
    assert!(valid_setting_value(&good, &setting));
    for invalid in [
      serde_json::json!([{"package":{"id":"island/example","version":"0.1.0"}}]),
      serde_json::json!([{"package":{"id":"island/example","version":"0.1.0"},"library":8}]),
      serde_json::json!([{"package":{"id":"island/example","version":"0.1.0","extra":"x"},"library":"lib/a.so"}]),
      serde_json::json!([{"package":{"id":"","version":"0.1.0"},"library":"lib/a.so"}]),
    ] {
      let value = toml::Value::try_from(invalid).unwrap();
      assert!(!valid_setting_value(&value, &setting));
    }
    let seventeen = toml::Value::Array(vec![good.as_array().unwrap()[0].clone(); 17]);
    assert!(!valid_setting_value(&seventeen, &setting));
    setting.items_schema = Some(serde_json::json!({"type":"string"}));
    assert!(!valid_setting_descriptor(&setting));
  }
}
