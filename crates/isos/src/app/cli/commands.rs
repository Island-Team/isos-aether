use super::{
  ConfigCommand, InstallArgs, LibraryCommand, PackageCommand, PackageConfigCommand, ServiceCommand,
};
use crate::runtime::manager::{manager_session_id, next_correlation_id, service_call};
use crate::{
  app::config, runtime::owners::core_setup, domain::graph::graph_contract,
  domain::graph::graph_apply, runtime::logs as log_store, domain::graph::node_sets,
  domain::packages, runtime::process, domain::registry::release, domain::services,
  domain::graph::starter,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
  fs,
  io::{self, IsTerminal, Write},
  os::unix::net::UnixStream,
  path::Path,
};

pub(crate) fn config_command(paths: &config::Paths, name: &str, cmd: ConfigCommand) -> Result<()> {
  match cmd {
    ConfigCommand::Init { output } => {
      let default = starter::init(paths, name)?;
      if let Some(output) = output {
        ensure!(!output.exists(), "{} already exists", output.display());
        config::atomic_write(&output, &fs::read(default)?)?;
        println!("created {}", output.display());
      } else {
        if let Some(revision) = core_setup::refresh_after_init(paths, name)? {
          println!(
            "refreshed core launch inputs for graph {revision}; Run or Restart to use the configured build"
          );
          if let Err(error) = clear_saved_setup_error(paths, name) {
            eprintln!("core inputs refreshed, but could not clear prior setup error: {error}");
          }
        }
        println!("editable graph: {}", default.display());
      }
    }
    ConfigCommand::Validate { file } => {
      let source = fs::read_to_string(&file)?;
      let graph = process::normalize_source(&source)?;
      ensure!(graph["version"] == "0.2.1", "graph version must be 0.2.1");
      let bytes = serde_json::to_vec_pretty(&graph)?;
      let proposal = graph_apply::Proposal {
        revision: graph_contract::revision(&bytes),
        graph,
        bytes,
        warnings: Vec::new(),
      };
      let prepared = graph_apply::prepare(paths, name, &proposal)
        .map_err(|diagnostic| anyhow::anyhow!(diagnostic.to_string()))?;
      println!("valid node graph");
      for warning in prepared.warnings {
        println!(
          "warning: {}: {}: {}",
          warning.path, warning.code, warning.message
        );
      }
    }
    ConfigCommand::Apply { file, revision } => {
      apply_graph_file(paths, name, &file, revision.as_deref()).inspect_err(|_| {
        if let Ok(logs) = log_store::LogStore::new(paths, name) {
          let correlation = next_correlation_id();
          let manager = logs
            .manager_scope_id()
            .unwrap_or_else(|| manager_session_id().to_owned());
          let _ = logs.append_apply_failure_context(
            "isos",
            "error",
            "deployment_preflight_failed",
            "Graph Apply failed; inspect the command diagnostic",
            log_store::LogContext {
              run_id: Some(&manager),
              correlation_id: Some(&correlation),
              node_path: None,
              fields: json!({}),
            },
          );
        }
      })?;
      println!("applied node graph");
    }
    ConfigCommand::Show => {
      println!("{}", fs::read_to_string(paths.node_graph_path(name))?);
    }
  }
  Ok(())
}

pub(crate) fn apply_graph_file(
  paths: &config::Paths,
  name: &str,
  file: &Path,
  expected_revision: Option<&str>,
) -> Result<()> {
  let pool = fs::canonicalize(paths.config.join("graph-pool"))?;
  let selected = fs::canonicalize(file)?;
  ensure!(
    selected.parent() == Some(pool.as_path()),
    "Apply file must be in the graph pool"
  );
  let filename = selected
    .file_name()
    .and_then(|value| value.to_str())
    .context("invalid graph pool filename")?;
  let pool_name = filename
    .strip_suffix(".nodes.json")
    .context("Apply requires a .nodes.json pool file")?;
  process::validate_deployment_name(pool_name)?;
  let expected = expected_revision.context("graph Apply requires --revision")?;
  graph_apply::apply_pool(paths, name, pool_name, expected)
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
  Ok(())
}

