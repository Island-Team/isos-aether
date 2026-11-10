use crate::{app::config, domain::version};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::json;
use std::{fs, os::unix::net::UnixStream, path::Path, process::Command};

#[derive(Deserialize)]
struct InstallSettings {
  install: InstallSection,
}

#[derive(Deserialize)]
struct InstallSection {
  release_repository: String,
  channel: String,
}

#[derive(Deserialize)]
struct Release {
  tag_name: String,
  prerelease: bool,
  draft: bool,
  assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
  name: String,
  browser_download_url: String,
}

fn settings() -> Result<InstallSettings> {
  let executable = std::env::current_exe()?.canonicalize()?;
  let installed = executable
    .parent()
    .and_then(|path| path.parent())
    .map(|path| path.join("share/isos/config/install.toml"));
  let selected = std::env::var_os("ISOS_INSTALL_CONFIG")
    .map(std::path::PathBuf::from)
    .or_else(|| installed.filter(|path| path.is_file()));
  let source = match selected {
    Some(path) => fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?,
    None => include_str!("../../../../../config/isos/install.toml").to_owned(),
  };
  settings_from_source(&source, |name| std::env::var(name).ok())
}

fn settings_from_source(
  source: &str,
  get: impl Fn(&str) -> Option<String>,
) -> Result<InstallSettings> {
  let mut settings: InstallSettings = toml::from_str(source)?;
  if let Some(value) = get("ISOS_RELEASE_REPOSITORY") {
    settings.install.release_repository = value;
  }
  if let Some(value) = get("ISOS_RELEASE_CHANNEL") {
    settings.install.channel = value;
  }
  ensure!(
    settings.install.release_repository.split('/').count() == 2
      && settings
        .install
        .release_repository
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/')),
    "invalid release repository"
  );
  ensure!(
    matches!(settings.install.channel.as_str(), "stable" | "prerelease"),
    "invalid release channel"
  );
  Ok(settings)
}

fn releases() -> Result<Vec<Release>> {
  let settings = settings()?;
  let url = format!(
    "https://api.github.com/repos/{}/releases?per_page=20",
    settings.install.release_repository
  );
  let client = reqwest::blocking::Client::builder()
    .user_agent("isos-updater")
    .build()?;
  let entries: Vec<Release> = client.get(url).send()?.error_for_status()?.json()?;
  Ok(
    entries
      .into_iter()
      .filter(|entry| {
        !entry.draft && (settings.install.channel == "prerelease" || !entry.prerelease)
      })
      .collect(),
  )
}

pub(crate) fn install(source: &Path) -> Result<()> {
  let source = source
    .canonicalize()
    .with_context(|| format!("source path {}", source.display()))?;
  let installer = source.join("install.sh");
  ensure!(
    installer.is_file(),
    "{} does not contain root install.sh",
    source.display()
  );
  let status = Command::new("bash")
    .arg(installer)
    .current_dir(&source)
    .status()?;
  ensure!(status.success(), "source installation failed: {status}");
  Ok(())
}

pub(crate) fn update() -> Result<()> {
  let entries = releases()?;
  println!("Installed IsOS {}", env!("CARGO_PKG_VERSION"));
  for entry in entries.iter().take(10) {
    println!(
      "{}{}",
      entry.tag_name,
      if entry.prerelease {
        " (prerelease)"
      } else {
        ""
      }
    );
  }
  if entries.is_empty() {
    println!("No releases available");
  }
  Ok(())
}

