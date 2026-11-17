//! Checked release publication. Every retry inspects remote state before mutation.

use super::*;

pub(super) fn stage_file(source: &Path, destination: &Path) -> Result<PathBuf> {
  ensure!(source.is_absolute(), "checked input path must be absolute");
  let metadata = fs::symlink_metadata(source)?;
  ensure!(
    metadata.file_type().is_file(),
    "checked input must be a regular file"
  );
  let bytes = fs::read(source)?;
  config::atomic_write(destination, &bytes)?;
  fs::set_permissions(destination, fs::Permissions::from_mode(0o400))?;
  Ok(destination.to_path_buf())
}

pub(super) fn verify_source_payload(
  source: &Path,
  release: &Release,
  archive: &Path,
) -> Result<()> {
  let output = tempfile::tempdir()?;
  let script = checked_file(source, "build.sh", true)?;
  let built = Command::new(script)
    .args([
      "--output",
      output
        .path()
        .to_str()
        .context("temporary path is not UTF-8")?,
      "--version",
      &release.version,
    ])
    .current_dir(source)
    .output()?;
  ensure!(
    built.status.success(),
    "independent source rebuild failed: {}",
    String::from_utf8_lossy(&built.stderr)
  );
  let (head, clean) = git_identity(source)?;
  ensure!(
    clean && head == release.source_commit,
    "source changed during independent rebuild"
  );
  let rebuilt_manifest = checked_file(output.path(), "release.json", false)?;
  let mut rebuilt = read_manifest(&rebuilt_manifest)?;
  let rebuilt_archive = checked_file(output.path(), &selected_asset_name(&rebuilt)?, false)?;
  release::verify_file(
    rebuilt.select_asset(&release::Platform::current())?,
    &rebuilt_archive,
  )?;
  let candidate = release.select_asset(&release::Platform::current())?;
  let rebuilt_asset = rebuilt
    .assets
    .iter_mut()
    .find(|asset| asset.os == candidate.os && asset.arch == candidate.arch)
    .context("rebuilt platform asset missing")?;
  rebuilt_asset.sha256 = candidate.sha256.clone();
  rebuilt_asset.size_bytes = candidate.size_bytes;
  ensure!(
    canonical_json(&rebuilt)? == canonical_json(release)?,
    "rebuild changed package manifest semantics"
  );
  let first = tempfile::tempdir()?;
  let second = tempfile::tempdir()?;
  unpack_archive(archive, first.path())?;
  unpack_archive(&rebuilt_archive, second.path())?;
  ensure!(
    payload_digest(first.path(), false)? == payload_digest(second.path(), false)?,
    "checked archive payload differs from independent clean-source rebuild"
  );
  Ok(())
}

fn gh_output(gh: &str, args: &[&str]) -> Result<std::process::Output> {
  Command::new(gh)
    .args(args)
    .output()
    .with_context(|| format!("run gh {}", args.join(" ")))
}

fn release_view(gh: &str, repo: &str, tag: &str) -> Result<Option<serde_json::Value>> {
  let output = gh_output(
    gh,
    &[
      "release",
      "view",
      tag,
      "--repo",
      repo,
      "--json",
      "isDraft,assets",
    ],
  )?;
  if output.status.success() {
    return Ok(Some(serde_json::from_slice(&output.stdout)?));
  }
  let error = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
  ensure!(
    error.contains("release not found") || error.contains("http 404"),
    "cannot inspect GitHub Release: {error}"
  );
  Ok(None)
}

