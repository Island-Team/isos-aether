//! Verified package-owned form metadata and deployment-local TOML values.
use super::{package_path, verify};
use crate::app::config::{self, Paths};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
  collections::BTreeSet,
  fs,
  io::Read,
  os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
  path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
  pub en: String,
  pub cn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
  pub value: String,
  pub label: Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
  pub name: String,
  #[serde(rename = "type")]
  pub field_type: String,
  pub required: bool,
  pub label: Text,
  pub description: Text,
  #[serde(default)]
  pub note: Option<Text>,
  #[serde(default)]
  pub default: Option<serde_json::Value>,
  #[serde(default)]
  pub min: Option<f64>,
  #[serde(default)]
  pub max: Option<f64>,
  #[serde(default)]
  pub options: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
  pub id: String,
  pub label: Text,
  pub description: Text,
  pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
  pub contract: String,
  pub schema_version: String,
  #[serde(default)]
  pub setup: Option<Setup>,
  pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
  pub service: String,
  pub entrypoint: String,
  pub prompt: Text,
  pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Saved {
  pub id: String,
  pub version: String,
  pub deployment: String,
  pub revision: String,
  pub toml: String,
  pub descriptor: Descriptor,
  pub values: toml::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExternalInput {
  pub path: String,
  pub sha256: String,
}

/// Resolve a selected service's declared local prerequisites and fingerprint files.
pub fn external_inputs(
  service: &crate::domain::registry::release::RuntimeService,
  saved: &Saved,
  instance: &str,
) -> Result<Vec<ExternalInput>> {
  let mut inputs = Vec::new();
  for requirement in &service.requirements {
    let (table, rest) = if let Some(rest) = requirement.path.strip_prefix("${config.host.") {
      (saved.values.get("host"), rest)
    } else if let Some(rest) = requirement.path.strip_prefix("${config.service.") {
      (
        saved
          .values
          .get("services")
          .and_then(|services| services.get(instance)),
        rest,
      )
    } else {
      anyhow::bail!("invalid service requirement template")
    };
    let (field, suffix) = rest
      .split_once('}')
      .context("unclosed service requirement template")?;
    ensure!(
      !field.is_empty() && !suffix.contains("${"),
      "invalid service requirement template"
    );
    let base = table
      .and_then(|value| value.get(field))
      .and_then(toml::Value::as_str)
      .filter(|value| !value.is_empty())
      .context("required local path is missing")?;
    let combined = format!("{base}{suffix}");
    ensure!(
      Path::new(&combined).is_absolute(),
      "service requirement path is not absolute"
    );
    let canonical = Path::new(&combined)
      .canonicalize()
      .context("service requirement path is missing")?;
    let metadata = fs::metadata(&canonical)?;
    ensure!(
      if requirement.kind == "file" {
        metadata.is_file()
      } else {
        metadata.is_dir()
      },
      "service requirement has wrong path kind"
    );
    let sha256 = if metadata.is_file() {
      ensure!(
        metadata.len() <= 1_073_741_824,
        "service requirement file is too large"
      );
      digest(&fs::read(&canonical)?)
    } else {
      digest(canonical.as_os_str().as_encoded_bytes())
    };
    inputs.push(ExternalInput {
      path: canonical.to_string_lossy().into_owned(),
      sha256,
    });
  }
  inputs.sort_by(|a, b| a.path.cmp(&b.path));
  Ok(inputs)
}

fn digest(bytes: &[u8]) -> String {
  hex::encode(Sha256::digest(bytes))
}

fn valid_slug(value: &str) -> bool {
  !value.is_empty()
    && value
      .bytes()
      .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn safe_deployment(value: &str) -> Result<()> {
  ensure!(valid_slug(value), "invalid deployment name");
  Ok(())
}

fn storage_path(paths: &Paths, deployment: &str, id: &str, version: &str) -> Result<PathBuf> {
  safe_deployment(deployment)?;
  let package = package_path(paths, id, version)?;
  let name = package.file_name().context("invalid package version")?;
  let package_name = package
    .parent()
    .and_then(Path::file_name)
    .context("invalid package name")?;
  let namespace = package
    .parent()
    .and_then(Path::parent)
    .and_then(Path::file_name)
    .context("invalid namespace")?;
  Ok(
    paths
      .data
      .join("state/package-config")
      .join(deployment)
      .join(namespace)
      .join(package_name)
      .join(name)
      .join("package.toml"),
  )
}

fn checked_package_file(root: &Path, relative: &str, expected: &str, max: u64) -> Result<Vec<u8>> {
  let path = root.join(relative);
  let canonical_root = root.canonicalize()?;
  let canonical = path.canonicalize()?;
  ensure!(
    canonical.starts_with(&canonical_root) && canonical.is_file(),
    "unsafe package config file"
  );
  let metadata = fs::symlink_metadata(&path)?;
  ensure!(
    metadata.file_type().is_file() && metadata.len() <= max,
    "invalid package config file"
  );
  let bytes = fs::read(canonical)?;
  ensure!(
    digest(&bytes).eq_ignore_ascii_case(expected),
    "package config digest mismatch"
  );
  Ok(bytes)
}

pub(super) fn validate_package_files(
  root: &Path,
  release: &crate::domain::registry::release::Release,
) -> Result<()> {
  if let Some(files) = &release.package_config {
    let descriptor =
      checked_package_file(root, &files.descriptor, &files.descriptor_sha256, 1_048_576)?;
    let template = checked_package_file(root, &files.template, &files.template_sha256, 1_048_576)?;
    let descriptor: Descriptor = serde_json::from_slice(&descriptor)?;
    ensure!(
      descriptor.contract == "isos.package-config@v1" && descriptor.schema_version == "0.1.0",
      "unsupported package config descriptor"
    );
    if let Some(contributions) = &release.node_contributions {
      for service in &contributions.runtime_services {
        ensure!(
          descriptor
            .sections
            .iter()
            .any(|section| section.id == format!("services.{}", service.config_service_type)),
          "runtime service config section missing"
        );
      }
    }
    validate_descriptor(&descriptor)?;
    if let Some(setup) = &descriptor.setup {
      let path = root.join(&setup.entrypoint);
      let metadata = fs::symlink_metadata(&path)?;
      ensure!(
        metadata.file_type().is_file()
          && metadata.len() <= 1_048_576
          && metadata.permissions().mode() & 0o111 != 0,
        "invalid package setup helper"
      );
      ensure!(
        path.canonicalize()?.starts_with(root.canonicalize()?),
        "unsafe package setup helper"
      );
    }
    let template = std::str::from_utf8(&template)?;
    validate(&descriptor, template, &[])?;
  }
  Ok(())
}

fn validate_descriptor(descriptor: &Descriptor) -> Result<()> {
  ensure!(
    descriptor.contract == "isos.package-config@v1" && descriptor.schema_version == "0.1.0",
    "unsupported package config descriptor"
  );
  let mut sections = BTreeSet::new();
  if let Some(setup) = &descriptor.setup {
    ensure!(valid_slug(&setup.service), "invalid package setup service");
    ensure!(
      !setup.entrypoint.starts_with('/')
        && !setup
          .entrypoint
          .split('/')
          .any(|part| part.is_empty() || part == "." || part == ".."),
      "invalid package setup entrypoint"
    );
    ensure!(
      !setup.prompt.en.trim().is_empty() && !setup.prompt.cn.trim().is_empty(),
      "missing package setup prompt"
    );
    let mut outputs = BTreeSet::new();
    ensure!(
      !setup.outputs.is_empty()
        && setup
          .outputs
          .iter()
          .all(|field| valid_slug(field) && outputs.insert(field)),
      "invalid or duplicate package setup outputs"
    );
  }
  for section in &descriptor.sections {
    ensure!(
      section.id == "package"
        || section.id == "host"
        || section.id.strip_prefix("services.").is_some_and(valid_slug),
      "invalid package config section"
    );
    ensure!(
      sections.insert(&section.id),
      "duplicate package config section"
    );
    ensure!(
      !section.label.en.is_empty() && !section.label.cn.is_empty(),
      "missing section label"
    );
    let mut fields = BTreeSet::new();
    for field in &section.fields {
      ensure!(
        valid_slug(&field.name) && fields.insert(&field.name),
        "invalid or duplicate config field"
      );
      ensure!(
        matches!(
          field.field_type.as_str(),
          "string" | "integer" | "boolean" | "directory" | "file" | "enum"
        ),
        "unsupported config field type"
      );
      ensure!(
        !field.label.en.is_empty() && !field.label.cn.is_empty(),
        "missing config field label"
      );
      ensure!(
        !field.description.en.is_empty() && !field.description.cn.is_empty(),
        "missing config field description"
      );
      ensure!(
        field.min.zip(field.max).is_none_or(|(min, max)| min <= max),
        "invalid config field range"
      );
      ensure!(
        field.min.is_none_or(f64::is_finite) && field.max.is_none_or(f64::is_finite),
        "config field bounds must be finite"
      );
      if field.field_type == "enum" {
        ensure!(
          !field.options.is_empty() && field.min.is_none() && field.max.is_none(),
          "enum field needs options and cannot have numeric bounds"
        );
        let mut options = BTreeSet::new();
        for option in &field.options {
          ensure!(
            !option.value.is_empty()
              && option.value.len() <= 256
              && !option.value.chars().any(char::is_control)
              && options.insert(&option.value)
              && !option.label.en.trim().is_empty()
              && !option.label.cn.trim().is_empty(),
            "invalid or duplicate config option"
          );
        }
      } else {
        ensure!(field.options.is_empty(), "options require enum field type");
      }
      if let Some(default) = &field.default {
        let valid = match field.field_type.as_str() {
          "string" | "directory" | "file" => default.is_string(),
          "integer" => default.as_i64().is_some_and(|number| {
            field.min.is_none_or(|min| (number as f64) >= min)
              && field.max.is_none_or(|max| (number as f64) <= max)
          }),
          "boolean" => default.is_boolean(),
          "enum" => default
            .as_str()
            .is_some_and(|value| field.options.iter().any(|option| option.value == value)),
          _ => false,
        };
        ensure!(valid, "invalid config field default");
      }
    }
  }
  if let Some(setup) = &descriptor.setup {
    ensure!(
      sections.contains(&format!("services.{}", setup.service)),
      "package setup service is not declared"
    );
    let host = descriptor
      .sections
      .iter()
      .find(|section| section.id == "host")
      .context("package setup needs host fields")?;
    for output in &setup.outputs {
      ensure!(
        host.fields.iter().any(|field| field.name == *output
          && matches!(field.field_type.as_str(), "directory" | "file")),
        "package setup output must name a host path field"
      );
    }
  }
  Ok(())
}

fn descriptor_and_template(paths: &Paths, id: &str, version: &str) -> Result<(Descriptor, String)> {
  let verified = verify(paths, id, version)?;
  let files = verified
    .release
    .package_config
    .context("package has no config descriptor")?;
  let descriptor_bytes = checked_package_file(
    &verified.root,
    &files.descriptor,
    &files.descriptor_sha256,
    1_048_576,
  )?;
  let descriptor: Descriptor = serde_json::from_slice(&descriptor_bytes)?;
  ensure!(
    descriptor.contract == "isos.package-config@v1" && descriptor.schema_version == "0.1.0",
    "unsupported package config descriptor"
  );
  validate_descriptor(&descriptor)?;
  let template = String::from_utf8(checked_package_file(
    &verified.root,
    &files.template,
    &files.template_sha256,
    1_048_576,
  )?)?;
  validate(&descriptor, &template, &[])?;
  Ok((descriptor, template))
}

fn table_at<'a>(value: &'a toml::Value, section: &str) -> Option<&'a toml::value::Table> {
  let mut current = value;
  for part in section.split('.') {
    current = current.get(part)?;
  }
  current.as_table()
}