pub(crate) fn clear_saved_setup_error(paths: &config::Paths, name: &str) -> Result<()> {
  let path = paths.runtime.join(format!("{name}.last-error"));
  match fs::remove_file(path) {
    Ok(()) => Ok(()),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(error) => Err(error.into()),
  }
}

pub(crate) fn install_package(
  paths: &config::Paths,
  mut args: InstallArgs,
  kind: Option<&str>,
) -> Result<()> {
  if let Some(raw) = args.id.clone()
    && let Some((id, version)) = raw.split_once('@')
  {
    ensure!(
      !id.is_empty() && !version.is_empty() && !version.contains('@'),
      "package must be namespace/name@version"
    );
    ensure!(
      args.version == "latest" || args.version == version,
      "package version conflicts with --version"
    );
    args.id = Some(id.to_owned());
    args.version = version.to_owned();
  }
  if let Some(source) = &args.source {
    ensure!(
      kind.is_none(),
      "source installation is available through isos pkg install"
    );
    ensure!(
      args.allow_source_build,
      "--source requires --allow-source-build"
    );
    ensure!(
      args.manifest.is_none()
        && args.asset.is_none()
        && !args.allow_unapproved
        && args.approve_download.is_none(),
      "--source cannot be combined with --manifest, --asset, --allow-unapproved, or --approve-download"
    );
    let expected_version = (args.version != "latest").then_some(args.version.as_str());
    let (release, path) =
      packages::install_source(paths, source, args.id.as_deref(), expected_version)?;
    println!(
      "installed {} {} at {}",
      release.id,
      release.version,
      path.display()
    );
    println!("Add the package to a deployment graph and run isos config apply");
    return Ok(());
  }
  ensure!(
    !args.allow_source_build,
    "--allow-source-build requires --source"
  );
  let release = match (&args.id, &args.manifest) {
    (Some(id), None) => packages::catalog_release(id, &args.version)?,
    (_, Some(file)) => {
      ensure!(
        args.allow_unapproved,
        "--manifest requires --allow-unapproved"
      );
      let release = packages::read_manifest(file)?;
      if let Some(id) = &args.id {
        ensure!(release.id == *id, "manifest ID differs from requested ID");
      }
      release
    }
    (None, None) => bail!("provide a package ID or --manifest <file>"),
  };
  if let Some(kind) = kind {
    ensure!(
      release.kind == kind,
      "{} is a {} package, expected {kind}",
      release.id,
      release.kind
    );
  }
  if args.asset.is_none() {
    ensure!(
      args.manifest.is_none(),
      "--manifest requires a local --asset; catalog downloads use an approved release"
    );
    ensure!(
      !args.allow_unapproved,
      "catalog download requires an approved release"
    );
    let preview = packages::download_preview(&release)?;
    println!(
      "Package: {}@{}\nPublisher: {}\nSource: {}\nAsset: {}\nSize: {} bytes\nSHA-256: {}\nTrust: {}\nApproval: {}",
      preview.id,
      preview.version,
      preview.publisher,
      preview.source_url.as_deref().unwrap_or("unspecified"),
      preview.url,
      preview.size_bytes,
      preview.sha256,
      preview.trust,
      preview.approval
    );
    if let Some(approval) = &args.approve_download {
      ensure!(
        approval == &preview.approval,
        "download approval does not match the selected release asset"
      );
    } else {
      ensure!(
        io::stdin().is_terminal(),
        "noninteractive download requires --approve-download <approval digest>"
      );
      print!("Download this exact asset? Type yes: ");
      io::stdout().flush()?;
      let mut answer = String::new();
      io::stdin().read_line(&mut answer)?;
      ensure!(answer.trim() == "yes", "package download cancelled");
    }
  } else {
    ensure!(
      args.approve_download.is_none(),
      "--approve-download applies only to catalog downloads"
    );
  }
  let path = if let Some(asset) = args.asset.as_deref() {
    ensure!(args.allow_unapproved, "--asset requires --allow-unapproved");
    packages::install_local(paths, &release, asset)?
  } else {
    let approval = packages::download_preview(&release)?.approval;
    packages::install_approved(paths, &release, &approval)?
  };
  println!(
    "installed {} {} at {}",
    release.id,
    release.version,
    path.display()
  );
  println!("Add the package to a deployment graph and run isos config apply");
  Ok(())
}

