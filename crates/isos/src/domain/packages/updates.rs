//! Local package update snapshot. The catalog is queried only for installed IDs.

use super::{CATALOG, listing};
use crate::app::config::{self, Paths};
use crate::domain::registry::release::Release;
use anyhow::{Context, Result, ensure};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, time::Duration};

const MAX_SNAPSHOT_BYTES: usize = 1024 * 1024;
const MAX_PACKAGE_IDS: usize = 256;
const MAX_PATCH_PROBES: usize = 32;
const MAX_CATALOG_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UpdateRow {
  pub id: String,
  pub installed_versions: Vec<String>,
  pub latest_version: Option<String>,
  pub update_version: Option<String>,
  pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateSnapshot {
  pub schema_version: u32,
  pub rows: Vec<UpdateRow>,
}

fn snapshot_path(paths: &Paths) -> PathBuf {
  paths.data.join("state/package-updates.json")
}

fn inventory(paths: &Paths) -> Result<BTreeMap<String, Vec<String>>> {
  let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
  for row in listing(paths)? {
    if row["status"] != "verified" {
      continue;
    }
    let (Some(id), Some(version)) = (row["id"].as_str(), row["version"].as_str()) else {
      continue;
    };
    found
      .entry(id.to_owned())
      .or_default()
      .push(version.to_owned());
  }
  ensure!(
    found.len() <= MAX_PACKAGE_IDS,
    "too many installed package IDs for update"
  );
  for versions in found.values_mut() {
    versions.sort();
    versions.dedup();
  }
  Ok(found)
}

fn newer_than_installed(latest: &str, installed: &[String]) -> Result<bool> {
  let latest = Version::parse(latest).context("catalog latest version is not semantic")?;
  let highest = installed
    .iter()
    .map(|version| Version::parse(version).context("installed package version is not semantic"))
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .max();
  Ok(highest.is_some_and(|version| latest > version))
}

fn checked_catalog_version(requested_id: &str, actual_id: &str, version: String) -> Result<String> {
  ensure!(
    actual_id == requested_id,
    "catalog returned a different package ID"
  );
  Ok(version)
}

fn exact_catalog_release(id: &str, version: &str) -> Result<Option<String>> {
  let (namespace, name) = id.split_once('/').context("invalid installed package ID")?;
  let url = format!("{CATALOG}/api/packages/v1/packages/{namespace}/{name}/releases/{version}");
  let response = reqwest::blocking::Client::builder()
    .timeout(Duration::from_secs(10))
    .redirect(reqwest::redirect::Policy::none())
    .build()?
    .get(url)
    .send()
    .context("query exact package release")?;
  if response.status().as_u16() == 404 {
    return Ok(None);
  }
  ensure!(
    response.status().is_success(),
    "catalog returned HTTP {}",
    response.status()
  );
  let mut bytes = Vec::new();
  response
    .take(MAX_CATALOG_BYTES + 1)
    .read_to_end(&mut bytes)?;
  ensure!(
    bytes.len() as u64 <= MAX_CATALOG_BYTES,
    "catalog release response too large"
  );
  let envelope: serde_json::Value = serde_json::from_slice(&bytes)?;
  ensure!(
    envelope["success"] == true,
    "catalog release was not approved"
  );
  let release: Release = serde_json::from_value(envelope["data"].clone())
    .context("catalog release lacks the current IsOS package contract")?;
  release.validate()?;
  ensure!(
    release.version == version,
    "catalog returned a different package version"
  );
  Ok(Some(checked_catalog_version(
    id,
    &release.id,
    release.version,
  )?))
}

fn next_patch_releases<F>(id: &str, installed: &[String], mut fetch: F) -> Result<Option<String>>
where
  F: FnMut(&str, &str) -> Result<Option<String>>,
{
  let mut current = installed
    .iter()
    .map(|version| Version::parse(version).context("installed package version is not semantic"))
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .max()
    .context("installed package version missing")?;
  ensure!(
    current.pre.is_empty() && current.build.is_empty(),
    "installed package version is not stable"
  );
  let mut latest = None;
  for _ in 0..MAX_PATCH_PROBES {
    current.patch = current
      .patch
      .checked_add(1)
      .context("package patch version exhausted")?;
    let requested = current.to_string();
    let Some(found) = fetch(id, &requested)? else {
      return Ok(latest);
    };
    ensure!(
      found == requested,
      "catalog returned a different package version"
    );
    latest = Some(found);
  }
  Ok(latest)
}

fn rows_for<F>(installed: BTreeMap<String, Vec<String>>, mut fetch: F) -> Vec<UpdateRow>
where
  F: FnMut(&str, &[String]) -> Result<Option<String>>,
{
  let mut rows = Vec::with_capacity(installed.len());
  for (id, installed_versions) in installed {
    let mut row = UpdateRow {
      id: id.clone(),
      installed_versions,
      latest_version: None,
      update_version: None,
      error: None,
    };
    match fetch(&id, &row.installed_versions) {
      Ok(Some(version)) => {
        row.latest_version = Some(version.clone());
        match newer_than_installed(&version, &row.installed_versions) {
          Ok(true) => row.update_version = Some(version),
          Ok(false) => (),
          Err(error) => row.error = Some(error.to_string()),
        }
      }
      Ok(None) => (),
      Err(error) => {
        let message = format!("catalog unavailable: {error:#}");
        row.error = Some(
          message
            .chars()
            .filter(|character| !character.is_control())
            .take(256)
            .collect(),
        );
      }
    }
    rows.push(row);
  }
  rows
}

pub fn refresh(paths: &Paths) -> Result<UpdateSnapshot> {
  let installed = inventory(paths)?;
  let rows = rows_for(installed, |id, versions| {
    next_patch_releases(id, versions, exact_catalog_release)
  });
  let snapshot = UpdateSnapshot {
    schema_version: 1,
    rows,
  };
  let bytes = serde_json::to_vec_pretty(&snapshot)?;
  ensure!(
    bytes.len() <= MAX_SNAPSHOT_BYTES,
    "package update snapshot too large"
  );
  config::atomic_write(&snapshot_path(paths), &bytes)?;
  Ok(snapshot)
}

pub fn selected(paths: &Paths, id: Option<&str>) -> Result<Vec<UpdateRow>> {
  let bytes = fs::read(snapshot_path(paths)).context("run `isos pkg update` first")?;
  ensure!(
    bytes.len() <= MAX_SNAPSHOT_BYTES,
    "package update snapshot too large"
  );
  let snapshot: UpdateSnapshot = serde_json::from_slice(&bytes)?;
  ensure!(
    snapshot.schema_version == 1,
    "unsupported package update snapshot"
  );
  ensure!(
    snapshot.rows.len() <= MAX_PACKAGE_IDS,
    "package update snapshot too large"
  );
  let current = inventory(paths)?;
  let saved: BTreeMap<_, _> = snapshot
    .rows
    .iter()
    .map(|row| (row.id.clone(), row.installed_versions.clone()))
    .collect();
  ensure!(
    saved.len() == snapshot.rows.len(),
    "duplicate package update entry"
  );
  ensure!(
    current == saved,
    "installed packages changed; run `isos pkg update` again"
  );
  for row in &snapshot.rows {
    if let Some(version) = &row.update_version {
      ensure!(
        row.latest_version.as_ref() == Some(version),
        "invalid package update entry"
      );
      ensure!(
        newer_than_installed(version, &row.installed_versions)?,
        "invalid package update version"
      );
    }
  }
  if let Some(id) = id {
    ensure!(
      current.contains_key(id),
      "package is not installed locally: {id}"
    );
  }
  Ok(
    snapshot
      .rows
      .into_iter()
      .filter(|row| id.is_none_or(|id| row.id == id))
      .filter(|row| row.update_version.is_some())
      .collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn latest_must_exceed_highest_installed_semver() {
    let installed = vec!["0.2.9".to_owned(), "0.2.10".to_owned()];
    assert!(!newer_than_installed("0.2.9", &installed).unwrap());
    assert!(!newer_than_installed("0.2.10", &installed).unwrap());
    assert!(newer_than_installed("0.2.11", &installed).unwrap());
    assert!(newer_than_installed("not-a-version", &installed).is_err());
    assert!(checked_catalog_version("vendor/a", "vendor/b", "1.0.1".into()).is_err());
  }

  #[test]
  fn update_queries_only_installed_ids_and_keeps_per_id_errors() {
    let installed = BTreeMap::from([
      ("island/mujoco-sim".to_owned(), vec!["0.2.2".to_owned()]),
      ("vendor/local".to_owned(), vec!["1.0.0".to_owned()]),
    ]);
    let mut queried = Vec::new();
    let rows = rows_for(installed, |id, _| {
      queried.push(id.to_owned());
      if id == "vendor/local" {
        anyhow::bail!("not published")
      }
      Ok(Some("0.2.3".to_owned()))
    });
    assert_eq!(queried, vec!["island/mujoco-sim", "vendor/local"]);
    assert_eq!(rows[0].update_version.as_deref(), Some("0.2.3"));
    assert!(
      rows[1]
        .error
        .as_deref()
        .is_some_and(|error| error.contains("not published"))
    );
  }

  #[test]
  fn patch_discovery_stops_at_first_missing_exact_release() {
    let mut queried = Vec::new();
    let result = next_patch_releases("island/mujoco-sim", &["0.2.1".to_owned()], |id, version| {
      queried.push((id.to_owned(), version.to_owned()));
      Ok(match version {
        "0.2.2" | "0.2.3" => Some(version.to_owned()),
        _ => None,
      })
    })
    .unwrap();
    assert_eq!(result.as_deref(), Some("0.2.3"));
    assert_eq!(
      queried
        .iter()
        .map(|(_, version)| version.as_str())
        .collect::<Vec<_>>(),
      vec!["0.2.2", "0.2.3", "0.2.4"]
    );
    assert!(
      next_patch_releases("island/mujoco-sim", &["0.2.2".to_owned()], |_, _| Ok(Some(
        "9.9.9".into()
      )))
      .is_err()
    );
  }

  #[test]
  fn patch_discovery_is_bounded_and_keeps_last_verified_release() {
    let mut probes = 0;
    let result = next_patch_releases("island/mujoco-sim", &["0.2.1".to_owned()], |_, version| {
      probes += 1;
      Ok(Some(version.to_owned()))
    })
    .unwrap();
    assert_eq!(probes, MAX_PATCH_PROBES);
    assert_eq!(result.as_deref(), Some("0.2.33"));
  }

  #[test]
  fn patch_discovery_does_not_skip_missing_release() {
    let mut queried = Vec::new();
    let result = next_patch_releases("island/mujoco-sim", &["0.2.1".to_owned()], |_, version| {
      queried.push(version.to_owned());
      Ok((version == "0.2.3").then(|| version.to_owned()))
    })
    .unwrap();
    assert_eq!(result, None);
    assert_eq!(queried, ["0.2.2"]);
  }

  #[test]
  fn empty_local_inventory_writes_snapshot_and_rejects_changed_inventory() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: home.path().join("config"),
      data: home.path().join("data"),
      runtime: home.path().join("run"),
    };
    let snapshot = refresh(&paths).unwrap();
    assert!(snapshot.rows.is_empty());
    assert!(selected(&paths, None).unwrap().is_empty());
    assert!(snapshot_path(&paths).is_file());
    let tampered = UpdateSnapshot {
      schema_version: 1,
      rows: vec![UpdateRow {
        id: "vendor/other".to_owned(),
        installed_versions: vec!["1.0.0".to_owned()],
        latest_version: Some("1.0.1".to_owned()),
        update_version: Some("1.0.1".to_owned()),
        error: None,
      }],
    };
    fs::write(
      snapshot_path(&paths),
      serde_json::to_vec(&tampered).unwrap(),
    )
    .unwrap();
    assert!(selected(&paths, None).is_err());
    let duplicate = UpdateSnapshot {
      schema_version: 1,
      rows: vec![tampered.rows[0].clone(), tampered.rows[0].clone()],
    };
    fs::write(
      snapshot_path(&paths),
      serde_json::to_vec(&duplicate).unwrap(),
    )
    .unwrap();
    assert!(
      selected(&paths, None)
        .unwrap_err()
        .to_string()
        .contains("duplicate")
    );
  }
}