/// `selected` contains service instance IDs required by the applied graph.
pub fn validate(descriptor: &Descriptor, source: &str, selected: &[String]) -> Result<()> {
  ensure!(source.len() <= 1_048_576, "package config exceeds 1 MiB");
  let document: toml::Value = toml::from_str(source).context("invalid package config TOML")?;
  let root = document
    .as_table()
    .context("package config must be a TOML table")?;
  for (key, value) in root {
    ensure!(
      matches!(key.as_str(), "package" | "host" | "services"),
      "unknown package config section {key}"
    );
    if key == "services" {
      for instance in value.as_table().context("services must be a table")?.keys() {
        ensure!(
          descriptor
            .sections
            .iter()
            .any(|section| section.id == format!("services.{instance}")),
          "undeclared service instance {instance}"
        );
      }
    } else {
      ensure!(
        descriptor.sections.iter().any(|section| section.id == *key),
        "undeclared package config section {key}"
      );
    }
  }
  let selected: BTreeSet<_> = selected.iter().map(String::as_str).collect();
  for instance in &selected {
    ensure!(
      descriptor
        .sections
        .iter()
        .any(|section| section.id == format!("services.{instance}")),
      "selected service instance {instance} is undeclared"
    );
  }
  for section in &descriptor.sections {
    let table = table_at(&document, &section.id);
    let active = if section.id.starts_with("services.") {
      selected.contains(section.id.trim_start_matches("services."))
    } else {
      !selected.is_empty()
    };
    if active && section.id.starts_with("services.") {
      ensure!(
        table.is_some(),
        "selected service {} has no config",
        section.id
      );
    }
    if let Some(table) = table {
      for key in table.keys() {
        ensure!(
          section.fields.iter().any(|field| field.name == *key),
          "unknown config field {}.{key}",
          section.id
        );
      }
      for field in &section.fields {
        let value = table.get(&field.name);
        if active && field.required {
          ensure!(
            value.is_some_and(|value| value.as_str().is_none_or(|text| !text.is_empty())),
            "required config field {}.{} missing",
            section.id,
            field.name
          );
        }
        if let Some(value) = value {
          match field.field_type.as_str() {
            "string" => ensure!(value.is_str(), "{}.{} must be text", section.id, field.name),
            "enum" => ensure!(
              value
                .as_str()
                .is_some_and(|text| field.options.iter().any(|option| option.value == text)),
              "{}.{} must be a declared option",
              section.id,
              field.name
            ),
            "directory" | "file" => {
              let path = value.as_str().context("path config field must be text")?;
              if !path.is_empty() {
                ensure!(
                  Path::new(path).is_absolute(),
                  "{}.{} must be absolute",
                  section.id,
                  field.name
                );
                if active {
                  let actual = Path::new(path)
                    .canonicalize()
                    .context("configured path missing")?;
                  ensure!(
                    if field.field_type == "directory" {
                      actual.is_dir()
                    } else {
                      actual.is_file()
                    },
                    "configured path has wrong kind"
                  );
                }
              }
            }
            "integer" => {
              let number = value
                .as_integer()
                .context("integer config field expected")? as f64;
              ensure!(
                field.min.is_none_or(|min| number >= min)
                  && field.max.is_none_or(|max| number <= max),
                "config integer out of bounds"
              );
            }
            "boolean" => ensure!(
              value.is_bool(),
              "{}.{} must be boolean",
              section.id,
              field.name
            ),
            _ => unreachable!(),
          }
        }
      }
    }
  }
  Ok(())
}