pub(crate) fn package_command(
  paths: &config::Paths,
  deployment: &str,
  cmd: PackageCommand,
) -> Result<()> {
  match cmd {
    PackageCommand::Check {
      source,
      version,
      output,
      graph,
      package_config,
    } => {
      let result = packages::check_source(
        &source,
        &version,
        &output,
        &graph,
        package_config.as_deref(),
      )?;
      println!("{}", serde_json::to_string_pretty(&result)?);
      ensure!(
        result["qualified"] == true,
        "package check requires a passing managed Apply, readiness, interface, logs, and Stop replay before release"
      );
    }
    PackageCommand::Release { checked } => packages::release_checked(paths, &checked)?,
    PackageCommand::Upload { manifest, asset } => packages::upload(paths, &manifest, &asset)?,
    PackageCommand::Search { query } => println!(
      "{}",
      serde_json::to_string_pretty(&packages::search(&query)?)?
    ),
    PackageCommand::Info { id, version } => println!(
      "{}",
      serde_json::to_string_pretty(&packages::catalog_release(&id, &version)?)?
    ),
    PackageCommand::RefreshSources => {
      let graph_path = paths.node_graph_path("default");
      if graph_path.exists() {
        let graph: Value = serde_json::from_slice(&fs::read(&graph_path)?)?;
        packages::refresh_graph_sources(paths, &graph)?;
        println!("source-installed packages for the bound graph are current");
      } else {
        println!("no bound graph; no source packages to refresh");
      }
    }
    PackageCommand::Update => {
      let snapshot = packages::updates::refresh(paths)?;
      if snapshot.rows.is_empty() {
        println!("no verified packages installed locally");
      }
      for row in snapshot.rows {
        if let Some(version) = row.update_version {
          println!(
            "{}: {} available (installed: {})",
            row.id,
            version,
            row.installed_versions.join(", ")
          );
        } else if let Some(error) = row.error {
          println!("{}: {error}", row.id);
        } else {
          println!(
            "{}: up to date (installed: {})",
            row.id,
            row.installed_versions.join(", ")
          );
        }
      }
    }
    PackageCommand::Upgrade {
      id,
      approve_download,
    } => {
      ensure!(
        id.is_some() || approve_download.is_none(),
        "--approve-download requires one package ID"
      );
      let selected = packages::updates::selected(paths, id.as_deref())?;
      if selected.is_empty() {
        println!("no catalog upgrades in the local update list");
      }
      for row in selected {
        let version = row
          .update_version
          .context("selected update lacks version")?;
        let release = packages::catalog_release(&row.id, &version)?;
        ensure!(
          release.id == row.id && release.version == version,
          "catalog release changed; run `isos pkg update` again"
        );
        install_package(
          paths,
          InstallArgs {
            id: Some(row.id),
            version,
            manifest: None,
            asset: None,
            source: None,
            allow_source_build: false,
            allow_unapproved: false,
            approve_download: approve_download.clone(),
          },
          None,
        )?;
      }
    }
    PackageCommand::Install(args) => install_package(paths, args, None)?,
    PackageCommand::Configure { pin, service } => {
      let (id, version) = package_pin(&pin)?;
      let saved = packages::package_config::show(paths, deployment, id, version)?;
      if let Some(service) = &service {
        ensure!(
          saved
            .descriptor
            .sections
            .iter()
            .any(|section| section.id == format!("services.{service}")),
          "package service {service} is not declared"
        );
      }
      let mut source = saved.toml.clone();
      if service.as_deref()
        == saved
          .descriptor
          .setup
          .as_ref()
          .map(|setup| setup.service.as_str())
        && packages::package_config::setup_needed(&saved)
        && io::stdin().is_terminal()
      {
        let setup = saved
          .descriptor
          .setup
          .as_ref()
          .expect("setup_needed requires setup");
        print!("{}", setup.prompt.en);
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if answer.trim().eq_ignore_ascii_case("y") {
          source = packages::package_config::run_setup(paths, &saved)?;
        }
      }
      let editor = ["VISUAL", "EDITOR"]
        .into_iter()
        .find_map(|name| {
          std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "nano".into());
      let temp = tempfile::Builder::new()
        .prefix("isos-package-config-")
        .suffix(".toml")
        .tempfile()?;
      fs::write(temp.path(), &source)?;
      let editor_parts = shlex::split(&editor).context("invalid VISUAL or EDITOR command")?;
      let (program, arguments) = editor_parts
        .split_first()
        .context("VISUAL or EDITOR must name an editor")?;
      let status = std::process::Command::new(program)
        .args(arguments)
        .arg(temp.path())
        .status()
        .context("could not start config editor; set VISUAL or EDITOR to an installed editor")?;
      ensure!(status.success(), "config editor failed");
      let source = fs::read_to_string(temp.path())?;
      if source == saved.toml {
        println!("package config unchanged for {id}@{version}");
        return Ok(());
      }
      let updated = match packages::package_config::save(
        paths,
        deployment,
        id,
        version,
        &saved.revision,
        &source,
      ) {
        Ok(updated) => updated,
        Err(error) => {
          let draft = temp
            .into_temp_path()
            .keep()
            .context("could not retain invalid package config draft")?;
          bail!(
            "package config was not saved: {error:#}\nYour edited draft is at {}. Correct it there, run `isos pkg config validate {id}@{version} {}`, then run `isos pkg config import {id}@{version} {} --revision {}`.",
            draft.display(),
            draft.display(),
            draft.display(),
            saved.revision
          );
        }
      };
      println!(
        "saved {}@{} package config revision {}",
        id, version, updated.revision
      );
    }
    PackageCommand::Config { command } => match command {
      PackageConfigCommand::Show { pin } => {
        let (id, version) = package_pin(&pin)?;
        let saved = packages::package_config::show(paths, deployment, id, version)?;
        println!("{}", saved.toml);
      }
      PackageConfigCommand::Validate { pin, file } => {
        let (id, version) = package_pin(&pin)?;
        let saved = packages::package_config::show(paths, deployment, id, version)?;
        packages::package_config::validate(&saved.descriptor, &fs::read_to_string(file)?, &[])?;
        println!("valid package config for {id}@{version}");
      }
      PackageConfigCommand::Import {
        pin,
        file,
        revision,
      } => {
        let (id, version) = package_pin(&pin)?;
        let updated = packages::package_config::save(
          paths,
          deployment,
          id,
          version,
          &revision,
          &fs::read_to_string(file)?,
        )?;
        println!(
          "saved {}@{} package config revision {}",
          id, version, updated.revision
        );
      }
    },
    PackageCommand::Link {
      id,
      version,
      path,
      manifest,
    } => {
      let installed =
        packages::link_with_manifest(paths, &id, &version, &path, manifest.as_deref())?;
      println!("linked {} {} at {}", id, version, installed.display());
    }
    PackageCommand::List { json } => {
      let receipts = packages::listing(paths)?;
      if json {
        println!("{}", serde_json::to_string_pretty(&receipts)?);
      } else if receipts.is_empty() {
        println!("no installed packages");
      } else {
        for receipt in receipts {
          println!(
            "{:<36} {:<14} {:<24} {}",
            receipt["id"].as_str().unwrap_or("unknown"),
            receipt["version"].as_str().unwrap_or("unknown"),
            receipt["status"].as_str().unwrap_or("unavailable"),
            receipt["source_kind"].as_str().unwrap_or("unknown source")
          );
        }
      }
    }
    PackageCommand::NodeSets => println!(
      "{}",
      serde_json::to_string_pretty(&node_sets::list(paths)?)?
    ),
    PackageCommand::PreviewSet {
      id,
      version,
      set_id,
      graph,
      values,
    } => {
      let graph = process::normalize_source(&fs::read_to_string(graph)?)?;
      let values = values
        .map(
          |file| -> Result<std::collections::BTreeMap<String, String>> {
            Ok(serde_json::from_slice(&fs::read(file)?)?)
          },
        )
        .transpose()?
        .unwrap_or_default();
      let preview = node_sets::compose(paths, &id, &version, &set_id, &graph, &values)?;
      process::validate_value(paths, &preview["graph"])?;
      println!("{}", serde_json::to_string_pretty(&preview)?);
    }
    PackageCommand::Verify { manifest, asset } => {
      let release = packages::read_manifest(&manifest)?;
      let selected = release.select_asset(&release::Platform::current())?;
      release::verify_file(selected, &asset)?;
      println!(
        "verified {} {} for this platform",
        release.id, release.version
      );
    }
    PackageCommand::Publish { manifest } => packages::publish(paths, &manifest)?,
    PackageCommand::Remove { id, version } => {
      packages::remove(paths, &id, &version)?;
      println!("removed {id} {version}");
    }
  }
  Ok(())
}

fn package_pin(pin: &str) -> Result<(&str, &str)> {
  let (id, version) = pin
    .rsplit_once('@')
    .context("package pin must be namespace/name@version")?;
  ensure!(!id.is_empty() && !version.is_empty(), "invalid package pin");
  Ok((id, version))
}

fn configured_nodes(paths: &config::Paths, name: &str, group_kind: &str) -> Result<Vec<Value>> {
  let graph: Value = serde_json::from_slice(&fs::read(paths.node_graph_path(name))?)?;
  let mut found = Vec::new();
  let cores = graph["cores"]
    .as_object()
    .context("node graph lacks cores")?;
  for (core_id, core) in cores {
    let Some(layers) = core["layers"].as_object() else {
      continue;
    };
    for (layer_id, layer) in layers {
      let Some(groups) = layer["groups"].as_object() else {
        continue;
      };
      for (group_id, group) in groups {
        if group_id != group_kind {
          continue;
        }
        let Some(nodes) = group["nodes"].as_object() else {
          continue;
        };
        for (node_id, node) in nodes {
          found.push(json!({
                        "id": node_id,
                        "path": format!("cores/{core_id}/layers/{layer_id}/groups/{group_id}/nodes/{node_id}"),
                        "core": core_id,
                        "enabled": node["enabled"].as_bool().unwrap_or(true),
                        "node": node,
                    }));
        }
      }
    }
  }
  Ok(found)
}

pub(crate) fn library(
  paths: &config::Paths,
  name: &str,
  kind: &str,
  command: LibraryCommand,
) -> Result<()> {
  let expected_kind = match kind {
    "skills" => "skill",
    "services" => "service",
    "backends" => "backend",
    _ => unreachable!(),
  };
  match command {
    LibraryCommand::Install(args) => return install_package(paths, args, Some(expected_kind)),
    LibraryCommand::Remove { id, version } => {
      let path = packages::package_path(paths, &id, &version)?;
      let release = packages::read_manifest(&path.join("release.json"))?;
      ensure!(
        release.kind == expected_kind,
        "{} is a {} package, expected {expected_kind}",
        id,
        release.kind
      );
      packages::remove(paths, &id, &version)?;
      println!("removed {id} {version}");
      return Ok(());
    }
    _ => {}
  }
  let nodes = configured_nodes(paths, name, expected_kind)?;
  match command {
    LibraryCommand::List { json } => {
      if json {
        println!("{}", serde_json::to_string_pretty(&nodes)?);
      } else if nodes.is_empty() {
        println!("no {kind} in deployment {name}");
      } else {
        for node in nodes {
          println!(
            "{:<20} {}",
            node["id"].as_str().unwrap_or(""),
            if node["enabled"] == true {
              "enabled"
            } else {
              "disabled"
            }
          );
        }
      }
    }
    LibraryCommand::Info { node } => {
      let selected = nodes
        .iter()
        .find(|entry| entry["id"] == node)
        .with_context(|| format!("{node} is not a {expected_kind} node in deployment {name}"))?;
      println!("{}", serde_json::to_string_pretty(selected)?);
    }
    LibraryCommand::Status { node, json } => {
      let selected = nodes
        .iter()
        .find(|entry| entry["id"] == node)
        .with_context(|| format!("{node} is not a {expected_kind} node in deployment {name}"))?;
      let deployment = if UnixStream::connect(paths.socket()).is_ok() {
        crate::runtime::manager::manager_call(paths, name, "deployment_status")?
      } else {
        json!({"state":"stopped"})
      };
      let status = json!({"node":selected,"deployment_state":deployment["state"]});
      if json {
        println!("{}", serde_json::to_string_pretty(&status)?);
      } else {
        println!(
          "{node}: {} in {} deployment",
          if selected["enabled"] == true {
            "enabled"
          } else {
            "disabled"
          },
          deployment["state"].as_str().unwrap_or("unknown")
        );
      }
    }
    LibraryCommand::Install(_) | LibraryCommand::Remove { .. } => unreachable!(),
  }
  Ok(())
}

pub(crate) fn services_command(
  paths: &config::Paths,
  name: &str,
  command: ServiceCommand,
) -> Result<()> {
  let result = match command {
    ServiceCommand::List => {
      json!({"services":services::load_specs(paths)?.into_keys().collect::<Vec<_>>()})
    }
    ServiceCommand::Validate => json!({"services":services::validate_all(paths)?}),
    ServiceCommand::Start { id } => service_call(paths, name, "services_start", Some(id), None)?,
    ServiceCommand::Stop { id } => {
      if UnixStream::connect(paths.socket()).is_err() {
        json!({"stopped":[]})
      } else {
        service_call(paths, name, "services_stop", Some(id), None)?
      }
    }
    ServiceCommand::Restart { id } => {
      service_call(paths, name, "services_restart", Some(id), None)?
    }
    ServiceCommand::Status { id } => {
      let status = if UnixStream::connect(paths.socket()).is_ok() {
        service_call(paths, name, "services_status", None, None)?
      } else {
        json!({"services":[]})
      };
      if let Some(id) = id {
        let entries = status["services"]
          .as_array()
          .context("invalid services status")?;
        json!({"services":entries.iter().filter(|entry| entry["id"] == id).collect::<Vec<_>>()})
      } else {
        status
      }
    }
    ServiceCommand::Logs { id, lines } => {
      service_call(paths, name, "services_logs", Some(id), Some(lines))?
    }
    ServiceCommand::Adapter {
      provider,
      repo,
      alias,
    } => {
      let command = services::island_models_adapter(&repo, &alias)?;
      json!({"provider":provider,"alias":alias,"managed_command":command})
    }
    ServiceCommand::Install(args) => {
      return library(paths, name, "services", LibraryCommand::Install(args));
    }
    ServiceCommand::Remove { id, version } => {
      return library(
        paths,
        name,
        "services",
        LibraryCommand::Remove { id, version },
      );
    }
  };
  println!("{}", serde_json::to_string_pretty(&result)?);
  Ok(())
}

pub(crate) fn print_init_next_steps(mode: &str) {
  println!("\nNext steps:");
  println!("  Build EOS, EI, and IsosUI in this workspace ({mode} mode).");
  println!("  Configure ISOS_AETHER_BIN_DIR, ISOS_EI_BIN_DIR, and ISOSUI_DIST_DIR.");
  println!("  isos doctor                       # inspect build and launch readiness");
  println!("  isos                               # open the workbench; cores stay stopped");
  println!("  In Graph, add a backend, then Check graph and Save graph before `isos run`.");
  println!("  Run `isos --help` for mode rules or `isos doctor` to check readiness.");
}

#[cfg(test)]
mod package_update_cli_tests {
  use super::*;

  #[test]
  fn package_upgrade_rejects_unscoped_script_approval() {
    let home = tempfile::tempdir().unwrap();
    let paths = config::Paths {
      config: home.path().join("config"),
      data: home.path().join("data"),
      runtime: home.path().join("run"),
    };
    let error = package_command(
      &paths,
      "default",
      PackageCommand::Upgrade {
        id: None,
        approve_download: Some("digest".to_owned()),
      },
    )
    .unwrap_err();
    assert!(error.to_string().contains("requires one package ID"));
  }
}