pub(crate) fn upgrade(version: Option<&str>) -> Result<()> {
  let entries = releases()?;
  let installed = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
  let chosen = match version {
    Some(version) => entries
      .iter()
      .find(|entry| entry.tag_name == version || entry.tag_name == format!("v{version}")),
    None => entries
      .iter()
      .filter_map(|entry| {
        semver::Version::parse(entry.tag_name.trim_start_matches('v'))
          .ok()
          .filter(|candidate| candidate > &installed)
          .map(|candidate| (candidate, entry))
      })
      .max_by(|left, right| left.0.cmp(&right.0))
      .map(|(_, entry)| entry),
  }
  .context("requested release is unavailable")?;
  let target = semver::Version::parse(chosen.tag_name.trim_start_matches('v'))
    .context("release tag is not a semantic version")?;
  ensure!(
    target > installed,
    "release {target} is not newer than installed {installed}"
  );
  let arch = match std::env::consts::ARCH {
    "x86_64" => "x86_64",
    "aarch64" => "aarch64",
    other => bail!("unsupported architecture: {other}"),
  };
  let bundle_name = format!(
    "isos-{}-linux-{arch}.tar.gz",
    chosen.tag_name.trim_start_matches('v')
  );
  let checksum_name = format!("{bundle_name}.sha256");
  let bundle = chosen
    .assets
    .iter()
    .find(|asset| asset.name == bundle_name)
    .context("release bundle missing")?;
  let checksum = chosen
    .assets
    .iter()
    .find(|asset| asset.name == checksum_name)
    .context("release checksum missing")?;
  let client = reqwest::blocking::Client::builder()
    .user_agent("isos-updater")
    .build()?;
  let bundle_bytes = client
    .get(&bundle.browser_download_url)
    .send()?
    .error_for_status()?
    .bytes()?;
  let checksum_text = client
    .get(&checksum.browser_download_url)
    .send()?
    .error_for_status()?
    .text()?;
  use sha2::Digest;
  let actual = hex::encode(sha2::Sha256::digest(&bundle_bytes));
  let expected = checksum_text
    .split_whitespace()
    .next()
    .context("empty release checksum")?;
  ensure!(actual == expected, "release checksum mismatch");
  let temporary_root = config::Paths::discover()?.data.join("tmp");
  fs::create_dir_all(&temporary_root)?;
  let download = tempfile::tempdir_in(&temporary_root)?;
  let archive = download.path().join(&bundle_name);
  fs::write(&archive, bundle_bytes)?;
  let executable = std::env::current_exe()?.canonicalize()?;
  let installed = executable
    .parent()
    .and_then(|path| path.parent())
    .map(|path| path.join("share/isos/install.sh"));
  let source = installed
    .filter(|path| path.is_file())
    .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install.sh"));
  ensure!(
    source.is_file(),
    "root installer unavailable; use source install"
  );
  let status = Command::new("bash")
    .arg(source)
    .arg("--bundle")
    .arg(&archive)
    .args(
      executable
        .ancestors()
        .nth(4)
        .filter(|prefix| prefix.join("releases").is_dir())
        .map(|prefix| vec!["--prefix".into(), prefix.as_os_str().to_os_string()])
        .unwrap_or_default(),
    )
    .status()?;
  ensure!(status.success(), "release upgrade failed: {status}");
  Ok(())
}

pub(crate) fn doctor(paths: &config::Paths, _deployment: &str) -> Result<()> {
  let support = version::bundled()?;
  let manager = if UnixStream::connect(paths.socket()).is_ok() {
    "running"
  } else {
    "stopped"
  };
  let ui_settings = crate::app::isosui::ui_settings()?;
  let ui_ready = std::env::var_os("ISOSUI_DIST_DIR")
    .map(std::path::PathBuf::from)
    .is_some_and(|path| path.join("index.html").is_file());
  let eos_ready = std::env::var_os("ISOS_AETHER_BIN_DIR")
    .map(std::path::PathBuf::from)
    .is_some_and(|path| {
      path.join("aether-profile").is_file() && path.join("aether-runtime").is_file()
    });
  let ei_ready = std::env::var_os("ISOS_EI_BIN_DIR")
    .map(std::path::PathBuf::from)
    .is_some_and(|path| path.join("ei-config").is_file());
  println!(
    "{}",
    serde_json::to_string_pretty(&json!({
      "isos_version": env!("CARGO_PKG_VERSION"),
      "manager": manager,
      "mode": paths.mode()?,
      "interfaces": support,
      "ui_settings": {"default_interface": ui_settings.default_interface, "locale": ui_settings.locale},
      "assets": {"ui": ui_ready, "eos": eos_ready, "ei": ei_ready}
    }))?
  );
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn release_environment_overrides_toml_and_is_validated() {
    let source = include_str!("../../../../../config/isos/install.toml");
    let settings = settings_from_source(source, |name| {
      match name {
        "ISOS_RELEASE_REPOSITORY" => Some("Island-Team/test-releases"),
        "ISOS_RELEASE_CHANNEL" => Some("prerelease"),
        _ => None,
      }
      .map(str::to_owned)
    })
    .unwrap();
    assert_eq!(
      settings.install.release_repository,
      "Island-Team/test-releases"
    );
    assert_eq!(settings.install.channel, "prerelease");
    assert!(
      settings_from_source(source, |name| {
        (name == "ISOS_RELEASE_CHANNEL").then(|| "unknown".to_owned())
      })
      .is_err()
    );
  }
}