fn read_config_file(path: &Path, template: String) -> Result<String> {
  Ok(match fs::symlink_metadata(path) {
    Ok(metadata) => {
      ensure!(
        metadata.file_type().is_file()
          && metadata.nlink() == 1
          && metadata.uid() == unsafe { libc::geteuid() }
          && metadata.permissions().mode() & 0o7777 == 0o600
          && metadata.len() <= 1_048_576,
        "unsafe package config file"
      );
      let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
      let opened = file.metadata()?;
      ensure!(
        opened.ino() == metadata.ino() && opened.dev() == metadata.dev(),
        "package config changed while opening"
      );
      let mut bytes = Vec::new();
      file.take(1_048_577).read_to_end(&mut bytes)?;
      ensure!(bytes.len() <= 1_048_576, "package config is too large");
      String::from_utf8(bytes)?
    }
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => template,
    Err(error) => return Err(error.into()),
  })
}

pub fn show(paths: &Paths, deployment: &str, id: &str, version: &str) -> Result<Saved> {
  let (descriptor, template) = descriptor_and_template(paths, id, version)?;
  let path = storage_path(paths, deployment, id, version)?;
  let toml = read_config_file(&path, template)?;
  let values: toml::Value = toml::from_str(&toml)?;
  ensure!(
    values
      .get("package")
      .and_then(|package| package.get("id"))
      .and_then(toml::Value::as_str)
      == Some(id),
    "package config identity differs from exact package pin"
  );
  Ok(Saved {
    id: id.into(),
    version: version.into(),
    deployment: deployment.into(),
    revision: digest(toml.as_bytes()),
    toml,
    descriptor,
    values,
  })
}