fn remote_asset_matches(gh: &str, repo: &str, tag: &str, name: &str, local: &Path) -> Result<bool> {
  let directory = tempfile::tempdir()?;
  let output = gh_output(
    gh,
    &[
      "release",
      "download",
      tag,
      "--repo",
      repo,
      "--pattern",
      name,
      "--dir",
      directory
        .path()
        .to_str()
        .context("temporary path is not UTF-8")?,
    ],
  )?;
  ensure!(
    output.status.success(),
    "fresh GitHub download of {name} failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );
  let downloaded = checked_file(directory.path(), name, false)?;
  Ok(digest(&fs::read(downloaded)?) == digest(&fs::read(local)?))
}

fn ensure_assets(
  gh: &str,
  repo: &str,
  tag: &str,
  view: &serde_json::Value,
  files: &[&Path],
) -> Result<()> {
  let assets = view["assets"]
    .as_array()
    .context("GitHub release assets missing")?;
  for file in files {
    let name = file
      .file_name()
      .and_then(|name| name.to_str())
      .context("asset filename is not UTF-8")?;
    ensure!(
      name != "check.json",
      "local check report is not a publication asset"
    );
    if assets.iter().any(|asset| asset["name"] == name) {
      ensure!(
        remote_asset_matches(gh, repo, tag, name, file)?,
        "remote GitHub asset {name} differs from qualified bytes; refusing overwrite"
      );
      continue;
    }
    ensure!(
      view["isDraft"] == true,
      "published GitHub Release is missing {name}"
    );
    let output = Command::new(gh)
      .args(["release", "upload", tag, "--repo", repo])
      .arg(file)
      .output()?;
    ensure!(
      output.status.success(),
      "upload of {name} failed: {}",
      String::from_utf8_lossy(&output.stderr)
    );
  }
  Ok(())
}

/// A published release may be resumed after local tools have been rebuilt,
/// but only when its immutable assets still prove the original qualification.
pub(super) fn verify_published_assets(gh: &str, release: &Release, files: &[&Path]) -> Result<()> {
  let repo = checked_release_repository(release)?;
  let tag = format!("v{}", release.version);
  let view = release_view(gh, &repo, &tag)?.context("GitHub Release is missing")?;
  ensure!(view["isDraft"] == false, "GitHub Release is not published");
  let assets = view["assets"]
    .as_array()
    .context("GitHub release assets missing")?;
  for file in files {
    let name = file
      .file_name()
      .and_then(|name| name.to_str())
      .context("asset filename is not UTF-8")?;
    ensure!(
      assets.iter().any(|asset| asset["name"] == name),
      "published GitHub Release is missing {name}"
    );
    ensure!(
      remote_asset_matches(gh, &repo, &tag, name, file)?,
      "published GitHub asset {name} differs from checked bytes"
    );
  }
  Ok(())
}

pub(super) struct CheckedFiles<'a> {
  pub manifest: &'a Path,
  pub archive: &'a Path,
  pub qualification: &'a Path,
  pub graphs: &'a [PathBuf],
  pub package_config_file: Option<&'a Path>,
}

