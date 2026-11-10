//! Local emergency shutdown when the manager or run identity is unavailable.

use crate::{app::config::Paths, runtime::manager::owner_deployment};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path, process::Command};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StopAuthConfig {
  code_sha256: String,
}

fn configured_hash(paths: &Paths) -> Result<Option<String>> {
  if let Ok(value) = std::env::var("ISOS_STOP_CODE_SHA256") {
    return Ok(Some(value));
  }
  let path = paths.config.join("isos/emergency-stop.toml");
  match fs::read_to_string(&path) {
    Ok(source) => Ok(Some(toml::from_str::<StopAuthConfig>(&source)?.code_sha256)),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(error) => Err(error.into()),
  }
}

fn authorize(paths: &Paths, code_stdin: bool) -> Result<()> {
  let Some(expected) = configured_hash(paths)? else {
    ensure!(!code_stdin, "no emergency stop code is configured");
    return Ok(());
  };
  ensure!(
    expected.len() == 64 && expected.bytes().all(|byte| byte.is_ascii_hexdigit()),
    "ISOS_STOP_CODE_SHA256 must be a SHA-256 hex digest"
  );
  ensure!(code_stdin, "emergency stop code required; use --code-stdin");
  let mut code = Vec::new();
  std::io::stdin().take(1025).read_to_end(&mut code)?;
  ensure!(code.len() <= 1024, "emergency stop code is too long");
  while code
    .last()
    .is_some_and(|byte| matches!(byte, b'\n' | b'\r'))
  {
    code.pop();
  }
  let actual = hex::encode(Sha256::digest(&code));
  let equal = actual
    .bytes()
    .zip(expected.to_ascii_lowercase().bytes())
    .fold(0u8, |difference, (left, right)| difference | (left ^ right));
  ensure!(equal == 0, "emergency stop code rejected");
  Ok(())
}

pub(crate) fn run(deployment: &str, code_stdin: bool) -> Result<()> {
  let paths = Paths::discover()?;
  authorize(&paths, code_stdin)?;
  let root = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(Path::parent)
    .context("source checkout root is unavailable")?;
  let executable = std::env::current_exe()?;
  ensure!(
    executable.starts_with(root.join("target")),
    "this emergency stop is supported by the source checkout build only"
  );
  let script = root.join("kill.sh");
  ensure!(script.is_file(), "emergency stop script is unavailable");
  eprintln!(
    "WARNING: emergency Stop signals this checkout's processes without run-ID proof; physical safety is unproven."
  );
  let status = Command::new(&script)
    .current_dir(root)
    .env("ISOS_STOP_RUN_DIR", &paths.runtime)
    .status()
    .with_context(|| format!("run {}", script.display()))?;
  if !status.success() {
    bail!("emergency Stop did not clear all matching processes");
  }
  owner_deployment::publish_emergency_recovery(&paths, deployment)?;
  println!("Emergency Stop complete. Verify physical safety before launching again.");
  Ok(())
}