pub fn setup_needed(saved: &Saved) -> bool {
  let Some(setup) = &saved.descriptor.setup else {
    return false;
  };
  setup.outputs.iter().any(|name| {
    let kind = saved
      .descriptor
      .sections
      .iter()
      .find(|section| section.id == "host")
      .and_then(|section| section.fields.iter().find(|field| field.name == *name))
      .map(|field| field.field_type.as_str());
    saved
      .values
      .get("host")
      .and_then(|host| host.get(name))
      .and_then(toml::Value::as_str)
      .is_none_or(|value| {
        value.is_empty()
          || match kind {
            Some("directory") => !Path::new(value).is_dir(),
            Some("file") => !Path::new(value).is_file(),
            _ => true,
          }
      })
  })
}

/// Run a verified, operator-approved package setup helper and return a config draft.
pub fn run_setup(paths: &Paths, saved: &Saved) -> Result<String> {
  let setup = saved
    .descriptor
    .setup
    .as_ref()
    .context("package has no setup helper")?;
  let package = verify(paths, &saved.id, &saved.version)?.root;
  let entrypoint = package
    .join(&setup.entrypoint)
    .canonicalize()
    .context("package setup helper is missing")?;
  ensure!(
    entrypoint.starts_with(package.canonicalize()?),
    "package setup path escapes package"
  );
  ensure!(entrypoint.is_file(), "package setup helper is not a file");
  ensure!(
    fs::metadata(&entrypoint)?.permissions().mode() & 0o111 != 0,
    "package setup helper is not executable"
  );
  let destination = paths
    .data
    .join("package-setup")
    .join(&saved.id)
    .join(&saved.version);
  fs::create_dir_all(&destination)?;
  let output = tempfile::NamedTempFile::new()?;
  let status = std::process::Command::new(&entrypoint)
    .arg(&destination)
    .arg(output.path())
    .status()
    .context("could not launch package setup helper")?;
  ensure!(status.success(), "package setup helper failed");
  let bytes = fs::read(output.path())?;
  ensure!(bytes.len() <= 4096, "package setup output is too large");
  let values: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&bytes)?;
  ensure!(
    values.len() == setup.outputs.len(),
    "package setup output fields differ"
  );
  let mut draft: toml::Value = toml::from_str(&saved.toml)?;
  let host = draft
    .get_mut("host")
    .and_then(toml::Value::as_table_mut)
    .context("package config has no host section")?;
  for name in &setup.outputs {
    let value = values
      .get(name)
      .and_then(serde_json::Value::as_str)
      .context("package setup output is missing a path")?;
    let path = Path::new(value);
    ensure!(
      path.is_absolute() && path.exists(),
      "package setup output path is unavailable"
    );
    let kind = saved
      .descriptor
      .sections
      .iter()
      .find(|section| section.id == "host")
      .and_then(|section| section.fields.iter().find(|field| field.name == *name))
      .map(|field| field.field_type.as_str())
      .context("package setup output field is undeclared")?;
    ensure!(
      (kind == "directory" && path.is_dir()) || (kind == "file" && path.is_file()),
      "package setup output has wrong path kind"
    );
    host.insert(name.clone(), toml::Value::String(value.to_owned()));
  }
  let source = toml::to_string_pretty(&draft)?;
  validate(&saved.descriptor, &source, &[])?;
  Ok(source)
}

