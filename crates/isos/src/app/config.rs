use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
  collections::BTreeMap,
  fs,
  io::Write,
  os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
  path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Paths {
  pub config: PathBuf,
  pub data: PathBuf,
  pub runtime: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeConfig {
  #[serde(default)]
  server: ServerConfig,
  #[serde(default)]
  paths: RuntimePaths,
  #[serde(default)]
  deployment: DeploymentConfig,
  #[serde(default)]
  logging: LoggingConfig,
  #[cfg(test)]
  #[serde(default)]
  vision_evidence: VisionEvidenceConfig,
  #[serde(default)]
  package_host_inputs: BTreeMap<String, PathBuf>,
}

#[cfg(test)]
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisionEvidenceConfig {
  gate_path: Option<String>,
}

#[cfg(test)]
thread_local! {
    static TEST_VISION_CONFIG_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn test_vision_config_path(path: Option<PathBuf>) {
  TEST_VISION_CONFIG_PATH.with(|selected| *selected.borrow_mut() = path);
}

/// The selected file supplies the default; a process environment value takes precedence.
#[cfg(test)]
pub fn vision_gate_selection() -> Result<(Option<String>, String)> {
  use sha2::{Digest, Sha256};
  let path = runtime_config_path(
    Path::new(env!("CARGO_MANIFEST_DIR")),
    std::env::var_os("ISOS_CONFIG").as_deref(),
  )?;
  #[cfg(test)]
  let path = TEST_VISION_CONFIG_PATH
    .with(|selected| selected.borrow().clone())
    .unwrap_or(path);
  let source = merged_config_source(&path, std::env::var_os("ISOS_CONFIG").is_some())?;
  let config: RuntimeConfig = toml::from_str(&source)?;
  let gate = select_vision_gate(
    config.vision_evidence.gate_path,
    std::env::var("ISOS_VISION_GATE_PATH").ok(),
  );
  let fingerprint_source = format!(
    "{source}\nvision_gate_override={:?}",
    std::env::var_os("ISOS_VISION_GATE_PATH")
  );
  Ok((
    gate,
    hex::encode(Sha256::digest(fingerprint_source.as_bytes())),
  ))
}

#[cfg(test)]
fn select_vision_gate(
  configured: Option<String>,
  override_value: Option<String>,
) -> Option<String> {
  override_value
    .or(configured)
    .filter(|value| !value.is_empty())
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerConfig {
  http_port: Option<u16>,
  ui_port: Option<u16>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimePaths {
  ui_repo: Option<PathBuf>,
  ui_dist: Option<PathBuf>,
  aether_bin_dir: Option<PathBuf>,
  ei_bin_dir: Option<PathBuf>,
  ei_defaults_dir: Option<PathBuf>,
  ei_factory_defaults_dir: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeploymentConfig {
  name: Option<String>,
  stop_timeout_ms: Option<u64>,
  eos_stop_grace_ms: Option<u64>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub struct StopBudget {
  pub timeout_ms: u64,
  pub eos_grace_ms: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct StopBudgetDiagnostic {
  pub code: &'static str,
  pub stage: &'static str,
  pub path: &'static str,
}

impl std::fmt::Display for StopBudgetDiagnostic {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(formatter, "{}: {}", self.code, self.path)
  }
}

impl std::error::Error for StopBudgetDiagnostic {}

fn validate_stop_budget(config: &DeploymentConfig) -> Result<(u64, u64)> {
  let timeout_ms = config.stop_timeout_ms.unwrap_or(30_000);
  ensure!(
    (5_000..=120_000).contains(&timeout_ms),
    StopBudgetDiagnostic {
      code: "stop_budget_invalid",
      stage: "config",
      path: "/deployment/stop_timeout_ms"
    }
  );
  let eos_grace_ms = config.eos_stop_grace_ms.unwrap_or(20_000);
  ensure!(
    (1_000..=timeout_ms - 3_000).contains(&eos_grace_ms),
    StopBudgetDiagnostic {
      code: "stop_budget_invalid",
      stage: "config",
      path: "/deployment/eos_stop_grace_ms"
    }
  );
  Ok((timeout_ms, eos_grace_ms))
}

#[cfg(test)]
impl StopBudget {
  fn from_deployment(config: &DeploymentConfig) -> Result<Self> {
    let (timeout_ms, eos_grace_ms) = validate_stop_budget(config)?;
    Ok(Self {
      timeout_ms,
      eos_grace_ms,
    })
  }
}

#[cfg(test)]
pub fn stop_budget() -> Result<StopBudget> {
  let path = runtime_config_path(
    Path::new(env!("CARGO_MANIFEST_DIR")),
    std::env::var_os("ISOS_CONFIG").as_deref(),
  )?;
  let source = merged_config_source(&path, std::env::var_os("ISOS_CONFIG").is_some())?;
  let mut config: RuntimeConfig = toml::from_str(&source)?;
  apply_stop_budget_env(&mut config.deployment)?;
  StopBudget::from_deployment(&config.deployment)
}

#[cfg(test)]
fn apply_stop_budget_env(config: &mut DeploymentConfig) -> Result<()> {
  if let Some(value) = std::env::var_os("ISOS_STOP_TIMEOUT_MS") {
    config.stop_timeout_ms = Some(
      value
        .to_string_lossy()
        .parse()
        .context("ISOS_STOP_TIMEOUT_MS must be an integer")?,
    );
  }
  if let Some(value) = std::env::var_os("ISOS_EOS_STOP_GRACE_MS") {
    config.eos_stop_grace_ms = Some(
      value
        .to_string_lossy()
        .parse()
        .context("ISOS_EOS_STOP_GRACE_MS must be an integer")?,
    );
  }
  Ok(())
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoggingConfig {
  default_source: Option<String>,
  max_bytes: Option<u64>,
  backups: Option<u8>,
  max_sources: Option<u16>,
  level: Option<String>,
}

#[cfg(test)]
fn config_values(source: &str, repo_root: &Path) -> Result<BTreeMap<String, String>> {
  let config: RuntimeConfig = toml::from_str(source)?;
  config_values_from(config, repo_root)
}

fn config_values_from(config: RuntimeConfig, repo_root: &Path) -> Result<BTreeMap<String, String>> {
  validate_stop_budget(&config.deployment)?;
  let mut values = BTreeMap::new();
  if let Some(timeout) = config.deployment.stop_timeout_ms {
    values.insert("ISOS_STOP_TIMEOUT_MS".into(), timeout.to_string());
  }
  if let Some(grace) = config.deployment.eos_stop_grace_ms {
    values.insert("ISOS_EOS_STOP_GRACE_MS".into(), grace.to_string());
  }
  for (port, key) in [
    (config.server.http_port, "ISOS_HTTP_PORT"),
    (config.server.ui_port, "ISOSUI_PORT"),
  ] {
    if let Some(port) = port {
      ensure!(port >= 1024, "{key} must be 1024..65535");
      values.insert(key.into(), port.to_string());
    }
  }
  for (path, key) in [
    (config.paths.ui_repo, "ISOSUI_REPO_PATH"),
    (config.paths.ui_dist, "ISOSUI_DIST_DIR"),
    (config.paths.aether_bin_dir, "ISOS_AETHER_BIN_DIR"),
    (config.paths.ei_bin_dir, "ISOS_EI_BIN_DIR"),
    (config.paths.ei_defaults_dir, "ISOS_EI_DEFAULTS_DIR"),
    (
      config.paths.ei_factory_defaults_dir,
      "ISOS_EI_FACTORY_DEFAULTS_DIR",
    ),
  ] {
    if let Some(path) = path {
      let path = if path.is_absolute() {
        path
      } else {
        repo_root.join(path)
      };
      values.insert(key.into(), path.to_string_lossy().into_owned());
    }
  }
  if let Some(name) = config.deployment.name {
    crate::runtime::process::validate_deployment_name(&name)?;
    values.insert("ISOS_DEPLOYMENT".into(), name);
  }
  if let Some(source) = config.logging.default_source {
    ensure!(
      source == "all" || crate::runtime::logs::valid_source(&source),
      "logging.default_source must be all or a valid log source"
    );
    values.insert("ISOS_LOG_DEFAULT_SOURCE".into(), source);
  }
  if let Some(max_bytes) = config.logging.max_bytes {
    ensure!(
      (65_536..=100_000_000).contains(&max_bytes),
      "logging.max_bytes must be 65536..100000000"
    );
    values.insert("ISOS_LOG_MAX_BYTES".into(), max_bytes.to_string());
  }
  if let Some(backups) = config.logging.backups {
    ensure!((1..=10).contains(&backups), "logging.backups must be 1..10");
    values.insert("ISOS_LOG_BACKUPS".into(), backups.to_string());
  }
  if let Some(max_sources) = config.logging.max_sources {
    ensure!(
      (10..=256).contains(&max_sources),
      "logging.max_sources must be 10..256"
    );
    values.insert("ISOS_LOG_MAX_SOURCES".into(), max_sources.to_string());
  }
  if let Some(level) = config.logging.level {
    ensure!(
      crate::runtime::logs::level_rank(&level).is_some(),
      "logging.level must be trace, debug, info, warn, or error"
    );
    values.insert("ISOS_LOG_LEVEL".into(), level);
  }
  for (key, path) in config.package_host_inputs {
    ensure!(
      key.len() <= 64
        && key
          .bytes()
          .next()
          .is_some_and(|byte| byte.is_ascii_uppercase())
        && key
          .bytes()
          .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        && !key.starts_with("ISOS_")
        && !key.starts_with("ISLAND_")
        && !key.starts_with("EI_"),
      "invalid package_host_inputs key {key}"
    );
    ensure!(
      path.is_absolute(),
      "package_host_inputs.{key} must be an absolute path"
    );
    ensure!(
      values
        .insert(key.clone(), path.to_string_lossy().into_owned())
        .is_none(),
      "package_host_inputs.{key} conflicts with a built-in setting"
    );
  }
  Ok(values)
}

fn config_without_env_overrides(
  values: BTreeMap<String, String>,
  has_env: impl Fn(&str) -> bool,
) -> BTreeMap<String, String> {
  values
    .into_iter()
    .filter(|(key, _)| !has_env(key))
    .collect()
}

fn runtime_config_path(
  repo_root: &Path,
  override_path: Option<&std::ffi::OsStr>,
) -> Result<PathBuf> {
  if let Some(path) = override_path {
    let path = PathBuf::from(path);
    ensure!(path.is_absolute(), "ISOS_CONFIG must be an absolute path");
    Ok(path)
  } else {
    Ok(repo_root.join("../../config/isos/isos.toml"))
  }
}

fn fill_missing_toml(selected: &mut toml::Value, defaults: toml::Value) {
  if let (toml::Value::Table(base), toml::Value::Table(fallbacks)) = (selected, defaults) {
    for (key, value) in fallbacks {
      if let Some(existing) = base.get_mut(&key) {
        fill_missing_toml(existing, value);
      } else {
        base.insert(key, value);
      }
    }
  }
}

/// Read the selected file first, then fill its missing fields from the example.
/// TOML comments never become configuration values.
fn merged_config_source(path: &Path, required: bool) -> Result<String> {
  let selected = match fs::read_to_string(path) {
    Ok(source) => source,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound && !required => String::new(),
    Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
  };
  let mut merged: toml::Value = if selected.trim().is_empty() {
    toml::Value::Table(toml::map::Map::new())
  } else {
    toml::from_str(&selected).with_context(|| format!("parse {}", path.display()))?
  };
  let defaults: toml::Value =
    toml::from_str(include_str!("../../../../config/isos/isos.example.toml"))
      .context("parse config/isos/isos.example.toml")?;
  fill_missing_toml(&mut merged, defaults);
  Ok(toml::to_string(&merged)?)
}

fn config_guide() -> String {
  "https://github.com/Island-Team/isos-p/blob/master/docs/isos/design/app/configuration.md".into()
}

fn apply_process_overrides(config: &mut RuntimeConfig) -> Result<()> {
  macro_rules! number {
    ($field:expr, $key:literal) => {
      if let Some(value) = std::env::var_os($key) {
        $field = Some(
          value
            .to_string_lossy()
            .parse()
            .with_context(|| format!("{} must be an integer", $key))?,
        );
      }
    };
  }
  macro_rules! string {
    ($field:expr, $key:literal) => {
      if let Some(value) = std::env::var_os($key) {
        $field = Some(
          value
            .into_string()
            .map_err(|_| anyhow::anyhow!("{} must be UTF-8", $key))?,
        );
      }
    };
  }
  macro_rules! path {
    ($field:expr, $key:literal) => {
      if let Some(value) = std::env::var_os($key) {
        let selected = PathBuf::from(value);
        ensure!(selected.is_absolute(), "{} must be an absolute path", $key);
        $field = Some(selected);
      }
    };
  }
  number!(config.server.http_port, "ISOS_HTTP_PORT");
  number!(config.server.ui_port, "ISOSUI_PORT");
  path!(config.paths.ui_repo, "ISOSUI_REPO_PATH");
  path!(config.paths.ui_dist, "ISOSUI_DIST_DIR");
  path!(config.paths.aether_bin_dir, "ISOS_AETHER_BIN_DIR");
  path!(config.paths.ei_bin_dir, "ISOS_EI_BIN_DIR");
  path!(config.paths.ei_defaults_dir, "ISOS_EI_DEFAULTS_DIR");
  path!(
    config.paths.ei_factory_defaults_dir,
    "ISOS_EI_FACTORY_DEFAULTS_DIR"
  );
  string!(config.deployment.name, "ISOS_DEPLOYMENT");
  number!(config.deployment.stop_timeout_ms, "ISOS_STOP_TIMEOUT_MS");
  number!(
    config.deployment.eos_stop_grace_ms,
    "ISOS_EOS_STOP_GRACE_MS"
  );
  string!(config.logging.default_source, "ISOS_LOG_DEFAULT_SOURCE");
  string!(config.logging.level, "ISOS_LOG_LEVEL");
  number!(config.logging.max_bytes, "ISOS_LOG_MAX_BYTES");
  number!(config.logging.backups, "ISOS_LOG_BACKUPS");
  number!(config.logging.max_sources, "ISOS_LOG_MAX_SOURCES");
  for (key, selected) in &mut config.package_host_inputs {
    if let Some(value) = std::env::var_os(key) {
      let path = PathBuf::from(value);
      ensure!(path.is_absolute(), "{key} must be an absolute path");
      *selected = path;
    }
  }
  Ok(())
}

fn validate_effective_values(values: &BTreeMap<String, String>) -> Result<()> {
  let budget = DeploymentConfig {
    stop_timeout_ms: values
      .get("ISOS_STOP_TIMEOUT_MS")
      .map(|value| {
        value
          .parse()
          .context("ISOS_STOP_TIMEOUT_MS must be an integer")
      })
      .transpose()?,
    eos_stop_grace_ms: values
      .get("ISOS_EOS_STOP_GRACE_MS")
      .map(|value| {
        value
          .parse()
          .context("ISOS_EOS_STOP_GRACE_MS must be an integer")
      })
      .transpose()?,
    ..DeploymentConfig::default()
  };
  validate_stop_budget(&budget)?;
  for key in ["ISOS_HTTP_PORT", "ISOSUI_PORT"] {
    if let Some(value) = values.get(key) {
      let port: u16 = value
        .parse()
        .with_context(|| format!("{key} must be 1024..65535"))?;
      ensure!(port >= 1024, "{key} must be 1024..65535");
    }
  }
  if let (Some(api), Some(ui)) = (values.get("ISOS_HTTP_PORT"), values.get("ISOSUI_PORT")) {
    ensure!(api != ui, "API and UI ports must differ");
  }
  for key in [
    "ISOSUI_REPO_PATH",
    "ISOSUI_DIST_DIR",
    "ISOS_AETHER_BIN_DIR",
    "ISOS_EI_BIN_DIR",
    "ISOS_EI_DEFAULTS_DIR",
    "ISOS_EI_FACTORY_DEFAULTS_DIR",
  ] {
    if let Some(path) = values.get(key) {
      ensure!(
        Path::new(path).is_absolute(),
        "{key} must be an absolute path"
      );
    }
  }
  if let Some(name) = values.get("ISOS_DEPLOYMENT") {
    crate::runtime::process::validate_deployment_name(name)?;
  }
  if let Some(source) = values.get("ISOS_LOG_DEFAULT_SOURCE") {
    ensure!(
      source == "all" || crate::runtime::logs::valid_source(source),
      "ISOS_LOG_DEFAULT_SOURCE must be all or a valid log source"
    );
  }
  if let Some(level) = values.get("ISOS_LOG_LEVEL") {
    ensure!(
      crate::runtime::logs::level_rank(level).is_some(),
      "ISOS_LOG_LEVEL must be trace, debug, info, warn, or error"
    );
  }
  for (key, range) in [
    ("ISOS_LOG_MAX_BYTES", 65_536..=100_000_000),
    ("ISOS_LOG_BACKUPS", 1..=10),
    ("ISOS_LOG_MAX_SOURCES", 10..=256),
  ] {
    if let Some(value) = values.get(key) {
      let number: u64 = value
        .parse()
        .with_context(|| format!("{key} must be a number"))?;
      ensure!(range.contains(&number), "{key} is out of range");
    }
  }
  if let Some(gate) = std::env::var_os("ISOS_VISION_GATE_PATH") {
    let gate = gate
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("ISOS_VISION_GATE_PATH must be UTF-8"))?;
    ensure!(
      gate.is_empty()
        || (gate.split('/').count() == 4
          && (gate.starts_with("eos/interface/gate/")
            || gate.starts_with("isos/eos_interface/gate/"))
          && gate.split('/').all(|part| !part.is_empty()
            && part.len() <= 48
            && part
              .bytes()
              .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))),
      "ISOS_VISION_GATE_PATH must name a canonical EOS gate"
    );
  }
  Ok(())
}

pub fn load_runtime_config() -> Result<()> {
  load_runtime_config_inner()
    .with_context(|| format!("Edit IsOS configuration: {}", config_guide()))
}

fn load_runtime_config_inner() -> Result<()> {
  let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
  let override_path = std::env::var_os("ISOS_CONFIG");
  let path = runtime_config_path(repo_root, override_path.as_deref())?;
  let source = merged_config_source(&path, override_path.is_some())?;
  let mut config: RuntimeConfig =
    toml::from_str(&source).with_context(|| format!("parse {}", path.display()))?;
  apply_process_overrides(&mut config)?;
  let values = config_values_from(config, repo_root)
    .with_context(|| format!("validate {}", path.display()))?;
  validate_effective_values(&values)?;
  for (key, value) in config_without_env_overrides(values, |key| std::env::var_os(key).is_some()) {
    // This runs once before CLI parsing, threads, or child processes exist.
    unsafe {
      std::env::set_var(key, value);
    }
  }
  Ok(())
}

impl Paths {
  fn from_home(base: &Path) -> Result<Self> {
    ensure!(base.is_absolute(), "ISOS_HOME must be an absolute path");
    Ok(Self {
      config: base.join("config"),
      data: base.join("data/isos"),
      runtime: base.join("data/isos/run"),
    })
  }

  fn system() -> Self {
    Self {
      config: PathBuf::from("/etc/island/isos"),
      data: PathBuf::from("/var/lib/island/isos"),
      runtime: PathBuf::from("/run/island/isos"),
    }
  }

  pub fn mode(&self) -> Result<&'static str> {
    let selected = if let Ok(mode) = std::env::var("ISOS_MODE") {
      Some(mode)
    } else {
      let path = self.config.join("isos/launch.toml");
      if path.exists() {
        let source = fs::read_to_string(&path)?;
        let config: toml::Value = toml::from_str(&source)?;
        config
          .get("launch")
          .and_then(|launch| launch.get("mode"))
          .and_then(toml::Value::as_str)
          .map(str::to_owned)
      } else {
        None
      }
    };
    match selected.as_deref().unwrap_or("product") {
      "dev" => Ok("dev"),
      "product" => Ok("product"),
      _ => anyhow::bail!("IsOS mode must be dev or product"),
    }
  }

  pub fn set_mode(&self, mode: &str) -> Result<()> {
    ensure!(
      matches!(mode, "dev" | "product"),
      "IsOS mode must be dev or product"
    );
    atomic_write(
      &self.config.join("isos/launch.toml"),
      format!("[launch]\nmode = \"{mode}\"\n").as_bytes(),
    )
  }

  pub fn discover() -> Result<Self> {
    if let Ok(base) = std::env::var("ISOS_HOME") {
      return Self::from_home(Path::new(&base));
    }
    if std::env::var("ISLAND_MODE").as_deref() == Ok("system") {
      return Ok(Self::system());
    }
    // A source checkout keeps runtime and test evidence under its own data/
    // tree. Installed executables retain their XDG defaults.
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
      .parent()
      .and_then(Path::parent);
    if let (Some(root), Ok(executable)) = (source_root, std::env::current_exe())
      && executable.starts_with(root.join("target"))
    {
      return Self::from_home(root);
    }
    let home = std::env::var_os("HOME")
      .map(PathBuf::from)
      .ok_or_else(|| anyhow::anyhow!("HOME is required"))?;
    let config = std::env::var_os("XDG_CONFIG_HOME")
      .map(PathBuf::from)
      .unwrap_or_else(|| home.join(".config"))
      .join("isos");
    let data = std::env::var_os("XDG_DATA_HOME")
      .map(PathBuf::from)
      .unwrap_or_else(|| home.join(".local/share"))
      .join("island/isos");
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
      .map(PathBuf::from)
      .unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", unsafe { libc::getuid() })))
      .join("isos");
    Ok(Self {
      config,
      data,
      runtime,
    })
  }

  pub fn node_graph_path(&self, name: &str) -> PathBuf {
    let path = self.graph_pool_path(&self.bound_graph_name(name));
    if fs::symlink_metadata(&path)
      .is_ok_and(|metadata| !metadata.file_type().is_file() || metadata.nlink() != 1)
    {
      return self.config.join("graph-pool/.unsafe-graph.nodes.json");
    }
    path
  }
  pub fn graph_pool_path(&self, name: &str) -> PathBuf {
    self
      .config
      .join("graph-pool")
      .join(format!("{name}.nodes.json"))
  }
  pub fn graph_binding_path(&self, name: &str) -> PathBuf {
    self
      .config
      .join("deployments")
      .join(format!("{name}.graph.json"))
  }
  pub fn bound_graph_name(&self, name: &str) -> String {
    let binding = self.graph_binding_path(name);
    match fs::symlink_metadata(&binding) {
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => return "demo".to_owned(),
      Ok(metadata) if metadata.file_type().is_file() && metadata.nlink() == 1 => {}
      _ => return "__invalid_binding__".to_owned(),
    }
    fs::read(binding)
      .ok()
      .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
      .and_then(|value| value["pool_name"].as_str().map(str::to_owned))
      .filter(|value| valid_graph_name(value))
      .unwrap_or_else(|| "__invalid_binding__".to_owned())
  }
  pub fn bind_graph(&self, deployment: &str, pool_name: &str) -> Result<()> {
    ensure!(valid_graph_name(pool_name), "invalid graph pool name");
    self.preflight_graph_binding(deployment)?;
    atomic_write(
      &self.graph_binding_path(deployment),
      &serde_json::to_vec(&serde_json::json!({"pool_name":pool_name}))?,
    )
  }
  pub fn preflight_graph_binding(&self, deployment: &str) -> Result<()> {
    ensure!(valid_graph_name(deployment), "invalid deployment name");
    let directory = self.config.join("deployments");
    match fs::symlink_metadata(&directory) {
      Ok(metadata) => ensure!(
        metadata.file_type().is_dir(),
        "unsafe graph binding directory"
      ),
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
        fs::create_dir(&directory)?;
      }
      Err(error) => return Err(error.into()),
    }
    match fs::symlink_metadata(self.graph_binding_path(deployment)) {
      Ok(metadata) => ensure!(
        metadata.file_type().is_file() && metadata.nlink() == 1,
        "unsafe graph binding"
      ),
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
      Err(error) => return Err(error.into()),
    }
    Ok(())
  }
  pub fn generated(&self, name: &str) -> PathBuf {
    self.data.join("state/generated").join(name)
  }
  pub fn owner_data(&self, owner: &str) -> PathBuf {
    self.data.parent().unwrap_or(&self.data).join(owner)
  }
  pub fn owner_runtime(&self, owner: &str) -> PathBuf {
    if self.runtime.ends_with("island/isos") {
      return self.runtime.parent().unwrap_or(&self.runtime).join(owner);
    }
    self.owner_data(owner).join("run")
  }
  #[cfg(test)]
  pub fn prepare_owner_runtime(&self, owner: &str) -> Result<()> {
    let directory = self.owner_runtime(owner);
    fs::create_dir_all(&directory).with_context(|| format!("create {}", directory.display()))?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    Ok(())
  }
  pub fn socket(&self) -> PathBuf {
    self.runtime.join("manager.sock")
  }

  pub fn prepare(&self) -> Result<()> {
    if let Some(parent) = self.data.parent() {
      fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
      fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    for dir in [&self.config, &self.data, &self.runtime] {
      fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
      fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    self.migrate_legacy_graphs()?;
    Ok(())
  }

  fn migrate_legacy_graphs(&self) -> Result<()> {
    let directory = self.config.join("deployments");
    if !directory.exists() {
      return Ok(());
    }
    ensure!(
      fs::symlink_metadata(&directory)?.file_type().is_dir(),
      "unsafe legacy graph directory"
    );
    use std::os::fd::AsRawFd;
    let lock_path = self.config.join(".graph-migration.lock");
    let lock = fs::OpenOptions::new()
      .create(true)
      .read(true)
      .write(true)
      .truncate(false)
      .custom_flags(libc::O_NOFOLLOW)
      .mode(0o600)
      .open(&lock_path)?;
    ensure!(
      lock.metadata()?.is_file() && lock.metadata()?.nlink() == 1,
      "unsafe graph migration lock"
    );
    ensure!(
      unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } == 0,
      "graph migration lock unavailable"
    );
    for entry in fs::read_dir(&directory)? {
      let entry = entry?;
      let filename = entry.file_name().to_string_lossy().into_owned();
      let Some(deployment) = filename.strip_suffix(".nodes.json") else {
        continue;
      };
      ensure!(
        valid_graph_name(deployment),
        "invalid legacy deployment graph name"
      );
      let legacy = entry.path();
      ensure!(
        fs::symlink_metadata(&legacy)?.file_type().is_file(),
        "unsafe legacy graph"
      );
      let bytes = fs::read(&legacy)?;
      let demo = self.graph_pool_path("demo");
      let same_demo = fs::symlink_metadata(&demo)
        .is_ok_and(|metadata| metadata.file_type().is_file() && metadata.nlink() == 1)
        && fs::read(&demo).is_ok_and(|other| other == bytes);
      let pool_name = if same_demo {
        "demo".to_owned()
      } else {
        let base: String = deployment.chars().take(35).collect();
        let mut candidate = format!("{base}-migrated");
        let mut number = 2;
        while self.graph_pool_path(&candidate).exists() {
          ensure!(
            fs::symlink_metadata(self.graph_pool_path(&candidate))?
              .file_type()
              .is_file(),
            "unsafe migrated graph file"
          );
          if fs::read(self.graph_pool_path(&candidate))? == bytes {
            break;
          }
          ensure!(number <= 999, "too many migrated graph files");
          candidate = format!("{base}-migrated-{number}");
          number += 1;
        }
        if !self.graph_pool_path(&candidate).exists() {
          atomic_write(&self.graph_pool_path(&candidate), &bytes)?;
        }
        candidate
      };
      if !same_demo {
        ensure!(
          fs::symlink_metadata(self.graph_pool_path(&pool_name))?
            .file_type()
            .is_file(),
          "unsafe migrated graph file"
        );
        ensure!(
          fs::read(self.graph_pool_path(&pool_name))? == bytes,
          "migrated graph bytes changed"
        );
      }
      if self.graph_binding_path(deployment).exists() {
        ensure!(
          self.bound_graph_name(deployment) == pool_name,
          "legacy graph conflicts with existing binding"
        );
      } else {
        self.bind_graph(deployment, &pool_name)?;
      }
      fs::remove_file(&legacy)?;
    }
    Ok(())
  }
}

fn valid_graph_name(value: &str) -> bool {
  !value.is_empty()
    && value.len() <= 48
    && value
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

pub fn deployment_name(cli: Option<String>) -> String {
  cli
    .or_else(|| std::env::var("ISOS_DEPLOYMENT").ok())
    .unwrap_or_else(|| "default".into())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
  let parent = path
    .parent()
    .ok_or_else(|| anyhow::anyhow!("path has no parent"))?;
  fs::create_dir_all(parent)?;
  fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
  let temporary = parent.join(format!(
    ".{}.{}.tmp",
    path.file_name().unwrap().to_string_lossy(),
    std::process::id()
  ));
  let mut file = fs::OpenOptions::new()
    .create_new(true)
    .write(true)
    .mode(0o600)
    .open(&temporary)
    .with_context(|| format!("create {}", temporary.display()))?;
  let result = (|| {
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    Ok::<_, anyhow::Error>(())
  })();
  if result.is_err() {
    let _ = fs::remove_file(&temporary);
  }
  result
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn owner_paths_are_separate_in_portable_and_system_modes() {
    let portable = Paths::from_home(Path::new("/opt/island")).unwrap();
    assert_eq!(portable.data, Path::new("/opt/island/data/isos"));
    assert_eq!(portable.runtime, Path::new("/opt/island/data/isos/run"));
    assert_eq!(portable.owner_data("ei"), Path::new("/opt/island/data/ei"));
    assert_eq!(
      portable.owner_runtime("eos"),
      Path::new("/opt/island/data/eos/run")
    );
    assert!(Paths::from_home(Path::new("relative")).is_err());

    let system = Paths::system();
    assert_eq!(system.data, Path::new("/var/lib/island/isos"));
    assert_eq!(system.runtime, Path::new("/run/island/isos"));
    assert_eq!(system.owner_data("ei"), Path::new("/var/lib/island/ei"));
    assert_eq!(system.owner_runtime("eos"), Path::new("/run/island/eos"));
    let xdg_runtime = Paths {
      runtime: PathBuf::from("/tmp/custom-session/island/isos"),
      ..portable
    };
    assert_eq!(
      xdg_runtime.owner_runtime("eos"),
      Path::new("/tmp/custom-session/island/eos")
    );
  }

  #[test]
  fn owner_runtime_is_created_only_when_selected() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(temp.path()).unwrap();
    paths.prepare().unwrap();
    let eos_runtime = paths.owner_runtime("eos");
    assert!(!eos_runtime.exists());
    paths.prepare_owner_runtime("eos").unwrap();
    assert_eq!(
      fs::symlink_metadata(eos_runtime)
        .unwrap()
        .permissions()
        .mode()
        & 0o777,
      0o700
    );
  }
  #[test]
  fn process_overrides_are_validated_after_toml_defaults() {
    let values = config_values(
      "[server]\nhttp_port=16000\nui_port=16001\n",
      Path::new("/tmp/isos-source"),
    )
    .unwrap();
    let mut effective = values.clone();
    effective.insert("ISOS_HTTP_PORT".into(), "37125".into());
    validate_effective_values(&effective).unwrap();
    effective.insert("ISOSUI_PORT".into(), "37125".into());
    assert!(validate_effective_values(&effective).is_err());
    effective.insert("ISOSUI_PORT".into(), "80".into());
    assert!(validate_effective_values(&effective).is_err());
    effective.insert("ISOSUI_PORT".into(), "37126".into());
    effective.insert("ISOS_LOG_LEVEL".into(), "verbose".into());
    assert!(validate_effective_values(&effective).is_err());
  }
  #[test]
  fn empty_vision_override_disables_configured_gate() {
    let configured = Some("eos/interface/gate/front".to_owned());
    assert_eq!(select_vision_gate(configured.clone(), None), configured);
    assert_eq!(select_vision_gate(configured, Some(String::new())), None);
  }
  #[test]
  fn config_toml_maps_settings_and_rejects_invalid_values() {
    let root = Path::new("/tmp/isos-source");
    let values = config_values("[server]\nhttp_port=16000\n[paths]\nui_repo='../IsosUI'\n[logging]\ndefault_source='all'\nlevel='warn'\nmax_bytes=262144\nbackups=2\nmax_sources=32\n", root).unwrap();
    assert_eq!(values["ISOS_HTTP_PORT"], "16000");
    assert_eq!(values["ISOSUI_REPO_PATH"], "/tmp/isos-source/../IsosUI");
    assert_eq!(values["ISOS_LOG_DEFAULT_SOURCE"], "all");
    assert_eq!(values["ISOS_LOG_BACKUPS"], "2");
    assert_eq!(values["ISOS_LOG_MAX_SOURCES"], "32");
    assert_eq!(values["ISOS_LOG_LEVEL"], "warn");
    let selected = config_without_env_overrides(values, |key| key == "ISOS_LOG_LEVEL");
    assert!(!selected.contains_key("ISOS_LOG_LEVEL"));
    assert_eq!(selected["ISOS_HTTP_PORT"], "16000");
    assert!(config_values("[server]\nhttp_port=80\n", root).is_err());
    assert!(config_values("[unknown]\nvalue=1\n", root).is_err());
    assert!(config_values("[logging]\nlevel='verbose'\n", root).is_err());
    assert!(config_values("[logging]\ndefault_source='../bad'\n", root).is_err());
    assert!(config_values("[logging]\nmax_sources=9\n", root).is_err());
    let defaults = StopBudget::from_deployment(&DeploymentConfig::default()).unwrap();
    assert_eq!(
      (defaults.timeout_ms, defaults.eos_grace_ms),
      (30_000, 20_000)
    );
    let error = config_values("[deployment]\nstop_timeout_ms=4999\n", root).unwrap_err();
    let diagnostic = error.downcast_ref::<StopBudgetDiagnostic>().unwrap();
    assert_eq!(diagnostic.path, "/deployment/stop_timeout_ms");
    let error = config_values(
      "[deployment]\nstop_timeout_ms=5000\neos_stop_grace_ms=3000\n",
      root,
    )
    .unwrap_err();
    let diagnostic = error.downcast_ref::<StopBudgetDiagnostic>().unwrap();
    assert_eq!(diagnostic.path, "/deployment/eos_stop_grace_ms");
    assert_eq!(
      runtime_config_path(root, None).unwrap(),
      root.join("../../config/isos/isos.toml")
    );
    let legacy = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(legacy.path().join("config")).unwrap();
    std::fs::write(
      legacy.path().join("config/isos.toml"),
      "[server]\nhttp_port=16002\n",
    )
    .unwrap();
    assert_eq!(
      runtime_config_path(legacy.path(), None).unwrap(),
      legacy.path().join("../../config/isos/isos.toml")
    );
    assert_eq!(
      runtime_config_path(root, Some(std::ffi::OsStr::new("/tmp/custom.toml"))).unwrap(),
      Path::new("/tmp/custom.toml")
    );
    assert!(runtime_config_path(root, Some(std::ffi::OsStr::new("custom.toml"))).is_err());
    for source in [
      include_str!("../../../../config/isos/isos.toml"),
      include_str!("../../../../config/isos/isos.example.toml"),
    ] {
      let parsed = config_values(source, root).unwrap();
      for key in ["ISOSUI_REPO_PATH", "ISOS_AETHER_BIN_DIR"] {
        if let Some(path) = parsed.get(key) {
          assert!(Path::new(path).is_absolute());
        }
      }
    }
  }

  #[test]
  fn selected_config_overrides_example_without_copying_comments() {
    let dir = tempfile::tempdir().unwrap();
    let selected = dir.path().join("selected.toml");
    fs::write(&selected, "# local note\n[server]\nhttp_port = 17123\n").unwrap();
    let merged = merged_config_source(&selected, true).unwrap();
    let values = config_values(&merged, dir.path()).unwrap();
    assert_eq!(values["ISOS_HTTP_PORT"], "17123");
    assert_eq!(values["ISOSUI_PORT"], "16001");
    assert_eq!(values["ISOS_LOG_LEVEL"], "debug");
    assert!(!merged.contains("local note"));
    fs::remove_file(&selected).unwrap();
    assert!(merged_config_source(&selected, true).is_err());
    assert!(merged_config_source(&selected, false).is_ok());
  }

  #[test]
  fn package_host_inputs_are_generic_absolute_paths() {
    let root = Path::new("/tmp/isos-source");
    let values = config_values("[package_host_inputs]\nSIM_ROOT='/opt/simulator'\n", root).unwrap();
    assert_eq!(values["SIM_ROOT"], "/opt/simulator");
    for key in [
      "ISOS_HOME",
      "ISLAND_PREFIX",
      "EI_PYTHON",
      "lower",
      "BAD-DASH",
    ] {
      let source = format!("[package_host_inputs]\n{key}='/opt/simulator'\n");
      assert!(config_values(&source, root).is_err(), "accepted {key}");
    }
    assert!(config_values("[package_host_inputs]\nSIM_ROOT='relative'\n", root).is_err());
  }

  #[test]
  fn ei_paths_resolve_against_repository() {
    let root = Path::new("/tmp/isos-source");
    let values = config_values(
      "[paths]\nei_bin_dir='bin/ei'\nei_defaults_dir='etc/ei'\n",
      root,
    )
    .unwrap();
    assert_eq!(values["ISOS_EI_BIN_DIR"], "/tmp/isos-source/bin/ei");
    assert_eq!(values["ISOS_EI_DEFAULTS_DIR"], "/tmp/isos-source/etc/ei");
  }

  #[test]
  fn legacy_graph_equal_to_demo_uses_one_pool_file() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    let demo = br#"{"version":"0.2.0","robot":{"id":"same"}}"#;
    atomic_write(&paths.graph_pool_path("demo"), demo).unwrap();
    let legacy = paths.config.join("deployments/default.nodes.json");
    atomic_write(&legacy, demo).unwrap();
    paths.prepare().unwrap();
    assert_eq!(paths.bound_graph_name("default"), "demo");
    assert_eq!(fs::read(paths.node_graph_path("default")).unwrap(), demo);
    assert!(!legacy.exists());
    assert_eq!(
      fs::read_dir(paths.config.join("graph-pool"))
        .unwrap()
        .count(),
      1
    );
  }

  #[test]
  fn distinct_legacy_graph_is_imported_without_overwriting_demo() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    atomic_write(&paths.graph_pool_path("demo"), b"{\"id\":\"demo\"}").unwrap();
    let legacy = paths.config.join("deployments/default.nodes.json");
    let old = b"{\"id\":\"newer\"}\n";
    atomic_write(&legacy, old).unwrap();
    paths.prepare().unwrap();
    assert_eq!(paths.bound_graph_name("default"), "default-migrated");
    assert_eq!(fs::read(paths.node_graph_path("default")).unwrap(), old);
    assert!(!legacy.exists());
    assert_eq!(
      fs::read(paths.graph_pool_path("demo")).unwrap(),
      b"{\"id\":\"demo\"}"
    );
  }

  #[test]
  fn whitespace_changed_legacy_graph_preserves_exact_revision() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    let demo = b"{\"id\":\"same\"}";
    let legacy_bytes = b"{\"id\":\"same\"}\n";
    atomic_write(&paths.graph_pool_path("demo"), demo).unwrap();
    atomic_write(
      &paths.config.join("deployments/default.nodes.json"),
      legacy_bytes,
    )
    .unwrap();
    paths.prepare().unwrap();
    assert_eq!(paths.bound_graph_name("default"), "default-migrated");
    assert_eq!(
      fs::read(paths.node_graph_path("default")).unwrap(),
      legacy_bytes
    );
    assert_eq!(fs::read(paths.graph_pool_path("demo")).unwrap(), demo);
  }

  #[test]
  fn malformed_binding_does_not_select_demo() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    atomic_write(&paths.graph_binding_path("default"), b"not json").unwrap();
    assert_eq!(paths.bound_graph_name("default"), "__invalid_binding__");
    assert_ne!(
      paths.node_graph_path("default"),
      paths.graph_pool_path("demo")
    );
  }

  #[test]
  fn concurrent_prepare_migrates_legacy_once() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    let legacy_bytes = b"{\"id\":\"legacy\"}";
    atomic_write(
      &paths.config.join("deployments/default.nodes.json"),
      legacy_bytes,
    )
    .unwrap();
    let first = paths.clone();
    let second = paths.clone();
    let a = std::thread::spawn(move || first.prepare());
    let b = std::thread::spawn(move || second.prepare());
    a.join().unwrap().unwrap();
    b.join().unwrap().unwrap();
    assert_eq!(paths.bound_graph_name("default"), "default-migrated");
    assert_eq!(
      fs::read(paths.node_graph_path("default")).unwrap(),
      legacy_bytes
    );
    assert_eq!(
      fs::read_dir(paths.config.join("graph-pool"))
        .unwrap()
        .count(),
      1
    );
  }

  #[test]
  fn conflicting_existing_binding_preserves_legacy_file() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    let legacy = paths.config.join("deployments/default.nodes.json");
    atomic_write(&legacy, b"{\"id\":\"legacy\"}").unwrap();
    atomic_write(&paths.graph_binding_path("default"), b"invalid").unwrap();
    assert!(paths.prepare().is_err());
    assert_eq!(fs::read(&legacy).unwrap(), b"{\"id\":\"legacy\"}");
  }
}