pub(super) fn publish_checked(
  paths: &Paths,
  release: &Release,
  files: CheckedFiles<'_>,
  gh: &str,
  already_cataloged: bool,
) -> Result<()> {
  let CheckedFiles {
    manifest,
    archive,
    qualification,
    graphs,
    package_config_file,
  } = files;
  if !already_cataloged {
    let credentials = auth::read(paths)?;
    ensure!(
      credentials.user.role == "developer",
      "package publication requires an Island-port developer account"
    );
  }
  let repo = checked_release_repository(release)?;
  let tag = format!("v{}", release.version);
  checked_git_tag(release, &repo, gh)?;
  let mut view = release_view(gh, &repo, &tag)?;
  if view.is_none() {
    let output = gh_output(
      gh,
      &[
        "release",
        "create",
        &tag,
        "--repo",
        &repo,
        "--draft",
        "--verify-tag",
        "--title",
        &format!("{} {}", release.id, release.version),
      ],
    )?;
    ensure!(
      output.status.success(),
      "create draft GitHub Release failed: {}",
      String::from_utf8_lossy(&output.stderr)
    );
    view = release_view(gh, &repo, &tag)?;
  }
  let view = view.context("created GitHub draft is not visible")?;
  let files = [manifest, archive, qualification];
  ensure_assets(gh, &repo, &tag, &view, &files)?;
  // Re-read the release after uploading. A partial upload can be retried safely.
  let view = release_view(gh, &repo, &tag)?.context("GitHub Release disappeared")?;
  for file in files {
    let name = file
      .file_name()
      .and_then(|name| name.to_str())
      .context("asset filename is not UTF-8")?;
    ensure!(
      view["assets"]
        .as_array()
        .is_some_and(|assets| assets.iter().any(|asset| asset["name"] == name)),
      "GitHub Release is missing {name}"
    );
    ensure!(
      remote_asset_matches(gh, &repo, &tag, name, file)?,
      "fresh GitHub asset {name} differs from qualified bytes"
    );
  }
  let replay_dir = tempfile::tempdir()?;
  let fresh_dir = tempfile::tempdir()?;
  let archive_name = archive
    .file_name()
    .and_then(|name| name.to_str())
    .context("archive filename is not UTF-8")?;
  let output = gh_output(
    gh,
    &[
      "release",
      "download",
      &tag,
      "--repo",
      &repo,
      "--pattern",
      archive_name,
      "--dir",
      fresh_dir
        .path()
        .to_str()
        .context("temporary path is not UTF-8")?,
    ],
  )?;
  ensure!(
    output.status.success(),
    "fresh release archive download failed"
  );
  let fetched = checked_file(fresh_dir.path(), archive_name, false)?;
  release::verify_file(
    release.select_asset(&release::Platform::current())?,
    &fetched,
  )?;
  let isolated = Paths {
    config: replay_dir.path().join("config"),
    data: replay_dir.path().join("data"),
    runtime: replay_dir.path().join("run"),
  };
  install_local(&isolated, release, &fetched)?;
  verify(&isolated, &release.id, &release.version)?;
  if let Some(file) = package_config_file {
    super::prepare_candidate_config(&isolated, release, graphs, file)?;
  }
  for (index, graph) in graphs.iter().enumerate() {
    let graph_value = crate::runtime::process::normalize_source(&fs::read_to_string(graph)?)?;
    crate::runtime::process::validate_value(&isolated, &graph_value)?;
    if release.schema_version == "0.2.1" {
      crate::domain::graph::graph_apply::apply_new(
        &isolated,
        &replay::qualification_name(&graph_value)?,
        graph_value.clone(),
        "",
      )
      .map_err(|error| anyhow::anyhow!("downloaded Apply failed: {error}"))?;
    }
    replay::managed_replay(
      &isolated,
      release,
      graph,
      &replay_dir.path().join(format!("graph-{index}")),
    )?;
  }
  if view["isDraft"] == true {
    let output = gh_output(
      gh,
      &["release", "edit", &tag, "--repo", &repo, "--draft=false"],
    )?;
    ensure!(
      output.status.success(),
      "publish GitHub Release failed: {}",
      String::from_utf8_lossy(&output.stderr)
    );
  }
  let view = release_view(gh, &repo, &tag)?.context("published GitHub Release disappeared")?;
  ensure!(
    view["isDraft"] == false,
    "GitHub Release publication was not confirmed"
  );
  if already_cataloged {
    println!(
      "{} {} already has the exact Island-port catalog record",
      release.id, release.version
    );
    return Ok(());
  }
  // The catalog submission is last. A failure leaves a verifiable GitHub release
  // and a retry can continue without replacing immutable assets.
  let body = serde_json::json!({
    "package_id": release.id,
    "version": release.version,
    "source_url": release.source_url,
    "release_url": release.release_url,
    "manifest": release,
  });
  let response = auth::authorized_post(
    paths,
    &format!("{CATALOG}/api/packages/v1/submissions"),
    &body,
  )?;
  let status = response.status();
  let envelope: serde_json::Value = response
    .json()
    .context("Island-port returned invalid publication response")?;
  ensure!(
    status.is_success() && envelope["success"] == true,
    "Island-port publication failed: {}",
    envelope["error"]["message"]
      .as_str()
      .unwrap_or("request failed")
  );
  println!(
    "submitted {} {} for Island-port review (status: {})",
    release.id,
    release.version,
    envelope["data"]["status"].as_str().unwrap_or("unknown")
  );
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::PermissionsExt;

  #[test]
  fn existing_remote_assets_must_match_exact_checked_bytes() {
    let root = tempfile::tempdir().unwrap();
    let local = root.path().join("release.json");
    let remote = root.path().join("remote-release.json");
    fs::write(&local, b"qualified").unwrap();
    fs::write(&remote, b"qualified").unwrap();
    let gh = root.path().join("gh");
    fs::write(&gh, format!("#!/bin/sh\ncase \"$1 $2\" in\n  'release download') while [ \"$1\" != '--dir' ]; do shift; done; cp '{}' \"$2/release.json\";;\n  *) exit 9;;\nesac\n", remote.display())).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o700)).unwrap();
    let view = serde_json::json!({"isDraft":true,"assets":[{"name":"release.json"}]});
    ensure_assets(
      gh.to_str().unwrap(),
      "vendor/package",
      "v0.2.0",
      &view,
      &[&local],
    )
    .unwrap();
    fs::write(&remote, b"different").unwrap();
    assert!(
      ensure_assets(
        gh.to_str().unwrap(),
        "vendor/package",
        "v0.2.0",
        &view,
        &[&local]
      )
      .unwrap_err()
      .to_string()
      .contains("refusing overwrite")
    );
  }

  #[test]
  fn published_release_cannot_gain_missing_asset() {
    let root = tempfile::tempdir().unwrap();
    let local = root.path().join("qualification.json");
    fs::write(&local, b"qualified").unwrap();
    let view = serde_json::json!({"isDraft":false,"assets":[]});
    assert!(
      ensure_assets("unused-gh", "vendor/package", "v0.2.0", &view, &[&local])
        .unwrap_err()
        .to_string()
        .contains("missing")
    );
  }

  #[test]
  fn changed_binary_resume_requires_published_exact_assets() {
    let root = tempfile::tempdir().unwrap();
    for name in ["release.json", "bundle.tar.gz", "qualification.json"] {
      fs::write(root.path().join(name), name.as_bytes()).unwrap();
    }
    let remote = root.path().join("remote");
    fs::create_dir(&remote).unwrap();
    for name in ["release.json", "bundle.tar.gz", "qualification.json"] {
      fs::copy(root.path().join(name), remote.join(name)).unwrap();
    }
    let state = root.path().join("state");
    fs::write(&state, "false").unwrap();
    let gh = root.path().join("gh");
    fs::write(&gh, format!("#!/bin/sh\nif [ \"$1 $2\" = 'release view' ]; then printf '{{\"isDraft\":%s,\"assets\":[{{\"name\":\"release.json\"}},{{\"name\":\"bundle.tar.gz\"}},{{\"name\":\"qualification.json\"}}]}}' \"$(cat '{}')\"; exit 0; fi\nif [ \"$1 $2\" = 'release download' ]; then while [ \"$1\" != '--pattern' ]; do shift; done; name=$2; while [ \"$1\" != '--dir' ]; do shift; done; cp '{}/'$name \"$2/$name\"; exit 0; fi\nexit 9\n", state.display(), remote.display())).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o700)).unwrap();
    let release: Release = serde_json::from_value(serde_json::json!({
      "contract":"isos.package@v1", "schema_version":"0.2.0", "id":"vendor/example",
      "kind":"component", "version":"0.2.0", "source_commit":"a".repeat(40),
      "source_url":"https://github.com/vendor/example",
      "release_url":"https://github.com/vendor/example/releases/tag/v0.2.0",
      "assets":[]
    }))
    .unwrap();
    let files: Vec<PathBuf> = ["release.json", "bundle.tar.gz", "qualification.json"]
      .iter()
      .map(|name| root.path().join(name))
      .collect();
    let paths: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let gh = gh.to_str().unwrap();
    verify_published_assets(gh, &release, &paths).unwrap();
    fs::write(&state, "true").unwrap();
    assert!(verify_published_assets(gh, &release, &paths).is_err());
    fs::write(&state, "false").unwrap();
    fs::write(root.path().join("qualification.json"), b"tampered").unwrap();
    assert!(verify_published_assets(gh, &release, &paths).is_err());
  }

  #[test]
  fn staged_publication_file_is_bound_before_original_mutation() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("release.json");
    let staged = root.path().join("staged.json");
    fs::write(&original, b"checked bytes").unwrap();
    stage_file(&original, &staged).unwrap();
    fs::write(&original, b"changed after validation").unwrap();
    assert_eq!(fs::read(&staged).unwrap(), b"checked bytes");
    assert_ne!(
      digest(&fs::read(&staged).unwrap()),
      digest(&fs::read(&original).unwrap())
    );
  }

  #[test]
  fn edited_check_and_archive_cannot_substitute_clean_source_payload() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("payload.txt"), b"source payload").unwrap();
    let script = source.join("build.sh");
    fs::write(&script, b"#!/bin/sh\nset -eu\n[ \"$1\" = '--output' ]\nout=$2\n[ \"$3\" = '--version' ]\nversion=$4\nmkdir -p \"$out/content\"\ncp payload.txt \"$out/content/payload.txt\"\ntar -czf \"$out/bundle.tar.gz\" -C \"$out/content\" payload.txt\nsha=$(sha256sum \"$out/bundle.tar.gz\" | cut -d' ' -f1)\nsize=$(stat -c %s \"$out/bundle.tar.gz\")\ncommit=$(git rev-parse HEAD)\nprintf '{\"contract\":\"isos.package@v1\",\"schema_version\":\"0.2.0\",\"id\":\"vendor/sample\",\"kind\":\"component\",\"version\":\"%s\",\"source_commit\":\"%s\",\"source_url\":\"https://github.com/vendor/sample\",\"release_url\":\"https://github.com/vendor/sample/releases/tag/v%s\",\"node_contributions\":{},\"assets\":[{\"os\":\"linux\",\"arch\":\"x86_64\",\"url\":\"https://github.com/vendor/sample/releases/download/v%s/bundle.tar.gz\",\"size_bytes\":%s,\"sha256\":\"%s\"}]}' \"$version\" \"$commit\" \"$version\" \"$version\" \"$size\" \"$sha\" > \"$out/release.json\"\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
      Command::new("git")
        .args(["init", "-q"])
        .current_dir(&source)
        .status()
        .unwrap()
        .success()
    );
    assert!(
      Command::new("git")
        .args(["add", "."])
        .current_dir(&source)
        .status()
        .unwrap()
        .success()
    );
    assert!(
      Command::new("git")
        .args([
          "-c",
          "user.name=Test",
          "-c",
          "user.email=test@example.invalid",
          "commit",
          "-qm",
          "fixture"
        ])
        .current_dir(&source)
        .status()
        .unwrap()
        .success()
    );
    let original = root.path().join("original");
    fs::create_dir(&original).unwrap();
    assert!(
      Command::new(&script)
        .args(["--output", original.to_str().unwrap(), "--version", "0.2.0"])
        .current_dir(&source)
        .status()
        .unwrap()
        .success()
    );
    let mut release = read_manifest(&original.join("release.json")).unwrap();
    verify_source_payload(&source, &release, &original.join("bundle.tar.gz")).unwrap();
    let altered = root.path().join("altered");
    fs::create_dir(&altered).unwrap();
    fs::write(altered.join("payload.txt"), b"substituted payload").unwrap();
    let archive = root.path().join("bundle.tar.gz");
    assert!(
      Command::new("tar")
        .args([
          "-czf",
          archive.to_str().unwrap(),
          "-C",
          altered.to_str().unwrap(),
          "payload.txt"
        ])
        .status()
        .unwrap()
        .success()
    );
    release.assets[0].sha256 = digest(&fs::read(&archive).unwrap());
    release.assets[0].size_bytes = fs::metadata(&archive).unwrap().len();
    assert!(
      verify_source_payload(&source, &release, &archive)
        .unwrap_err()
        .to_string()
        .contains("payload differs")
    );
  }
}