/// Read and validate only the service instances selected by one graph revision.
/// The returned revision is the exact TOML SHA-256 for Apply/Run fencing.
pub fn selected(
  paths: &Paths,
  deployment: &str,
  id: &str,
  version: &str,
  instances: &[String],
) -> Result<Saved> {
  let saved = show(paths, deployment, id, version)?;
  validate(&saved.descriptor, &saved.toml, instances)?;
  Ok(saved)
}

pub fn save(
  paths: &Paths,
  deployment: &str,
  id: &str,
  version: &str,
  expected_revision: &str,
  source: &str,
) -> Result<Saved> {
  let current = show(paths, deployment, id, version)?;
  ensure!(
    expected_revision == current.revision,
    "package config revision mismatch"
  );
  validate(&current.descriptor, source, &[])?;
  let values: toml::Value = toml::from_str(source)?;
  ensure!(
    values
      .get("package")
      .and_then(|package| package.get("id"))
      .and_then(toml::Value::as_str)
      == Some(id),
    "package config identity differs from exact package pin"
  );
  let path = storage_path(paths, deployment, id, version)?;
  config::atomic_write(&path, source.as_bytes())?;
  show(paths, deployment, id, version)
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn package_setup_declares_only_host_paths_and_safe_entrypoints() {
    let mut descriptor: Descriptor = serde_json::from_value(serde_json::json!({
      "contract":"isos.package-config@v1", "schema_version":"0.1.0",
      "setup":{"service":"robocasa","entrypoint":"bin/setup","prompt":{"en":"Download?","cn":"下载？"},
        "outputs":["assets"]},
      "sections":[
        {"id":"host","label":{"en":"Host","cn":"本机"},"description":{"en":"Host","cn":"本机"},
          "fields":[{"name":"assets","type":"directory","required":false,"label":{"en":"Assets","cn":"资源"},
            "description":{"en":"Assets","cn":"资源"}}]},
        {"id":"services.robocasa","label":{"en":"RoboCasa","cn":"RoboCasa"},
          "description":{"en":"RoboCasa","cn":"RoboCasa"},"fields":[]}
      ]
    })).unwrap();
    validate_descriptor(&descriptor).unwrap();
    descriptor.setup.as_mut().unwrap().entrypoint = "../setup".into();
    assert!(validate_descriptor(&descriptor).is_err());
    descriptor.setup.as_mut().unwrap().entrypoint = "bin/setup".into();
    descriptor.setup.as_mut().unwrap().outputs = vec!["undeclared".into()];
    assert!(validate_descriptor(&descriptor).is_err());
    descriptor.setup.as_mut().unwrap().outputs = vec!["assets".into(), "assets".into()];
    assert!(validate_descriptor(&descriptor).is_err());
  }
  #[test]
  fn config_read_rejects_symlink_and_uses_template_only_when_absent() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("package.toml");
    let target = temp.path().join("target.toml");
    assert_eq!(
      read_config_file(&config, "[package]\n".into()).unwrap(),
      "[package]\n"
    );
    fs::write(&target, b"[package]\nid='safe'\n").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    std::os::unix::fs::symlink(&target, &config).unwrap();
    assert!(read_config_file(&config, "fallback".into()).is_err());
    fs::remove_file(&config).unwrap();
    fs::rename(&target, &config).unwrap();
    assert!(
      read_config_file(&config, "fallback".into())
        .unwrap()
        .contains("safe")
    );
  }
  #[test]
  fn external_file_requirement_fingerprint_changes_with_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("driver");
    fs::create_dir_all(root.join("lib")).unwrap();
    let library = root.join("lib/libdriver.so");
    fs::write(&library, b"first").unwrap();
    let service: crate::domain::registry::release::RuntimeService = serde_json::from_value(serde_json::json!({
      "id":"native","entrypoint":"bin/sim","config_service_type":"native","node_types":["eos.hardware.camera.device@v0.2"],
      "requirements":[{"path":"${config.host.driver_root}/lib/libdriver.so","kind":"file"}]
    })).unwrap();
    let values: toml::Value = toml::from_str(&format!(
      "[host]\ndriver_root = '{}'\n[services.native]\nscene = 'pick-place'\n",
      root.display()
    ))
    .unwrap();
    let saved = Saved {
      id: "vendor/sim".into(),
      version: "0.2.1".into(),
      deployment: "default".into(),
      revision: String::new(),
      toml: String::new(),
      descriptor: Descriptor {
        contract: "isos.package-config@v1".into(),
        schema_version: "0.1.0".into(),
        setup: None,
        sections: Vec::new(),
      },
      values,
    };
    let first = external_inputs(&service, &saved, "native").unwrap();
    assert_eq!(first[0].path, library.display().to_string());
    fs::write(&library, b"second").unwrap();
    let second = external_inputs(&service, &saved, "native").unwrap();
    assert_ne!(first[0].sha256, second[0].sha256);
  }
  #[test]
  fn selected_service_requires_path_and_preserves_optional_service() {
    let descriptor = Descriptor {
      contract: "isos.package-config@v1".into(),
      schema_version: "0.1.0".into(),
      setup: None,
      sections: vec![Section {
        id: "services.native".into(),
        label: Text {
          en: "Native".into(),
          cn: "本地".into(),
        },
        description: Text {
          en: "Native".into(),
          cn: "本地".into(),
        },
        fields: vec![Field {
          name: "root".into(),
          field_type: "directory".into(),
          required: true,
          label: Text {
            en: "Root".into(),
            cn: "目录".into(),
          },
          description: Text {
            en: "Root".into(),
            cn: "目录".into(),
          },
          note: None,
          default: None,
          min: None,
          max: None,
          options: Vec::new(),
        }],
      }],
    };
    assert!(validate(&descriptor, "[services.native]\nroot = ''\n", &[]).is_ok());
    assert!(
      validate(
        &descriptor,
        "[services.native]\nroot = ''\n",
        &["native".into()]
      )
      .is_err()
    );
  }
  #[test]
  fn enum_options_reject_unknown_values_and_bad_descriptors() {
    let mut descriptor: Descriptor = serde_json::from_value(serde_json::json!({
      "contract":"isos.package-config@v1", "schema_version":"0.1.0", "sections":[{
        "id":"services.native", "label":{"en":"Native","cn":"本地"},
        "description":{"en":"Native","cn":"本地"}, "fields":[{
          "name":"scene", "type":"enum", "required":true,
          "label":{"en":"Scene","cn":"场景"},
          "description":{"en":"Selected scene","cn":"所选场景"},
          "default":"pick-place", "options":[
            {"value":"pick-place","label":{"en":"Pick place","cn":"抓取放置"}},
            {"value":"navigation","label":{"en":"Navigation","cn":"导航"}}
          ]
        }]
      }]
    }))
    .unwrap();
    assert!(validate_descriptor(&descriptor).is_ok());
    assert!(
      validate(
        &descriptor,
        "[services.native]\nscene = 'navigation'\n",
        &["native".into()]
      )
      .is_ok()
    );
    assert!(
      validate(
        &descriptor,
        "[services.native]\nscene = 'unknown'\n",
        &["native".into()]
      )
      .is_err()
    );
    assert!(
      validate(
        &descriptor,
        "[services.native]\nscene = 1\n",
        &["native".into()]
      )
      .is_err()
    );
    let duplicate = descriptor.sections[0].fields[0].options[0].clone();
    descriptor.sections[0].fields[0].options.push(duplicate);
    assert!(validate_descriptor(&descriptor).is_err());
    descriptor.sections[0].fields[0].options.pop();
    descriptor.sections[0].fields[0].default = Some(serde_json::json!("unknown"));
    assert!(validate_descriptor(&descriptor).is_err());
  }
}
