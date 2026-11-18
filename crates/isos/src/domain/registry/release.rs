//! Versioned release metadata; downloading is deliberately separate from validation.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::{fs::File, io::Read, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
  pub contract: String,
  pub schema_version: String,
  pub id: String,
  pub kind: String,
  pub version: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub license: Option<String>,
  pub source_commit: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source_url: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub release_url: Option<String>,
  #[serde(default)]
  pub contracts: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub compatibility: Option<Compatibility>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub config_inputs: Vec<ConfigInput>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub activation: Option<Activation>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub node_contributions: Option<NodeContributions>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub package_config: Option<PackageConfigFiles>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub adapters: Vec<AdapterLibrary>,
  pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
  pub isos_min_inclusive: String,
  pub isos_max_exclusive: String,
  #[serde(default)]
  pub required_owner_capabilities: Vec<OwnerCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerCapability {
  pub owner: String,
  pub capability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterLibrary {
  pub family: String,
  pub library: String,
  pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeContributions {
  #[serde(default)]
  pub node_types: Vec<Value>,
  #[serde(default)]
  pub libraries: Vec<NodeLibrary>,
  #[serde(default)]
  pub port_routes: Vec<Value>,
  #[serde(default)]
  pub bridge_adapters: Vec<Value>,
  #[serde(default)]
  pub safety_adapters: Vec<Value>,
  #[serde(default)]
  pub adapter_families: Vec<Value>,
  #[serde(default)]
  pub data_types: Vec<DataType>,
  #[serde(default)]
  pub node_sets: Vec<NodeSet>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub processes: Vec<ProcessDeclaration>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub runtime_services: Vec<RuntimeService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageConfigFiles {
  pub descriptor: String,
  pub descriptor_sha256: String,
  pub template: String,
  pub template_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeService {
  pub id: String,
  pub entrypoint: String,
  #[serde(default)]
  pub args: Vec<String>,
  pub config_service_type: String,
  pub node_types: Vec<String>,
  #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
  pub required_node_settings: std::collections::BTreeMap<String, Vec<String>>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub resources: Vec<ProcessResource>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub environment: Vec<ServiceEnvironment>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub requirements: Vec<ServiceRequirement>,
}

impl RuntimeService {
  pub fn check_selected_node_settings(&self, node: &Value) -> Result<()> {
    let Some(node_type) = node["node_type"].as_str() else {
      return Ok(());
    };
    let Some(fields) = self.required_node_settings.get(node_type) else {
      return Ok(());
    };
    let settings = node["settings"]
      .as_object()
      .context("selected service node lacks settings")?;
    for field in fields {
      ensure!(
        settings.get(field).is_some_and(|value| !value.is_null()),
        "selected service node lacks required setting {field}"
      );
    }
    Ok(())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceEnvironment {
  pub name: String,
  pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceRequirement {
  pub path: String,
  pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeLibrary {
  pub path: String,
  pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessDeclaration {
  pub node_type: String,
  pub entrypoint: String,
  pub lifecycle_adapter: String,
  #[serde(default)]
  pub args: Vec<String>,
  pub resources: Vec<ProcessResource>,
  #[serde(default)]
  pub host_inputs: Vec<HostInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessResource {
  pub setting: String,
  pub kind: String,
  pub descriptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostInput {
  pub name: String,
  pub kind: String,
  pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataType {
  pub id: String,
  pub version: String,
  pub purpose: String,
  pub media_type: String,
  pub codec: String,
  pub schema: Value,
  pub max_payload_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSet {
  pub id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub label: Option<crate::domain::packages::package_config::Text>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<crate::domain::packages::package_config::Text>,
  #[serde(default)]
  pub parameters: BTreeMap<String, NodeParameter>,
  pub nodes: Vec<NodeTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeParameter {
  #[serde(rename = "type")]
  pub value_type: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeTemplate {
  #[serde(rename = "ref")]
  pub reference: String,
  pub core: String,
  pub layer: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub group: Option<String>,
  pub profile: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
  pub files: Vec<ActivationFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationFile {
  pub source: String,
  pub destination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigInput {
  pub name: String,
  #[serde(rename = "type")]
  pub input_type: String,
  pub choices: Vec<String>,
  pub default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  pub os: String,
  pub arch: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub libc: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub gpu: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub ros: Option<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub required_libraries: Vec<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub tested_ubuntu: Vec<String>,
  pub url: String,
  pub size_bytes: u64,
  pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct Platform {
  pub os: String,
  pub arch: String,
  pub libc: Option<String>,
  pub gpu: Option<String>,
  pub ros: Option<String>,
}

fn valid_slug(s: &str) -> bool {
  !s.is_empty()
    && s
      .bytes()
      .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn safe_package_path(path: &str) -> bool {
  !path.is_empty()
    && !path.starts_with('/')
    && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

fn valid_data_type_identity(data_type: &DataType) -> bool {
  let valid_version = data_type
    .version
    .split_once('.')
    .is_some_and(|(major, minor)| {
      !major.is_empty()
        && !minor.is_empty()
        && major.bytes().all(|b| b.is_ascii_digit())
        && minor.bytes().all(|b| b.is_ascii_digit())
    });
  valid_version
    && data_type.id.ends_with(&format!("@v{}", data_type.version))
    && data_type.id.split_once("@v").is_some_and(|(prefix, _)| {
      prefix.contains('.') && !prefix.starts_with('.') && !prefix.ends_with('.')
    })
}

fn numeric_version(s: &str) -> Option<Vec<u32>> {
  let values: Option<Vec<u32>> = s.split('.').map(|v| v.parse().ok()).collect();
  values.filter(|v| !v.is_empty())
}

impl Release {
  pub fn validate(&self) -> Result<()> {
    let support = crate::domain::version::bundled()?;
    ensure!(
      self.contract == support.package_contract
        && (self.schema_version == support.package_schema
          || self.schema_version == "0.2.0"
          || self.schema_version == "0.1.0"),
      "unsupported package contract"
    );
    ensure!(
      self.node_contributions.is_some(),
      "unsupported_package_contract: node_contributions required"
    );
    ensure!(
      self
        .id
        .split_once('/')
        .is_some_and(|(a, b)| valid_slug(a) && valid_slug(b)),
      "release id must be namespace/name"
    );
    ensure!(
      self.kind.len() <= 48 && valid_slug(&self.kind),
      "release kind must be a bounded stable identifier"
    );
    ensure!(
      self.version.split('.').count() >= 3
        && self.version.starts_with(|c: char| c.is_ascii_digit()),
      "invalid release version"
    );
    ensure!(
      self.source_commit.len() == 40 && self.source_commit.bytes().all(|b| b.is_ascii_hexdigit()),
      "source_commit must be a full Git commit SHA"
    );
    if let Some(compatibility) = &self.compatibility {
      ensure!(
        self.schema_version == support.package_schema || self.schema_version == "0.2.0",
        "compatibility requires current package schema"
      );
      let minimum = semver::Version::parse(&compatibility.isos_min_inclusive)?;
      let maximum = semver::Version::parse(&compatibility.isos_max_exclusive)?;
      let current = semver::Version::parse(&support.product_version)?;
      ensure!(minimum < maximum, "invalid IsOS compatibility range");
      ensure!(
        current >= minimum && current < maximum,
        "incompatible IsOS version"
      );
      let mut capabilities = std::collections::BTreeSet::new();
      for required in &compatibility.required_owner_capabilities {
        ensure!(
          matches!(required.owner.as_str(), "isos" | "ei" | "eos")
            && (1..=64).contains(&required.capability.len())
            && required
              .capability
              .bytes()
              .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && capabilities.insert((&required.owner, &required.capability)),
          "invalid or duplicate required owner capability"
        );
      }
    }
    for url in [&self.source_url, &self.release_url].into_iter().flatten() {
      ensure!(
        url.starts_with("https://github.com/"),
        "release source URLs must use GitHub HTTPS"
      );
    }
    if let Some(config) = &self.package_config {
      ensure!(
        self.schema_version == "0.2.1",
        "package_config requires schema 0.2.1"
      );
      for (path, hash, suffix) in [
        (&config.descriptor, &config.descriptor_sha256, ".json"),
        (&config.template, &config.template_sha256, ".toml"),
      ] {
        ensure!(
          safe_package_path(path) && path.ends_with(suffix),
          "invalid package config path"
        );
        ensure!(
          hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
          "invalid package config digest"
        );
      }
      ensure!(
        config.descriptor != config.template,
        "package config files must differ"
      );
    }
    ensure!(
      !self.assets.is_empty(),
      "release requires at least one asset"
    );
    let mut adapter_families = std::collections::BTreeSet::new();
    for adapter in &self.adapters {
      ensure!(
        !adapter.family.is_empty() && adapter_families.insert(&adapter.family),
        "duplicate or empty adapter family"
      );
      ensure!(
        adapter.library.ends_with(".so")
          && !adapter.library.starts_with('/')
          && adapter
            .library
            .split('/')
            .all(|part| !matches!(part, "" | "." | "..")),
        "adapter library must be a safe package-relative .so path"
      );
      ensure!(
        adapter.sha256.len() == 64 && adapter.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "adapter library sha256 must be 64 hex characters"
      );
    }
    if let Some(activation) = &self.activation {
      ensure!(!activation.files.is_empty(), "activation requires files");
      let mut destinations = std::collections::BTreeSet::new();
      for file in &activation.files {
        for path in [&file.source, &file.destination] {
          ensure!(
            !path.is_empty()
              && path.split('/').all(|p| !matches!(p, "" | "." | ".."))
              && !path.starts_with('/'),
            "activation paths must be safe relative paths"
          );
        }
        ensure!(
          file.destination.ends_with(".toml"),
          "activation destination must be TOML"
        );
        ensure!(
          destinations.insert(&file.destination),
          "duplicate activation destination"
        );
      }
    }
    if let Some(contributions) = &self.node_contributions {
      let mut service_ids = std::collections::BTreeSet::new();
      for service in &contributions.runtime_services {
        ensure!(
          self.schema_version == "0.2.1" && self.package_config.is_some(),
          "runtime services require package config schema 0.2.1"
        );
        ensure!(
          valid_slug(&service.id) && service_ids.insert(&service.id),
          "invalid or duplicate runtime service ID"
        );
        ensure!(
          valid_slug(&service.config_service_type),
          "invalid runtime service config type"
        );
        ensure!(
          safe_package_path(&service.entrypoint),
          "invalid runtime service entrypoint"
        );
        ensure!(
          !service.node_types.is_empty(),
          "runtime service requires node types"
        );
        ensure!(
          service.args.len() <= 16
            && service.args.iter().all(|arg| !arg.is_empty()
              && arg.len() <= 256
              && !arg.contains('\0')
              && (!arg.contains("${")
                || arg == "${socket}"
                || service
                  .resources
                  .iter()
                  .any(|resource| arg == &format!("${{resource.{}.path}}", resource.setting)))),
          "invalid runtime service arguments"
        );
        ensure!(
          service
            .resources
            .iter()
            .all(|resource| valid_slug(&resource.setting)
              && valid_slug(&resource.kind)
              && safe_package_path(&resource.descriptor)),
          "invalid runtime service resource"
        );
        let mut environment_names = std::collections::BTreeSet::new();
        for variable in &service.environment {
          ensure!(
            variable.name.len() <= 64
              && variable
                .name
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
              && !matches!(
                variable.name.as_str(),
                "LD_PRELOAD" | "PATH" | "HOME" | "ISOS_RUN_ID" | "ISOS_GRAPH_REVISION"
              )
              && environment_names.insert(&variable.name),
            "invalid runtime service environment name"
          );
          ensure!(
            variable.value.len() <= 1024 && !variable.value.contains('\0'),
            "invalid runtime service environment value"
          );
        }
        ensure!(
          service.requirements.len() <= 16,
          "too many runtime service requirements"
        );
        for requirement in &service.requirements {
          ensure!(
            matches!(requirement.kind.as_str(), "file" | "directory")
              && requirement.path.len() <= 1024
              && (requirement.path.starts_with("${config.host.")
                || requirement.path.starts_with("${config.service."))
              && !requirement.path.contains('\0'),
            "invalid runtime service requirement"
          );
        }
        for node_type in &service.node_types {
          ensure!(
            contributions
              .node_types
              .iter()
              .any(|value| value["id"] == *node_type)
              || (node_type.starts_with("eos.hardware.") && node_type.ends_with("@v0.2")),
            "runtime service node type is neither contributed nor a historical EOS hardware type: {node_type}"
          );
        }
        for (node_type, fields) in &service.required_node_settings {
          ensure!(
            service.node_types.contains(node_type),
            "required node settings refer to an undeclared service node type"
          );
          let definition = contributions
            .node_types
            .iter()
            .find(|value| value["id"] == *node_type)
            .context("required node settings need a contributed node type")?;
          let declared = definition["settings"]
            .as_array()
            .context("required node settings need node setting definitions")?;
          let mut unique = std::collections::BTreeSet::new();
          ensure!(
            !fields.is_empty() && fields.len() <= 32,
            "invalid required node setting list"
          );
          for field in fields {
            ensure!(
              !field.is_empty()
                && field.len() <= 64
                && field.bytes().all(|byte| byte.is_ascii_lowercase()
                  || byte.is_ascii_digit()
                  || byte == b'_'
                  || byte == b'-')
                && unique.insert(field),
              "invalid or duplicate required node setting"
            );
            ensure!(
              declared.iter().any(|setting| setting["name"] == *field),
              "required node setting is not declared by node type"
            );
          }
        }
      }
      let mut libraries = std::collections::BTreeSet::<String>::new();
      for library in &contributions.libraries {
        ensure!(
          safe_package_path(&library.path)
            && library.path.ends_with(".so")
            && libraries.insert(library.path.clone())
            && library.sha256.len() == 64
            && library.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()),
          "invalid or duplicate node library {}",
          library.path
        );
      }
      let mut data_ids = std::collections::BTreeSet::new();
      for data_type in &contributions.data_types {
        ensure!(
          valid_data_type_identity(data_type) && data_ids.insert(&data_type.id),
          "invalid or duplicate data type {}",
          data_type.id
        );
        ensure!(
          !data_type.purpose.trim().is_empty(),
          "{}: purpose is required",
          data_type.id
        );
        ensure!(
          !data_type.media_type.trim().is_empty(),
          "{}: media_type is required",
          data_type.id
        );
        ensure!(
          !data_type.codec.trim().is_empty(),
          "{}: codec is required",
          data_type.id
        );
        ensure!(
          data_type.codec != "json_v1" || data_type.media_type == "application/json",
          "{}: json_v1 requires application/json",
          data_type.id
        );
        ensure!(
          data_type.schema.is_object(),
          "{}: schema must be an object",
          data_type.id
        );
        ensure!(
          jsonschema::validator_for(&data_type.schema).is_ok(),
          "{}: schema is invalid",
          data_type.id
        );
        ensure!(
          (1..=100_000_000).contains(&data_type.max_payload_bytes),
          "{}: max_payload_bytes must be 1..100000000",
          data_type.id
        );
      }
      let mut type_ids = std::collections::BTreeSet::new();
      for node_type in &contributions.node_types {
        let id = node_type
          .get("id")
          .and_then(Value::as_str)
          .ok_or_else(|| anyhow::anyhow!("contributed node type requires id"))?;
        ensure!(
          ["isos.", "ei.", "eos."]
            .iter()
            .any(|prefix| id.starts_with(prefix))
            && id.ends_with("@v0.2")
            && type_ids.insert(id),
          "invalid or duplicate contributed node type {id}"
        );
        for field in [
          "core",
          "layer",
          "group",
          "kind",
          "lifecycle",
          "capabilities",
          "settings",
          "inputs",
          "outputs",
        ] {
          ensure!(
            node_type.get(field).is_some(),
            "{id}: node type requires {field}"
          );
        }
        super::node_schema::validate_type(node_type)?;
        ensure!(
          node_type["core"]
            .as_str()
            .is_some_and(|core| id.starts_with(&format!("{core}."))),
          "{id}: type ID and core mismatch"
        );
        match node_type["implementation"]["kind"].as_str() {
          Some("native_plugin") => ensure!(
            node_type["implementation"]["library"]
              .as_str()
              .is_some_and(|path| libraries.contains(path)),
            "{id}: native library is not declared in node_contributions.libraries"
          ),
          Some("external") | Some("managed_process") => (),
          _ if self.schema_version == "0.1.0" => (),
          _ => anyhow::bail!(
            "{id}: contributed node requires native_plugin or external implementation"
          ),
        }
        for port in ["inputs", "outputs"]
          .into_iter()
          .flat_map(|key| node_type[key].as_array().into_iter().flatten())
        {
          ensure!(
            port.get("expected_rate_hz").is_none(),
            "{id}: expected_rate_hz was replaced by frequency_hz; migrate the package declaration"
          );
          if let Some(rate) = port.get("frequency_hz") {
            ensure!(
              rate
                .as_f64()
                .is_some_and(|value| value.is_finite() && value > 0.0),
              "{id}: frequency_hz must be positive"
            );
          }
          if let Some(transport) = port.get("transport").and_then(Value::as_str) {
            ensure!(
              matches!(transport, "none" | "in_process" | "unix_socket"),
              "{id}: invalid port transport"
            );
          }
          if let Some(class) = port.get("data_class").and_then(Value::as_str) {
            ensure!(
              matches!(
                class,
                "image" | "observation" | "command" | "event" | "state" | "opaque"
              ),
              "{id}: invalid port data_class"
            );
          }
        }
      }
      let mut route_ids = std::collections::BTreeSet::new();
      for route in &contributions.port_routes {
        let source = route["source_type"].as_str().unwrap_or("");
        let target = route["target_type"].as_str().unwrap_or("");
        let output = route["output"].as_str().unwrap_or("");
        let input = route["input"].as_str().unwrap_or("");
        ensure!(
          !source.is_empty()
            && !target.is_empty()
            && !output.is_empty()
            && !input.is_empty()
            && ["in_process", "unix_socket"].contains(&route["transport"].as_str().unwrap_or(""))
            && route["delivery_mode"]
              .as_str()
              .is_some_and(|s| !s.is_empty())
            && route["data_type"].as_str().is_some_and(|s| !s.is_empty())
            && route["contract"]["owner"]
              .as_str()
              .is_some_and(|s| !s.is_empty())
            && route["contract"]["id"]
              .as_str()
              .is_some_and(|s| !s.is_empty())
            && route["contract"]["version"]
              .as_str()
              .is_some_and(|s| !s.is_empty())
            && route_ids.insert((source, output, target, input)),
          "invalid or duplicate contributed route"
        );
      }
      let mut bridges = std::collections::BTreeSet::new();
      for bridge in &contributions.bridge_adapters {
        let core = bridge["core"].as_str().unwrap_or("");
        let node_type = bridge["node_type"].as_str().unwrap_or("");
        ensure!(
          ["isos", "ei", "eos"].contains(&core)
            && bridge["adapter"] == "unix_socket"
            && node_type.starts_with(&format!("{core}."))
            && bridges.insert((core, node_type)),
          "invalid or duplicate contributed bridge"
        );
      }
      let mut process_types = std::collections::BTreeSet::new();
      for process in &contributions.processes {
        let node_type = contributions
          .node_types
          .iter()
          .find(|node_type| node_type["id"] == process.node_type)
          .ok_or_else(|| anyhow::anyhow!("process declaration lacks a contributed node type"))?;
        let owner = node_type["core"].as_str().unwrap_or("");
        let current = ["isos", "ei", "eos"].contains(&owner)
          && node_type["layer"] == "process"
          && process.node_type.starts_with(&format!("{owner}.process."))
          && process.node_type.ends_with("@v0.2")
          && node_type["implementation"]["kind"] == "managed_process";
        let historical = (self.schema_version == "0.1.0"
          && (node_type["implementation"].is_null()
            || node_type["implementation"]["kind"] == "external"))
          || (self.version.starts_with("0.1.")
            && node_type["implementation"]["kind"] == "managed_process"
            && node_type["core"] == "isos"
            && node_type["layer"] == "eos_process");
        ensure!(
          process_types.insert(&process.node_type)
            && (current || historical)
            && node_type["lifecycle"] == "process",
          "process declaration requires a unique registered process node type and owner"
        );
        ensure!(
          safe_package_path(&process.entrypoint)
            && !process.lifecycle_adapter.is_empty()
            && process.lifecycle_adapter.len() <= 64
            && process.lifecycle_adapter.bytes().all(|byte| {
              byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_' | b'.')
            })
            && process.resources.len() <= 32,
          "process entrypoint or lifecycle adapter invalid"
        );
        let mut settings = std::collections::BTreeSet::new();
        ensure!(
          process.args.len() <= 16
            && process.args.iter().all(|arg| {
              !arg.is_empty()
                && arg.len() <= 256
                && !arg.contains('\0')
                && (!arg.contains("${")
                  || arg == "${socket}"
                  || arg
                    .strip_prefix("${resource.")
                    .and_then(|rest| rest.strip_suffix(".path}"))
                    .is_some_and(|name| {
                      process
                        .resources
                        .iter()
                        .any(|resource| resource.setting == name)
                    }))
            }),
          "process arguments invalid"
        );
        for resource in &process.resources {
          ensure!(
            !resource.setting.is_empty()
              && settings.insert(&resource.setting)
              && !resource.kind.is_empty()
              && safe_package_path(&resource.descriptor),
            "process resource declaration invalid"
          );
        }
        let mut inputs = std::collections::BTreeSet::new();
        for input in &process.host_inputs {
          ensure!(
            !input.name.is_empty()
              && input
                .name
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
              && inputs.insert(&input.name)
              && input.kind == "directory",
            "process host input invalid"
          );
        }
      }
      let mut adapter_ids = std::collections::BTreeSet::new();
      for adapter in &contributions.safety_adapters {
        let id = adapter
          .get("id")
          .and_then(Value::as_str)
          .ok_or_else(|| anyhow::anyhow!("safety adapter requires id"))?;
        ensure!(adapter_ids.insert(id), "duplicate safety adapter {id}");
        ensure!(
          adapter.get("supports_stop") == Some(&Value::Bool(true)),
          "{id}: safety adapter must support stop"
        );
      }
      let mut family_ids = std::collections::BTreeSet::new();
      for family in &contributions.adapter_families {
        let id = family
          .get("id")
          .and_then(Value::as_str)
          .ok_or_else(|| anyhow::anyhow!("adapter family requires id"))?;
        ensure!(
          !id.trim().is_empty() && family_ids.insert(id),
          "invalid or duplicate adapter family {id}"
        );
        ensure!(
          family
            .get("device_kinds")
            .and_then(Value::as_array)
            .is_some_and(|kinds| !kinds.is_empty()),
          "{id}: device_kinds must be a nonempty array"
        );
        ensure!(
          family
            .get("transport")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty()),
          "{id}: transport is required"
        );
        ensure!(
          family
            .get("description")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty()),
          "{id}: description is required"
        );
      }
      let mut set_ids = std::collections::BTreeSet::new();
      for set in &contributions.node_sets {
        ensure!(
          valid_slug(&set.id) && set_ids.insert(&set.id),
          "invalid or duplicate node set {}",
          set.id
        );
        ensure!(!set.nodes.is_empty(), "node set {} requires nodes", set.id);
        for (name, parameter) in &set.parameters {
          ensure!(
            !name.is_empty()
              && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.'),
            "invalid node set parameter {name}"
          );
          ensure!(
            parameter.value_type == "string",
            "{name}: only string parameter is supported"
          );
        }
        let mut refs = std::collections::BTreeSet::new();
        for node in &set.nodes {
          ensure!(
            valid_slug(&node.reference) && refs.insert(&node.reference),
            "invalid or duplicate node ref {}",
            node.reference
          );
          ensure!(
            valid_slug(&node.core) && valid_slug(&node.layer),
            "invalid node placement"
          );
          ensure!(
            node.group.as_deref().is_none_or(valid_slug),
            "invalid node group"
          );
          ensure!(node.profile.is_object(), "node profile must be an object");
          ensure!(
            node
              .profile
              .get("id")
              .and_then(Value::as_str)
              .is_some_and(valid_slug),
            "node profile requires slug id"
          );
          ensure!(
            node
              .profile
              .get("node_type")
              .and_then(Value::as_str)
              .is_some(),
            "node profile requires node_type"
          );
        }
      }
    }
    for a in &self.assets {
      ensure!(
        !a.os.is_empty() && !a.arch.is_empty(),
        "asset requires os and arch"
      );
      ensure!(
        a.url.starts_with("https://github.com/") && a.url.contains("/releases/download/"),
        "asset URL must be a GitHub Release HTTPS download URL"
      );
      ensure!(
        a.size_bytes > 0 && a.size_bytes <= 2_000_000_000,
        "asset size must be 1..2000000000 bytes"
      );
      let hash = a.sha256.strip_prefix("sha256:").unwrap_or(&a.sha256);
      ensure!(
        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "asset sha256 must be 64 hexadecimal digits"
      );
      if let Some(libc) = &a.libc {
        ensure!(
          libc
            .strip_prefix("glibc>=")
            .and_then(numeric_version)
            .is_some(),
          "asset libc must use glibc>=major.minor"
        );
      }
      ensure!(
        a.required_libraries
          .iter()
          .all(|name| !name.is_empty() && !name.contains('/')),
        "required_libraries must contain library names"
      );
    }
    Ok(())
  }

  pub fn select_asset<'a>(&'a self, host: &Platform) -> Result<&'a Asset> {
    self.validate()?;
    self
      .assets
      .iter()
      .find(|a| {
        a.os == host.os
          && a.arch == host.arch
          && a.gpu.as_ref().is_none_or(|v| host.gpu.as_ref() == Some(v))
          && a.ros.as_ref().is_none_or(|v| host.ros.as_ref() == Some(v))
          && {
            let need = a
              .libc
              .as_deref()
              .and_then(|v| v.strip_prefix("glibc>="))
              .and_then(numeric_version);
            let have = host.libc.as_deref().and_then(numeric_version);
            need.is_none_or(|n| matches!(have, Some(h) if h >= n))
          }
      })
      .ok_or_else(|| {
        anyhow::anyhow!(
          "no asset for {} {} (libc {:?}, gpu {:?}, ros {:?})",
          host.os,
          host.arch,
          host.libc,
          host.gpu,
          host.ros
        )
      })
  }
}

impl Platform {
  pub fn current() -> Self {
    let arch = match std::env::consts::ARCH {
      "x86_64" => "x86_64",
      "aarch64" => "aarch64",
      other => other,
    }
    .to_string();
    let libc = if cfg!(target_os = "linux") {
      // glibc exposes this as a static NUL-terminated string.
      unsafe {
        let ptr = libc::gnu_get_libc_version();
        if ptr.is_null() {
          None
        } else {
          Some(std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
      }
    } else {
      None
    };
    Self {
      os: std::env::consts::OS.into(),
      arch,
      libc,
      gpu: std::env::var("ISOS_GPU").ok(),
      ros: std::env::var("ISOS_ROS").ok(),
    }
  }
}

pub fn verify_file(asset: &Asset, path: &Path) -> Result<()> {
  let mut file = File::open(path).with_context(|| format!("cannot read {}", path.display()))?;
  ensure!(
    file.metadata()?.len() == asset.size_bytes,
    "asset size mismatch: {}",
    path.display()
  );
  let mut hasher = Sha256::new();
  let mut buffer = [0_u8; 1024 * 1024];
  loop {
    let size = file.read(&mut buffer)?;
    if size == 0 {
      break;
    }
    hasher.update(&buffer[..size]);
  }
  let expected = asset
    .sha256
    .strip_prefix("sha256:")
    .unwrap_or(&asset.sha256);
  ensure!(
    hex::encode(hasher.finalize()).eq_ignore_ascii_case(expected),
    "asset SHA-256 mismatch: {}",
    path.display()
  );
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn managed_process_admission_is_general_across_owners() {
    for owner in ["isos", "ei", "eos"] {
      let id = format!("{owner}.process.mock.service@v0.2");
      let value = serde_json::json!({
        "contract":"isos.package@v1","schema_version":"0.2.0","id":"vendor/mock",
        "kind":"backend","version":"0.2.0","source_commit":"a".repeat(40),
        "assets":[{"os":"linux","arch":"x86_64","url":"https://github.com/vendor/mock/releases/download/v0.2.0/mock.tar.gz",
          "size_bytes":1,"sha256":"b".repeat(64)}],
        "node_contributions":{
          "node_types":[{"id":id,"core":owner,"layer":"process","group":"mock","kind":"software",
            "description":"Mock process","lifecycle":"process","capabilities":["process"],"settings":[],"inputs":[],"outputs":[],
            "implementation":{"kind":"managed_process","interface":"mock-v1","version":"1","lifecycle_adapter":"mock-v1"}}],
          "processes":[{"node_type":id,"entrypoint":"bin/mock","lifecycle_adapter":"mock-v1",
            "args":["--socket","${socket}"],"resources":[]}]
        }
      });
      serde_json::from_value::<Release>(value)
        .unwrap()
        .validate()
        .unwrap();
    }
  }

  #[test]
  fn current_package_accepts_ei_native_type_only_with_declared_linux_library() {
    let mut value = serde_json::json!({
      "contract":"isos.package@v1", "schema_version":"0.2.0",
      "id":"vendor/custom", "kind":"component", "version":"1.0.0",
      "source_commit":"a".repeat(40),
      "assets":[{"os":"linux","arch":"x86_64","url":"https://github.com/vendor/custom/releases/download/v1.0.0/custom.tar.gz","size_bytes":1,"sha256":"b".repeat(64)}],
      "node_contributions":{
        "libraries":[{"path":"lib/custom.so","sha256":"c".repeat(64)}],
        "node_types":[{"id":"ei.cognition.custom.filter@v0.2","core":"ei","layer":"cognition","group":"custom",
          "kind":"software","description":"Community filter","lifecycle":"internal","capabilities":[],"settings":[],"inputs":[],"outputs":[],
          "implementation":{"kind":"native_plugin","abi":"isos_node_v1","library":"lib/custom.so"}}]
      }
    });
    let release: Release = serde_json::from_value(value.clone()).unwrap();
    release.validate().unwrap();
    value["node_contributions"]["libraries"] = serde_json::json!([]);
    assert!(
      serde_json::from_value::<Release>(value.clone())
        .unwrap()
        .validate()
        .is_err()
    );
    value["node_contributions"]["libraries"] =
      serde_json::json!([{"path":"lib/custom.dll","sha256":"c".repeat(64)}]);
    assert!(
      serde_json::from_value::<Release>(value)
        .unwrap()
        .validate()
        .is_err()
    );
  }

  #[test]
  fn compatibility_is_bounded_and_owner_neutral() {
    let mut value = serde_json::json!({
      "contract":"isos.package@v1", "schema_version":"0.2.0",
      "id":"vendor/shared", "kind":"component", "version":"1.2.3",
      "source_commit":"a".repeat(40),
      "compatibility":{"isos_min_inclusive":"0.2.0","isos_max_exclusive":"0.3.0",
        "required_owner_capabilities":[{"owner":"isos","capability":"package-process-v1"},
          {"owner":"ei","capability":"native-plugin-v1"},
          {"owner":"eos","capability":"native-plugin-v1"}]},
      "node_contributions":{},
      "assets":[{"os":"linux","arch":"x86_64","url":"https://github.com/vendor/shared/releases/download/v1.2.3/shared.tar.gz",
        "size_bytes":1,"sha256":"b".repeat(64)}]
    });
    serde_json::from_value::<Release>(value.clone())
      .unwrap()
      .validate()
      .unwrap();
    value["compatibility"]["isos_max_exclusive"] = serde_json::json!("0.2.3");
    assert!(
      serde_json::from_value::<Release>(value.clone())
        .unwrap()
        .validate()
        .is_err()
    );
    value["compatibility"]["isos_max_exclusive"] = serde_json::json!("0.3.0");
    value["compatibility"]["required_owner_capabilities"][1]["owner"] =
      serde_json::json!("unknown");
    assert!(
      serde_json::from_value::<Release>(value.clone())
        .unwrap()
        .validate()
        .is_err()
    );
    value["compatibility"]["required_owner_capabilities"][1]["owner"] = serde_json::json!("eos");
    assert!(
      serde_json::from_value::<Release>(value)
        .unwrap()
        .validate()
        .is_err()
    );
  }

  #[test]
  fn unrelated_package_families_share_registered_type_validation() {
    for (id, type_id, resource_kind) in [
      (
        "field/sensor-rig",
        "isos.process.capture@v0.2",
        "calibration",
      ),
      ("studio/synth-bank", "isos.process.render@v0.2", "preset"),
    ] {
      let release = serde_json::json!({
        "contract":"isos.package@v1", "schema_version":"0.1.0",
        "id":id, "kind":"component", "version":"1.2.3",
        "source_commit":"a".repeat(40),
        "assets":[{"os":"linux","arch":"x86_64",
          "url":"https://github.com/vendor/package/releases/download/v1.2.3/package.tar.gz",
          "size_bytes":1,"sha256":"b".repeat(64)}],
        "node_contributions":{
          "node_types":[{"id":type_id,"core":"isos","layer":"process","group":"component",
            "kind":"software","description":"Verified package process",
            "lifecycle":"process","capabilities":[],
            "settings":[{"name":"selection","data_type":"string","required":true,
              "description":"Verified resource ID","selector":{"provider":"package",
                "resource":"selection","package_from":"node.package"}}],
            "inputs":[],"outputs":[{"name":"result","data_type":"generic.record@v1.0","required":true,
              "description":"Bounded result record",
              "transport":"in_process","delivery_mode":"message","cardinality":"one",
              "contract":{"owner":"isos","id":"generic-record","version":"1.0"}}]}],
          "data_types":[{"id":"generic.record@v1.0","version":"1.0",
            "purpose":"Bounded record","media_type":"application/json","codec":"json_v1",
            "schema":{"type":"object"},"max_payload_bytes":4096}],
          "processes":[{"node_type":type_id,"entrypoint":"bin/worker",
            "lifecycle_adapter":"process-v1",
            "resources":[{"setting":"selection","kind":resource_kind,
              "descriptor":"resources/index.json"}]}]
        }
      });
      let accepted: Release = serde_json::from_value(release.clone()).unwrap();
      accepted.validate().unwrap();

      let mut malformed = release.clone();
      malformed["node_contributions"]["node_types"][0]["settings"][0]["data_type"] =
        serde_json::json!("vendor_only_value");
      let rejected: Release = serde_json::from_value(malformed).unwrap();
      assert!(
        rejected.validate().is_err(),
        "{id} must reject unknown setting kinds"
      );

      let mut unsafe_entrypoint = release;
      unsafe_entrypoint["node_contributions"]["processes"][0]["entrypoint"] =
        serde_json::json!("../outside");
      let rejected: Release = serde_json::from_value(unsafe_entrypoint).unwrap();
      assert!(
        rejected.validate().is_err(),
        "{id} must reject escaping paths"
      );
    }
  }

  #[test]
  fn adapter_metadata_is_independent_of_package_kind_and_rejects_unsafe_libraries() {
    let mut release: Release = serde_json::from_value(serde_json::json!({
            "contract":"isos.package@v1","schema_version":"0.1.0", "id": "vendor/devices", "kind": "backend", "version": "2.0.0",
            "source_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "node_contributions":{}, "adapters": [{"family": "vendor-base", "library": "lib/base.so", "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}],
            "assets": [{"os": "linux", "arch": "x86_64", "url": "https://github.com/vendor/devices/releases/download/v2.0.0/devices.tar.gz", "size_bytes": 1, "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}]
        })).unwrap();
    release.validate().unwrap();
    release.kind = "service".into();
    release.validate().unwrap();
    release.kind = "synthetic-family".into();
    release.validate().unwrap();
    release.kind = "bad/type".into();
    assert!(release.validate().is_err());
    release.kind = "backend".into();
    release.adapters[0].library = "../base.so".into();
    assert!(release.validate().is_err());
    release.adapters[0].library = "lib/base.so".into();
    release.adapters.push(release.adapters[0].clone());
    assert!(release.validate().is_err());
  }

  #[test]
  fn adapter_family_requires_device_class_and_transport() {
    let mut release: Release = serde_json::from_value(serde_json::json!({
            "contract":"isos.package@v1","schema_version":"0.1.0","id":"island/test","kind":"backend","version":"2.0.0",
            "source_commit":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "assets":[{"os":"linux","arch":"x86_64","url":"https://github.com/Island-Team/test/releases/download/v2.0.0/test.tar.gz","size_bytes":1,"sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}],
            "node_contributions":{"adapter_families":[{"id":"test-v2","device_kinds":["base"],"transport":"unix_socket","description":"Test base adapter"}]}
        })).unwrap();
    release.validate().unwrap();
    release
      .node_contributions
      .as_mut()
      .unwrap()
      .adapter_families[0]["device_kinds"] = serde_json::json!([]);
    assert!(release.validate().is_err());
  }

  #[test]
  fn custom_data_idl_requires_explicit_version_and_purpose() {
    let valid = serde_json::json!({"id":"vendor.camera.frame@v1.0","version":"1.0","purpose":"Camera preview","media_type":"application/json","codec":"json_v1","schema":{"type":"object"},"max_payload_bytes":65536});
    let descriptor: DataType = serde_json::from_value(valid.clone()).unwrap();
    assert!(valid_data_type_identity(&descriptor));
    let mut mismatched = descriptor.clone();
    mismatched.version = "1.1".into();
    assert!(!valid_data_type_identity(&mismatched));
    let mut missing_version = valid.clone();
    missing_version.as_object_mut().unwrap().remove("version");
    assert!(serde_json::from_value::<DataType>(missing_version).is_err());
    let mut missing_purpose = valid;
    missing_purpose.as_object_mut().unwrap().remove("purpose");
    assert!(serde_json::from_value::<DataType>(missing_purpose).is_err());
  }

  #[test]
  fn json_data_type_requires_matching_media_type() {
    let mut release: Release = serde_json::from_value(serde_json::json!({
      "contract":"isos.package@v1","schema_version":"0.1.0","id":"island/test",
      "kind":"backend","version":"2.0.0","source_commit":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "assets":[{"os":"linux","arch":"x86_64","url":"https://github.com/Island-Team/test/releases/download/v2.0.0/test.tar.gz","size_bytes":1,"sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}],
      "node_contributions":{"data_types":[{"id":"vendor.scene.record@v1.0","version":"1.0",
        "purpose":"Scene record","media_type":"application/json","codec":"json_v1",
        "schema":{"type":"object"},"max_payload_bytes":1024}]}
    })).unwrap();
    release.validate().unwrap();
    release.node_contributions.as_mut().unwrap().data_types[0].media_type = "image/jpeg".into();
    assert!(
      release
        .validate()
        .unwrap_err()
        .to_string()
        .contains("json_v1 requires application/json")
    );
  }

  #[test]
  fn current_activation_uses_only_declared_files() {
    let release: Release = serde_json::from_str(&format!(
            r#"{{"contract":"isos.package@v1","schema_version":"0.1.0","id":"vendor/sample","kind":"backend","version":"1.0.1","source_commit":"{}","config_inputs":[],"node_contributions":{{}},"activation":{{"files":[{{"source":"config/module.toml","destination":"module.toml"}}]}},"assets":[{{"os":"linux","arch":"x86_64","libc":"glibc>=2.35","url":"https://github.com/vendor/sample/releases/download/v1.0.1/sim.tar.gz","size_bytes":10,"sha256":"{}"}}]}}"#,
            "a".repeat(40),
            "b".repeat(64)
        ))
        .unwrap();

    release.validate().unwrap();
    assert_eq!(release.activation.as_ref().unwrap().files.len(), 1);
    assert!(
      release
        .select_asset(&Platform {
          os: "linux".into(),
          arch: "x86_64".into(),
          libc: Some("2.39".into()),
          gpu: None,
          ros: None,
        })
        .is_ok()
    );
    assert!(
      release
        .select_asset(&Platform {
          os: "linux".into(),
          arch: "x86_64".into(),
          libc: Some("2.31".into()),
          gpu: None,
          ros: None,
        })
        .is_err()
    );
    let encoded = serde_json::to_value(release).unwrap();
    let activation = encoded["activation"].as_object().unwrap();
    assert_eq!(activation.len(), 1);
    assert!(activation.contains_key("files"));
  }

  #[test]
  fn legacy_catalog_fields_do_not_enter_current_release() {
    let mut release = serde_json::json!({
      "contract":"isos.package@v1", "schema_version":"0.1.0", "id":"vendor/sample",
      "kind":"component", "version":"1.0.0", "source_commit":"a".repeat(40),
      "node_contributions":{}, "assets":[{"os":"linux","arch":"x86_64",
        "url":"https://github.com/vendor/sample/releases/download/v1.0.0/bundle.tar.gz",
        "size_bytes":1,"sha256":"b".repeat(64)}],
      "activation":{"files":[{"source":"config/base.toml","destination":"base.toml"}]}
    });
    release["assets"][0]["glibc_min"] = serde_json::json!("2.35");
    assert!(serde_json::from_value::<Release>(release.clone()).is_err());
    release["assets"][0]
      .as_object_mut()
      .unwrap()
      .remove("glibc_min");
    release["activation"]["backend_template"] = serde_json::json!("legacy");
    assert!(serde_json::from_value::<Release>(release).is_err());
  }

  #[test]
  fn selects_matching_asset() {
    let release: Release = serde_json::from_str(&format!(r#"{{"contract":"isos.package@v1","schema_version":"0.1.0","id":"island/eos","kind":"component","version":"2.0.0","source_commit":"{}","node_contributions":{{}},"assets":[{{"os":"linux","arch":"x86_64","libc":"glibc>=2.35","url":"https://github.com/Island-Team/Aether-EOS-R/releases/download/v2.0.0/eos.tar.gz","size_bytes":10,"sha256":"{}"}}]}}"#, "a".repeat(40), "b".repeat(64))).unwrap();
    let host = Platform {
      os: "linux".into(),
      arch: "x86_64".into(),
      libc: Some("2.39".into()),
      gpu: None,
      ros: None,
    };
    assert!(release.select_asset(&host).is_ok());
    assert!(
      release
        .select_asset(&Platform {
          libc: Some("2.31".into()),
          ..host
        })
        .is_err()
    );
  }

  #[test]
  fn runtime_service_preserves_historical_hardware_and_requires_config_digest() {
    let mut value = serde_json::json!({
      "contract":"isos.package@v1", "schema_version":"0.2.1", "id":"vendor/robot", "kind":"component", "version":"0.2.1",
      "source_commit":"a".repeat(40),
      "assets":[{"os":"linux","arch":"x86_64","url":"https://github.com/vendor/robot/releases/download/v0.2.1/robot.tar.gz","size_bytes":1,"sha256":"b".repeat(64)}],
      "package_config":{"descriptor":"config/descriptor.json","descriptor_sha256":"c".repeat(64),"template":"config/package.toml","template_sha256":"d".repeat(64)},
      "node_contributions":{"runtime_services":[{"id":"native","entrypoint":"bin/robot","args":["${socket}"],"config_service_type":"native","node_types":["eos.hardware.camera.device@v0.2"]}]}
    });
    serde_json::from_value::<Release>(value.clone())
      .unwrap()
      .validate()
      .unwrap();
    value["node_contributions"]["libraries"] = serde_json::json!([
      {"path":"lib/camera.so","sha256":"e".repeat(64)}
    ]);
    value["node_contributions"]["node_types"] = serde_json::json!([{
      "id":"eos.hardware.camera.device@v0.2","core":"eos","layer":"hardware",
      "group":"camera","kind":"camera","lifecycle":"adapter",
      "description":"Package camera","capabilities":["jpeg"],"settings":[],
      "inputs":[],"outputs":[],"implementation":{"kind":"native_plugin",
        "abi":"isos_node_v2","library":"lib/camera.so"}
    }]);
    serde_json::from_value::<Release>(value.clone())
      .unwrap()
      .validate()
      .unwrap();
    value["node_contributions"]["node_types"][0]["settings"] = serde_json::json!([
      {"name":"camera_width","data_type":"positive_integer","required":false,"min":64,"max":2048,"description":"Camera width"}
    ]);
    value["node_contributions"]["runtime_services"][0]["required_node_settings"] =
      serde_json::json!({"eos.hardware.camera.device@v0.2":["camera_width"]});
    let with_required: Release = serde_json::from_value(value.clone()).unwrap();
    with_required.validate().unwrap();
    let service = &with_required
      .node_contributions
      .as_ref()
      .unwrap()
      .runtime_services[0];
    assert!(
      service
        .check_selected_node_settings(
          &serde_json::json!({"node_type":"eos.hardware.camera.device@v0.2","settings":{}})
        )
        .is_err()
    );
    assert!(service.check_selected_node_settings(&serde_json::json!({"node_type":"eos.hardware.camera.device@v0.2","settings":{"camera_width":1280}})).is_ok());
    value["node_contributions"]["runtime_services"][0]["required_node_settings"] =
      serde_json::json!({"eos.hardware.camera.device@v0.2":["undeclared"]});
    assert!(
      serde_json::from_value::<Release>(value.clone())
        .unwrap()
        .validate()
        .is_err()
    );
    value["node_contributions"]["runtime_services"][0]["required_node_settings"] =
      serde_json::json!({"eos.hardware.camera.device@v0.2":["camera_width"]});
    value.as_object_mut().unwrap().remove("package_config");
    assert!(
      serde_json::from_value::<Release>(value)
        .unwrap()
        .validate()
        .is_err()
    );
  }
}
