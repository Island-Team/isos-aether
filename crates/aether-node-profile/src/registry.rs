use std::{
  collections::{BTreeMap, BTreeSet},
  sync::{Arc, LazyLock, Mutex},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::{valid_segment, NodeKind, NodeProfile, PackageRef, ProfileError};

static TYPE_IDL: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
  let source: serde_json::Value =
    serde_json::from_str(include_str!("../../../contract/node/node-type.schema.json"))
      .expect("embedded node type IDL is JSON");
  jsonschema::validator_for(&source).expect("embedded node type IDL is valid")
});

// Data descriptors are immutable after registry construction. Compile each
// distinct schema once: frame validation may run for every camera sample.
static DATA_SCHEMA_CACHE: LazyLock<Mutex<BTreeMap<[u8; 32], Arc<jsonschema::Validator>>>> =
  LazyLock::new(|| Mutex::new(BTreeMap::new()));

fn compiled_data_schema(
  schema: &serde_json::Value,
) -> Result<Arc<jsonschema::Validator>, ProfileError> {
  let bytes = serde_json::to_vec(schema)
    .map_err(|error| ProfileError(format!("invalid schema JSON: {error}")))?;
  let key: [u8; 32] = Sha256::digest(bytes).into();
  let mut cache = DATA_SCHEMA_CACHE
    .lock()
    .map_err(|_| ProfileError("data schema cache unavailable".into()))?;
  if let Some(validator) = cache.get(&key) {
    return Ok(Arc::clone(validator));
  }
  let validator = Arc::new(
    jsonschema::validator_for(schema)
      .map_err(|error| ProfileError(format!("invalid data schema: {error}")))?,
  );
  if cache.len() >= 128 {
    cache.pop_first();
  }
  cache.insert(key, Arc::clone(&validator));
  Ok(validator)
}

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
  pub frequency_hz: Option<f64>,
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
  /// Required for package actuator types.
  #[serde(default)]
  pub safety_adapter_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImplementationDescriptor {
  pub kind: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub abi: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub library: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub interface: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub lifecycle_adapter: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeLibrary {
  pub path: String,
  pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterLibrary {
  pub family: String,
  pub library: String,
  pub sha256: String,
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
  native_libraries: BTreeMap<String, std::path::PathBuf>,
  native_digests: BTreeMap<String, String>,
  family_libraries: BTreeMap<String, std::path::PathBuf>,
  family_digests: BTreeMap<String, String>,
  port_routes: Vec<serde_json::Value>,
  bridge_adapters: Vec<serde_json::Value>,
}

/// Verified package contribution passed by IsOS, separate from graph instances.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeTypeContribution {
  pub package: PackageRef,
  #[serde(default)]
  pub libraries: Vec<NativeLibrary>,
  #[serde(default)]
  pub adapters: Vec<AdapterLibrary>,
  #[serde(default)]
  pub port_routes: Vec<serde_json::Value>,
  #[serde(default)]
  pub bridge_adapters: Vec<serde_json::Value>,
  #[serde(default)]
  pub node_types: Vec<NodeType>,
  #[serde(default)]
  pub safety_adapters: Vec<SafetyAdapterType>,
  #[serde(default)]
  pub data_types: Vec<DataTypeDescriptor>,
  #[serde(default)]
  pub adapter_families: Vec<AdapterFamilyDescriptor>,
}

fn safe_library_path(path: &str) -> bool {
  let path = std::path::Path::new(path);
  path.extension().is_some_and(|ext| ext == "so")
    && !path.is_absolute()
    && path
      .components()
      .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn valid_digest(digest: &str) -> bool {
  digest.len() == 64
    && digest
      .bytes()
      .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn verify_native_libraries(
  path: &std::path::Path,
  contribution: &NodeTypeContribution,
) -> Result<(), ProfileError> {
  let root = contribution_root(path)?;
  for library in &contribution.libraries {
    if !safe_library_path(&library.path) || !valid_digest(&library.sha256) {
      return Err(ProfileError(format!(
        "invalid native library declaration: {}",
        library.path
      )));
    }
    let candidate = root.join(&library.path);
    let resolved = candidate
      .canonicalize()
      .map_err(|error| ProfileError(format!("{}: {error}", candidate.display())))?;
    if !resolved.starts_with(&root) || !resolved.is_file() {
      return Err(ProfileError(format!(
        "native library escapes verified package: {}",
        library.path
      )));
    }
    let bytes = std::fs::read(&resolved)
      .map_err(|error| ProfileError(format!("{}: {error}", resolved.display())))?;
    if bytes.len() > 64 * 1024 * 1024 {
      return Err(ProfileError(format!(
        "native library exceeds 64 MiB: {}",
        library.path
      )));
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != library.sha256 {
      return Err(ProfileError(format!(
        "native library digest mismatch: {}",
        library.path
      )));
    }
  }
  Ok(())
}

fn contribution_root(path: &std::path::Path) -> Result<std::path::PathBuf, ProfileError> {
  let parent = path
    .parent()
    .ok_or_else(|| ProfileError("node contribution has no package directory".into()))?;
  let receipt = parent.join("receipt.json");
  let linked = std::fs::read(&receipt)
    .ok()
    .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    .and_then(|value| value["local_link"].as_str().map(std::path::PathBuf::from));
  let root = linked.unwrap_or_else(|| parent.to_path_buf());
  root
    .canonicalize()
    .map_err(|error| ProfileError(format!("{}: {error}", root.display())))
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
      verify_native_libraries(path, &contribution)?;
      let adapter_libraries: Vec<(String, String, String)> = contribution
        .adapters
        .iter()
        .map(|adapter| {
          let declared = contribution
            .libraries
            .iter()
            .find(|library| library.path == adapter.library);
          if !contribution
            .adapter_families
            .iter()
            .any(|family| family.id == adapter.family)
            || declared.is_none_or(|library| library.sha256 != adapter.sha256)
          {
            return Err(ProfileError(format!(
              "adapter {} lacks a matching verified family and library",
              adapter.family
            )));
          }
          Ok((
            adapter.family.clone(),
            adapter.library.clone(),
            adapter.sha256.clone(),
          ))
        })
        .collect::<Result<_, _>>()?;
      let native_types: Vec<(String, String, String)> = contribution
        .node_types
        .iter()
        .filter_map(|node| {
          let implementation = node.implementation.as_ref()?;
          if implementation.kind != "native_plugin" {
            return None;
          }
          let library = implementation.library.clone()?;
          let digest = contribution
            .libraries
            .iter()
            .find(|item| item.path == library)?
            .sha256
            .clone();
          Some((node.id.clone(), library, digest))
        })
        .collect();
      registry.register_contribution(contribution)?;
      for (id, library, digest) in native_types {
        registry.native_libraries.insert(
          id.clone(),
          contribution_root(path)?
            .join(library)
            .canonicalize()
            .map_err(|error| ProfileError(error.to_string()))?,
        );
        registry.native_digests.insert(id, digest);
      }
      for (family, library, digest) in adapter_libraries {
        let resolved = contribution_root(path)?
          .join(library)
          .canonicalize()
          .map_err(|error| ProfileError(error.to_string()))?;
        if registry
          .family_libraries
          .insert(family.clone(), resolved)
          .is_some()
        {
          return Err(ProfileError(format!(
            "duplicate adapter library for {family}"
          )));
        }
        registry.family_digests.insert(family, digest);
      }
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
      "image_frame@v0.2",
      "base_state@v0.2",
      "joint_state@v0.2",
      "base_twist_command@v0.2",
      "joint_position_command@v0.2",
      "pose_state@v0.2",
      "gripper_state@v0.2",
      "normalized_axes_command@v0.2",
      "gripper_binary_command@v0.2",
      "control_mode_command@v0.2",
      "sample_request@v0.2",
      "movement_binding_request@v0.2",
      "physical_event@v0.2",
      "device_status@v0.2",
      "binding_result@v0.2",
      "stop_request@v0.2",
      "stop_latch@v0.2",
      "evidence_record@v0.2",
      "episode_reference@v0.2",
      "gap_record@v0.2",
      "manifest_request@v0.2",
      "dataset_manifest_reference@v0.2",
      "depth_frame_ref@v0.2",
      "segmentation_mask_ref@v0.2",
      "point_cloud_ref@v0.2",
      "physical_detection@v0.2",
      "imu_sample@v0.2",
      "force_torque_sample@v0.2",
      "tactile_sample@v0.2",
      "contact_event@v0.2",
      "audio_chunk_ref@v0.2",
    ] {
      registry
        .register_builtin_data_type(DataTypeDescriptor {
          id: id.into(),
          version: "0.2".into(),
          purpose: builtin_data_purpose(id).into(),
          media_type: "application/json".into(),
          codec: "json_v1".into(),
          schema: builtin_data_schema(id),
          max_payload_bytes: if id == "image_frame@v0.2" {
            20 * 1024 * 1024
          } else if crate::scene_data::is_scene_type(id) {
            4_096
          } else {
            16_384
          },
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
    for (id, kind, mode) in [
      ("base_velocity", NodeKind::Base, "body_velocity"),
      (
        "manipulator_joint_position",
        NodeKind::Manipulator,
        "joint_positions",
      ),
      ("base_normalized_axes", NodeKind::Base, "normalized_axes"),
      (
        "manipulator_normalized_cartesian",
        NodeKind::Manipulator,
        "normalized_cartesian",
      ),
      ("gripper_binary", NodeKind::Gripper, "binary"),
      ("torso_normalized_axes", NodeKind::Torso, "normalized_axes"),
    ] {
      registry
        .register_safety_adapter(SafetyAdapterType {
          id: id.into(),
          kind,
          movement_modes: vec![mode.into()],
          supports_stop: true,
        })
        .expect("valid general safety declaration");
    }
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
    for mut definition in builtin_types() {
      if matches!(
        definition.id.as_str(),
        "eos.skill.demo.joint_delta@v0.2" | "eos.skill.demo.arm_status@v0.2"
      ) {
        definition.implementation = Some(ImplementationDescriptor {
          kind: "native_plugin".into(),
          abi: Some("isos_node_v1".into()),
          library: Some("libaether_runtime.so".into()),
          interface: None,
          version: None,
          lifecycle_adapter: None,
        });
      }
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

  pub fn port_routes(&self) -> &[serde_json::Value] {
    &self.port_routes
  }
  pub fn bridge_adapters(&self) -> &[serde_json::Value] {
    &self.bridge_adapters
  }

  pub fn catalog_port_routes(&self) -> Vec<serde_json::Value> {
    let mut routes = self.port_routes.clone();
    for camera in self.all().filter(|node| {
      node.core == "eos"
        && node.kind == NodeKind::Camera
        && node.outputs.iter().any(|port| {
          port.name == "frame"
            && port.data_type.id() == "image_frame@v0.2"
            && port.transport == Some(PortTransport::InProcess)
            && port.delivery_mode == DeliveryMode::Event
            && port.data_class == Some(DataClass::Image)
        })
    }) {
      routes.push(serde_json::json!({"source_type":camera.id,"output":"frame",
        "target_type":"eos.interface.egress.vision_flow@v0.2","input":"frame",
        "transport":"in_process","delivery_mode":"event","data_type":"image_frame@v0.2",
        "contract":{"owner":"eos","id":"image_frame@v0.2","version":"0.2"}}));
    }
    let pairs = [
      (
        "ei.governance.safety.gate@v0.2",
        "activation",
        "eos.interface.ingress.trigger@v0.2",
        "activation",
      ),
      (
        "eos.interface.ingress.trigger@v0.2",
        "activation",
        "eos.skill.demo.joint_delta@v0.2",
        "activation",
      ),
      (
        "eos.skill.demo.joint_delta@v0.2",
        "delta",
        "eos.dispatcher.arm.delta@v0.2",
        "delta",
      ),
      (
        "eos.dispatcher.arm.delta@v0.2",
        "delta",
        "eos.hardware.mock.arm@v0.2",
        "delta",
      ),
      (
        "eos.hardware.mock.arm@v0.2",
        "observation",
        "eos.hardware.mock.arm_output@v0.2",
        "observation",
      ),
      (
        "eos.hardware.mock.arm_output@v0.2",
        "observation",
        "eos.dispatcher.arm.monitor@v0.2",
        "observation",
      ),
      (
        "eos.hardware.mock.arm_observer@v0.2",
        "observation",
        "eos.dispatcher.arm.monitor@v0.2",
        "observation",
      ),
      (
        "eos.dispatcher.arm.monitor@v0.2",
        "observation",
        "eos.skill.demo.arm_status@v0.2",
        "observation",
      ),
      (
        "eos.skill.demo.arm_status@v0.2",
        "observation",
        "eos.interface.egress.arm_status@v0.2",
        "observation",
      ),
      (
        "eos.interface.egress.arm_status@v0.2",
        "observation",
        "ei.io.input.arm_status@v0.2",
        "observation",
      ),
    ];
    for (source_type, output, target_type, input) in pairs {
      let port = self
        .get(target_type)
        .and_then(|node| node.inputs.iter().find(|port| port.name == input))
        .or_else(|| {
          self
            .get(source_type)
            .and_then(|node| node.outputs.iter().find(|port| port.name == output))
        });
      if let Some(port) = port {
        routes.push(serde_json::json!({"source_type":source_type,"output":output,"target_type":target_type,"input":input,
          "transport":port.transport,"delivery_mode":port.delivery_mode,"data_type":port.data_type,"contract":port.contract}));
      }
    }
    routes
  }

  pub fn catalog_bridge_adapters(&self) -> Vec<serde_json::Value> {
    let mut adapters = self.bridge_adapters.clone();
    for node_type in [
      "eos.interface.ingress.trigger@v0.2",
      "eos.interface.egress.arm_status@v0.2",
      "eos.interface.egress.vision_flow@v0.2",
    ] {
      adapters
        .push(serde_json::json!({"core":"eos","adapter":"unix_socket","node_type":node_type}));
    }
    adapters
  }

  pub fn definitions(&self) -> impl Iterator<Item = &NodeType> {
    self.all()
  }

  pub fn package_for_type(&self, id: &str) -> Option<&PackageRef> {
    self.packages.get(id)
  }

  pub fn native_library_for_type(&self, id: &str) -> Option<&std::path::Path> {
    self
      .native_libraries
      .get(id)
      .map(std::path::PathBuf::as_path)
  }

  pub fn verified_native_library_for_type(
    &self,
    id: &str,
  ) -> Result<&std::path::Path, ProfileError> {
    let library = self
      .native_library_for_type(id)
      .ok_or_else(|| ProfileError(format!("native library unavailable for {id}")))?;
    let expected = self
      .native_digests
      .get(id)
      .ok_or_else(|| ProfileError(format!("native digest unavailable for {id}")))?;
    let bytes = std::fs::read(library)
      .map_err(|error| ProfileError(format!("{}: {error}", library.display())))?;
    if bytes.len() > 64 * 1024 * 1024 || format!("{:x}", Sha256::digest(&bytes)) != *expected {
      return Err(ProfileError(format!(
        "native library changed after catalog: {}",
        library.display()
      )));
    }
    Ok(library)
  }

  pub fn verified_adapter_library_for_family(
    &self,
    family: &str,
  ) -> Result<&std::path::Path, ProfileError> {
    let library = self
      .family_libraries
      .get(family)
      .ok_or_else(|| ProfileError(format!("adapter library unavailable for {family}")))?;
    let expected = self
      .family_digests
      .get(family)
      .ok_or_else(|| ProfileError(format!("adapter digest unavailable for {family}")))?;
    let bytes = std::fs::read(library)
      .map_err(|error| ProfileError(format!("{}: {error}", library.display())))?;
    if bytes.len() > 64 * 1024 * 1024 || format!("{:x}", Sha256::digest(&bytes)) != *expected {
      return Err(ProfileError(format!(
        "adapter library changed after catalog: {}",
        library.display()
      )));
    }
    Ok(library)
  }

  pub fn data_type(&self, id: &str) -> Option<&DataTypeDescriptor> {
    self.data_types.get(id)
  }

  pub fn validate_data_payload(
    &self,
    id: &str,
    payload: &serde_json::Value,
  ) -> Result<(), ProfileError> {
    let descriptor = self
      .data_type(id)
      .ok_or_else(|| ProfileError(format!("unknown data type: {id}")))?;
    let bytes = serde_json::to_vec(payload)
      .map_err(|error| ProfileError(format!("invalid data payload: {error}")))?;
    if bytes.len() as u64 > descriptor.max_payload_bytes {
      return Err(ProfileError(format!("payload exceeds bound for {id}")));
    }
    let schema = compiled_data_schema(&descriptor.schema)
      .map_err(|error| ProfileError(format!("invalid schema for {id}: {error}")))?;
    if !schema.is_valid(payload) {
      return Err(ProfileError(format!("payload does not match {id}")));
    }
    if id == "pose_state@v0.2" {
      let orientation = payload["orientation_xyzw"]
        .as_array()
        .ok_or_else(|| ProfileError("pose quaternion missing".into()))?;
      let norm = orientation
        .iter()
        .filter_map(serde_json::Value::as_f64)
        .map(|value| value * value)
        .sum::<f64>();
      if !norm.is_finite() || (norm - 1.0).abs() > 1e-3 {
        return Err(ProfileError("pose quaternion must have unit norm".into()));
      }
    }
    if id == "gripper_state@v0.2" {
      let mut names = BTreeSet::new();
      if payload["channels"].as_array().is_none_or(|channels| {
        channels.iter().any(|channel| {
          channel["name"]
            .as_str()
            .is_none_or(|name| !names.insert(name))
        })
      }) {
        return Err(ProfileError(
          "gripper channels must have unique names".into(),
        ));
      }
    }
    Ok(())
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
          ("mock-cxx", None) | ("mock-arm", None)
        );
        let loadable = !compiled && self.family_libraries.contains_key(&descriptor.id);
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
    for route in &contribution.port_routes {
      if !route.is_object()
        || route["source_type"].as_str().is_none_or(str::is_empty)
        || route["target_type"].as_str().is_none_or(str::is_empty)
        || !["in_process", "unix_socket"].contains(&route["transport"].as_str().unwrap_or(""))
      {
        return Err(ProfileError("invalid contributed EOS port route".into()));
      }
    }
    for bridge in &contribution.bridge_adapters {
      if bridge["core"] != "eos" {
        continue;
      }
      if bridge["adapter"].as_str().is_none_or(str::is_empty)
        || bridge["node_type"].as_str().is_none_or(str::is_empty)
      {
        return Err(ProfileError(
          "invalid contributed EOS bridge adapter".into(),
        ));
      }
    }
    let mut library_paths = BTreeSet::new();
    for library in &contribution.libraries {
      if !safe_library_path(&library.path)
        || !valid_digest(&library.sha256)
        || !library_paths.insert(library.path.as_str())
      {
        return Err(ProfileError(format!(
          "invalid or duplicate native library: {}",
          library.path
        )));
      }
    }
    let mut next = self.clone();
    next.port_routes.extend(
      contribution
        .port_routes
        .iter()
        .filter(|route| {
          route["source_type"]
            .as_str()
            .is_some_and(|id| id.starts_with("eos."))
            || route["target_type"]
              .as_str()
              .is_some_and(|id| id.starts_with("eos."))
        })
        .cloned(),
    );
    next.bridge_adapters.extend(
      contribution
        .bridge_adapters
        .iter()
        .filter(|bridge| bridge["core"] == "eos")
        .cloned(),
    );
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
            NodeKind::Camera
              | NodeKind::Base
              | NodeKind::Manipulator
              | NodeKind::Gripper
              | NodeKind::Torso
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
      if definition.core != "eos" {
        continue;
      }
      if !definition.id.starts_with("eos.") || !definition.id.ends_with("@v0.2") {
        return Err(ProfileError(format!(
          "EOS package node type must use the current graph contract: {}",
          definition.id
        )));
      }
      {
        match definition.implementation.as_ref() {
          Some(implementation) if implementation.kind == "native_plugin" => {
            let library = implementation.library.as_deref().unwrap_or("");
            if !matches!(
              implementation.abi.as_deref(),
              Some("isos_node_v1" | "isos_node_v2")
            ) || !library_paths.contains(library)
              || implementation.interface.is_some()
              || implementation.version.is_some()
              || implementation.lifecycle_adapter.is_some()
            {
              return Err(ProfileError(format!(
                "invalid native implementation for {}",
                definition.id
              )));
            }
          }
          Some(implementation) if implementation.kind == "managed_process" => {
            if implementation
              .interface
              .as_deref()
              .is_none_or(str::is_empty)
              || implementation.version.as_deref() != Some("1")
              || implementation
                .lifecycle_adapter
                .as_deref()
                .is_none_or(str::is_empty)
              || implementation.library.is_some()
              || implementation.abi.is_some()
            {
              return Err(ProfileError(format!(
                "invalid external implementation for {}",
                definition.id
              )));
            }
          }
          _ => {
            return Err(ProfileError(format!(
              "package node type {} requires a supported implementation",
              definition.id
            )))
          }
        }
      }
      let id = definition.id.clone();
      next.register(definition)?;
      next.packages.insert(id, contribution.package.clone());
    }
    for route in &next.port_routes {
      for (type_key, port_key, output) in [
        ("source_type", "output", true),
        ("target_type", "input", false),
      ] {
        let Some(type_id) = route[type_key].as_str() else {
          return Err(ProfileError("route type missing".into()));
        };
        if !type_id.starts_with("eos.") {
          continue;
        }
        let Some(node) = next.get(type_id) else {
          return Err(ProfileError(format!(
            "EOS route references unknown type {type_id}"
          )));
        };
        let ports = if output { &node.outputs } else { &node.inputs };
        let Some(port) = ports
          .iter()
          .find(|port| Some(port.name.as_str()) == route[port_key].as_str())
        else {
          return Err(ProfileError(format!(
            "EOS route references missing port on {type_id}"
          )));
        };
        if serde_json::to_value(port.transport).ok().as_ref() != Some(&route["transport"])
          || serde_json::to_value(port.delivery_mode).ok().as_ref() != Some(&route["delivery_mode"])
          || port.data_type.id() != route["data_type"].as_str().unwrap_or("")
          || serde_json::to_value(&port.contract).ok().as_ref() != Some(&route["contract"])
        {
          return Err(ProfileError(format!(
            "EOS route disagrees with {type_id} port {}",
            port.name
          )));
        }
      }
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
    if matches!(
      definition.kind,
      NodeKind::Base | NodeKind::Manipulator | NodeKind::Gripper | NodeKind::Torso
    ) {
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
      || !matches!(
        definition.kind,
        NodeKind::Base | NodeKind::Manipulator | NodeKind::Gripper | NodeKind::Torso
      )
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
            || p.frequency_hz.is_some_and(|r| !r.is_finite() || r <= 0.0)
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
    Ok(())
  }
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
    frequency_hz: None,
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
      "isos.eos_dispatcher.control@v0.2" => {
        "Admits control requests and emits typed base or arm commands and status."
      }
      "isos.eos_dispatcher.primitive_base@v0.2" => {
        "Binds a declared base primitive to a typed base command output."
      }
      "isos.eos_dispatcher.primitive_manipulator@v0.2" => {
        "Binds a declared arm primitive to a typed arm command output."
      }
      _ => "",
    }
    .into(),
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
  let mut vision = node_type(
    "eos.interface.egress.vision_flow@v0.2",
    ("eos", "interface", "egress"),
    NodeKind::Software,
    Lifecycle::Internal,
    &["bridge"],
    (
      vec![live(
        "frame",
        "image_frame@v0.2",
        PortTransport::InProcess,
        "eos",
        "image_frame@v0.2",
        DataClass::Image,
      )],
      vec![live(
        "frame",
        "image_frame@v0.2",
        PortTransport::UnixSocket,
        "eos",
        "vision-flow-to-isos",
        DataClass::Image,
      )],
    ),
  );
  vision.description = "Transfers validated camera frames to IsOS Vision".into();
  vision.inputs[0].delivery_mode = DeliveryMode::Event;
  vision.inputs[0].contract.as_mut().unwrap().version = "0.2".into();
  vision.outputs[0].delivery_mode = DeliveryMode::Stream;
  vision.bridge = Some(BridgeDeclaration {
    adapter: "unix_socket".into(),
    inputs: vec![],
    outputs: vec!["frame".into()],
  });
  let mut types = vec![
    ingress,
    skill,
    dispatcher,
    hardware,
    output,
    observer,
    monitor_dispatcher,
    status_skill,
    egress,
    vision,
  ];
  types.extend(isolated_types());
  types
}

fn isolated_types() -> Vec<NodeType> {
  fn local(name: &str, data: &str, class: DataClass) -> Port {
    Port {
      name: name.into(),
      data_type: PortDataType::Custom(data.into()),
      contract: None,
      required: false,
      cardinality: PortCardinality::One,
      description: Some(format!("Isolated {data} value")),
      frequency_hz: None,
      max_rate_hz: None,
      transport: Some(PortTransport::None),
      delivery_mode: DeliveryMode::Declaration,
      data_class: Some(class),
    }
  }
  fn frame_setting() -> Setting {
    let mut value = setting("frame_id", SettingType::String, true);
    value.description = Some("Exact source frame identity".into());
    value
  }
  let mut camera = node_type(
    "eos.hardware.mock.camera@v0.2",
    ("eos", "hardware", "mock"),
    NodeKind::Camera,
    Lifecycle::Adapter,
    &[],
    (
      vec![local("sample", "sample_request@v0.2", DataClass::Event)],
      vec![local("frame", "image_frame@v0.2", DataClass::Image)],
    ),
  );
  camera.description = "Produces a deterministic bounded mock RGB frame on direct input".into();
  camera.settings = vec![frame_setting()];
  let mut features = node_type(
    "eos.perception.physical.features@v0.2",
    ("eos", "perception", "physical"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![local("state", "joint_state@v0.2", DataClass::Observation)],
      vec![local("event", "physical_event@v0.2", DataClass::Event)],
    ),
  );
  features.description = "Checks measured joint positions for physical limit proximity".into();
  features.settings = vec![frame_setting()];
  let mut binding = node_type(
    "eos.dispatcher.command.binding@v0.2",
    ("eos", "dispatcher", "command"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![local(
        "request",
        "movement_binding_request@v0.2",
        DataClass::Command,
      )],
      vec![local("binding", "binding_result@v0.2", DataClass::State)],
    ),
  );
  binding.description =
    "Validates one mock-arm primitive binding without releasing movement".into();
  let mut target = setting("target_node", SettingType::String, true);
  target.description = Some("Exact mock arm target path".into());
  binding.settings = vec![frame_setting(), target];
  let mut guard = node_type(
    "eos.safety.device.guard@v0.2",
    ("eos", "safety", "device"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![local("state", "joint_state@v0.2", DataClass::Observation)],
      vec![local("status", "device_status@v0.2", DataClass::State)],
    ),
  );
  guard.description = "Checks measured joint-state freshness and physical position limits".into();
  guard.settings = vec![frame_setting()];
  let mut estop = node_type(
    "eos.safety.device.estop@v0.2",
    ("eos", "safety", "device"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![local("stop", "stop_request@v0.2", DataClass::Command)],
      vec![local("latch", "stop_latch@v0.2", DataClass::State)],
    ),
  );
  estop.description = "Delegates an emergency Stop request to an injected EOS Stop service".into();
  let mut recorder = node_type(
    "eos.evidence.episode.recorder@v0.2",
    ("eos", "evidence", "episode"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![local("record", "evidence_record@v0.2", DataClass::Event)],
      vec![
        local("gap", "gap_record@v0.2", DataClass::Event),
        local("episode", "episode_reference@v0.2", DataClass::State),
      ],
    ),
  );
  recorder.description =
    "Writes bounded run-fenced physical metadata to an owner-supplied append-only store".into();
  let mut manifest = node_type(
    "eos.dataset.export.manifest@v0.2",
    ("eos", "dataset", "export"),
    NodeKind::Software,
    Lifecycle::Internal,
    &[],
    (
      vec![local(
        "request",
        "manifest_request@v0.2",
        DataClass::Command,
      )],
      vec![local(
        "manifest",
        "dataset_manifest_reference@v0.2",
        DataClass::State,
      )],
    ),
  );
  manifest.description =
    "Writes a deterministic manifest from owner-authorized episode references".into();
  for node in [
    &mut camera,
    &mut features,
    &mut binding,
    &mut guard,
    &mut estop,
    &mut recorder,
    &mut manifest,
  ] {
    node.implementation = Some(ImplementationDescriptor {
      kind: "built_in".into(),
      abi: None,
      library: None,
      interface: None,
      version: None,
      lifecycle_adapter: None,
    });
  }
  vec![camera, features, binding, guard, estop, recorder, manifest]
}

fn builtin_data_purpose(id: &str) -> &'static str {
  match id {
    "button_activation@v0.2" => "General user activation event",
    "arm_joint_delta@v0.2" => "Six ordered relative arm joint angles in radians",
    "arm_monitor_state@v0.2" => "Run-fenced six-joint mock arm observation",
    "image_frame@v0.2" => "Bounded camera image with source time and frame identity",
    "sample_request@v0.2" => "Run-local request for one deterministic mock camera sample",
    "movement_binding_request@v0.2" => "Exact mock-arm primitive binding candidate",
    "physical_event@v0.2" => "Measured physical limit proximity event",
    "device_status@v0.2" => "Measured device safety status",
    "binding_result@v0.2" => "Validated primitive binding without actuator release",
    "stop_request@v0.2" => "Run-fenced request for emergency Stop",
    "stop_latch@v0.2" => "EOS emergency Stop latch and proof reference",
    "evidence_record@v0.2" => "Bounded physical metadata record with source identity",
    "episode_reference@v0.2" => "Digest-qualified local physical episode reference",
    "gap_record@v0.2" => "Explicit physical evidence loss record",
    "manifest_request@v0.2" => "Authorized dataset manifest selection and policy",
    "dataset_manifest_reference@v0.2" => "Digest-qualified local dataset manifest reference",
    "depth_frame_ref@v0.2" => "Bounded reference to calibrated physical depth pixels",
    "segmentation_mask_ref@v0.2" => "Bounded reference to a binary physical mask",
    "point_cloud_ref@v0.2" => "Bounded reference to calibrated XYZ point samples",
    "physical_detection@v0.2" => "Source-attributed physical obstacle detection",
    "imu_sample@v0.2" => "Calibrated inertial acceleration and angular velocity",
    "force_torque_sample@v0.2" => "Calibrated three-axis force and torque sample",
    "tactile_sample@v0.2" => "Calibrated bounded tactile pressure channels",
    "contact_event@v0.2" => "Source-attributed physical contact transition",
    "audio_chunk_ref@v0.2" => "Bounded reference to microphone PCM audio bytes",
    "base_state@v0.2" => "Frame-qualified mobile base pose and velocity",
    "joint_state@v0.2" => "Named manipulator joint positions and velocities",
    "base_twist_command@v0.2" => "Bounded body-frame base velocity request",
    "joint_position_command@v0.2" => "Named manipulator joint position request",
    "pose_state@v0.2" => "Frame-qualified position and unit-quaternion observation",
    "gripper_state@v0.2" => "Named gripper channels with explicit units",
    "normalized_axes_command@v0.2" => "Declared normalized axes with a bounded lease",
    "gripper_binary_command@v0.2" => "Binary gripper open or close request",
    "control_mode_command@v0.2" => "Declared control mode request",
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
  let sample = json!({
    "type":"object","additionalProperties":false,
    "required":["source_time_ns","clock","sequence","frame_id"],
    "properties":{
      "source_time_ns":{"type":"integer","minimum":0,"x-unit":"ns"},
      "clock":{"enum":["system","simulation","monotonic"]},
      "sequence":{"type":"integer","minimum":0},
      "frame_id":{"type":"string","minLength":1,"maxLength":128}
    }
  });
  if let Some(schema) = crate::scene_data::schema(id, &sample) {
    return schema;
  }
  let properties = match id {
    "stop_request@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
      "required":["run_id","reason","source_time_ns"],"properties":{
        "run_id":{"type":"string","minLength":1,"maxLength":128},
        "reason":{"type":"string","minLength":1,"maxLength":128},
        "source_time_ns":{"type":"integer","minimum":0,"x-unit":"ns"}}})
    }
    "stop_latch@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
      "required":["run_id","latched","proof_ref"],"properties":{
        "run_id":{"type":"string","minLength":1,"maxLength":128},
        "latched":{"const":true},"proof_ref":{"type":"string","minLength":1,"maxLength":128}}})
    }
    "evidence_record@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
      "required":["run_id","record_id","source_time_ns","kind","source_ref"],"properties":{
        "run_id":{"type":"string","minLength":1,"maxLength":128},
        "record_id":{"type":"string","minLength":1,"maxLength":128},
        "source_time_ns":{"type":"integer","minimum":0,"x-unit":"ns"},
        "kind":{"enum":["sample","command","event","intervention"]},
        "source_ref":{"type":"string","minLength":1,"maxLength":128}}})
    }
    "episode_reference@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
      "required":["run_id","episode_id","record_count","sha256"],"properties":{
        "run_id":{"type":"string","minLength":1,"maxLength":128},
        "episode_id":{"type":"string","minLength":1,"maxLength":128},
        "record_count":{"type":"integer","minimum":0},
        "sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}})
    }
    "gap_record@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
      "required":["run_id","episode_id","lost_count","reason"],"properties":{
        "run_id":{"type":"string","minLength":1,"maxLength":128},
        "episode_id":{"type":"string","minLength":1,"maxLength":128},
        "lost_count":{"type":"integer","minimum":1},
        "reason":{"enum":["byte_budget","write_failure"]}}})
    }
    "manifest_request@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
      "required":["run_id","episodes","split_policy_id","label_policy_id"],"properties":{
        "run_id":{"type":"string","minLength":1,"maxLength":128},
        "episodes":{"type":"array","minItems":1,"maxItems":128,"uniqueItems":true,
          "items":{"type":"object","additionalProperties":false,
            "required":["episode_id","record_count","sha256"],"properties":{
              "episode_id":{"type":"string","minLength":1,"maxLength":128},
              "record_count":{"type":"integer","minimum":0},
              "sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}},
        "split_policy_id":{"type":"string","minLength":1,"maxLength":64},
        "label_policy_id":{"type":"string","minLength":1,"maxLength":64}}})
    }
    "dataset_manifest_reference@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
      "required":["run_id","manifest_id","episode_count","sha256"],"properties":{
        "run_id":{"type":"string","minLength":1,"maxLength":128},
        "manifest_id":{"type":"string","minLength":1,"maxLength":128},
        "episode_count":{"type":"integer","minimum":1},
        "sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}})
    }
    "sample_request@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["source_time_ns","sequence","frame_id","clock"],"properties":{
          "source_time_ns":{"type":"integer","minimum":0,"x-unit":"ns"},
          "sequence":{"type":"integer","minimum":1},
          "clock":{"const":"monotonic"},
          "frame_id":{"type":"string","minLength":1,"maxLength":128}}})
    }
    "movement_binding_request@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["source_time_ns","frame_id","target_node","primitive","mode","unit","joint_deltas_rad"],
        "properties":{"source_time_ns":{"type":"integer","minimum":0,"x-unit":"ns"},
          "frame_id":{"type":"string","minLength":1,"maxLength":128},
          "target_node":{"type":"string","minLength":1,"maxLength":128},
          "primitive":{"const":"arm.delta@1.0.0"},"mode":{"const":"joint_positions"},
          "unit":{"const":"rad"},"joint_deltas_rad":{"type":"array","minItems":6,"maxItems":6,
            "items":{"type":"number","minimum":-0.25,"maximum":0.25},"x-unit":"rad"}}})
    }
    "physical_event@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["run_id","source_time_ns","frame_id","source_sequence","event","state","provenance","rule_version"],
        "properties":{"run_id":{"type":"string","minLength":1,"maxLength":128},
          "source_time_ns":{"type":"integer","minimum":0,"x-unit":"ns"},
          "frame_id":{"type":"string","minLength":1,"maxLength":128},
          "source_sequence":{"type":"integer","minimum":0},
          "event":{"const":"joint_limit_proximity"},"state":{"enum":["detected","clear","unknown"]},
          "provenance":{"const":"input_joint_state"},"rule_version":{"const":"1"}}})
    }
    "device_status@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["run_id","source_time_ns","frame_id","source_sequence","state","reason"],
        "properties":{"run_id":{"type":"string","minLength":1,"maxLength":128},
          "source_time_ns":{"type":"integer","minimum":0,"x-unit":"ns"},
          "frame_id":{"type":"string","minLength":1,"maxLength":128},
          "source_sequence":{"type":"integer","minimum":0},
          "state":{"enum":["ready","degraded","inhibited"]},
          "reason":{"type":["string","null"],"maxLength":64}}})
    }
    "binding_result@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["run_id","target_node","primitive","mode","unit","valid"],
        "properties":{"run_id":{"type":"string","minLength":1,"maxLength":128},
          "target_node":{"type":"string","minLength":1,"maxLength":128},
          "primitive":{"const":"arm.delta@1.0.0"},"mode":{"const":"joint_positions"},
          "unit":{"const":"rad"},"valid":{"const":true}}})
    }
    "image_frame@v0.2" => {
      return json!({
        "type":"object","additionalProperties":false,
        "required":["sample","width_px","height_px","encoding","data"],
        "properties":{
          "sample":sample,
          "width_px":{"type":"integer","minimum":1,"x-unit":"px"},
          "height_px":{"type":"integer","minimum":1,"x-unit":"px"},
          "encoding":{"enum":["jpeg","rgb8"]},
          "data":{"type":"string","contentEncoding":"base64"}
        }
      })
    }
    "base_state@v0.2" => {
      return json!({
        "type":"object","additionalProperties":false,
        "required":["sample","x_m","y_m","yaw_rad","vx_m_s","vy_m_s","yaw_rate_rad_s"],
        "properties":{
          "sample":sample,
          "x_m":{"type":"number","x-unit":"m"},
          "y_m":{"type":"number","x-unit":"m"},
          "yaw_rad":{"type":"number","x-unit":"rad"},
          "vx_m_s":{"type":"number","x-unit":"m/s"},
          "vy_m_s":{"type":"number","x-unit":"m/s"},
          "yaw_rate_rad_s":{"type":"number","x-unit":"rad/s"}
        }
      })
    }
    "joint_state@v0.2" => {
      return json!({
        "type":"object","additionalProperties":false,
        "required":["sample","joint_names","position_rad","velocity_rad_s"],
        "properties":{
          "sample":sample,
          "joint_names":{"type":"array","minItems":1,"uniqueItems":true,"items":{"type":"string","minLength":1}},
          "position_rad":{"type":"array","minItems":1,"items":{"type":"number"},"x-unit":"rad"},
          "velocity_rad_s":{"type":"array","minItems":1,"items":{"type":"number"},"x-unit":"rad/s"}
        }
      })
    }
    "base_twist_command@v0.2" => {
      return json!({
        "type":"object","additionalProperties":false,
        "required":["frame_id","vx_m_s","vy_m_s","yaw_rate_rad_s","lease_ms"],
        "properties":{
          "frame_id":{"type":"string","minLength":1,"maxLength":128},
          "vx_m_s":{"type":"number","x-unit":"m/s"},
          "vy_m_s":{"type":"number","x-unit":"m/s"},
          "yaw_rate_rad_s":{"type":"number","x-unit":"rad/s"},
          "lease_ms":{"type":"integer","minimum":1,"x-unit":"ms"}
        }
      })
    }
    "joint_position_command@v0.2" => {
      return json!({
        "type":"object","additionalProperties":false,
        "required":["joint_names","position_rad"],
        "properties":{
          "joint_names":{"type":"array","minItems":1,"uniqueItems":true,"items":{"type":"string","minLength":1}},
          "position_rad":{"type":"array","minItems":1,"items":{"type":"number"},"x-unit":"rad"}
        }
      })
    }
    "pose_state@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["sample","position_m","orientation_xyzw"],
        "properties":{"sample":sample,
          "position_m":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"number"},"x-unit":"m"},
          "orientation_xyzw":{"type":"array","minItems":4,"maxItems":4,"items":{"type":"number"}}}})
    }
    "gripper_state@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["sample","channels"],"properties":{"sample":sample,
          "channels":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"object","additionalProperties":false,
            "required":["name","value","unit"],"properties":{"name":{"type":"string","minLength":1,"maxLength":64},
            "value":{"type":"number"},"unit":{"type":"string","minLength":1,"maxLength":32}}}}}})
    }
    "normalized_axes_command@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["axes","lease_ms"],"properties":{"axes":{"type":"object","minProperties":1,
          "maxProperties":32,"additionalProperties":{"type":"number","minimum":-1.0,"maximum":1.0}},
          "lease_ms":{"type":"integer","minimum":1,"maximum":1000,"x-unit":"ms"}}})
    }
    "gripper_binary_command@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["action"],"properties":{"action":{"enum":["open","close"]}}})
    }
    "control_mode_command@v0.2" => {
      return json!({"type":"object","additionalProperties":false,
        "required":["mode"],"properties":{"mode":{"type":"string","minLength":1,"maxLength":64}}})
    }
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

#[cfg(test)]
mod data_schema_cache_tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn reuses_only_the_exact_data_schema() {
    let numbers = json!({"type":"integer"});
    let strings = json!({"type":"string"});
    let first = compiled_data_schema(&numbers).unwrap();
    let again = compiled_data_schema(&numbers).unwrap();
    let other = compiled_data_schema(&strings).unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    assert!(!Arc::ptr_eq(&first, &other));
    assert!(first.is_valid(&json!(7)));
    assert!(!first.is_valid(&json!("7")));
    assert!(other.is_valid(&json!("7")));
  }
}
