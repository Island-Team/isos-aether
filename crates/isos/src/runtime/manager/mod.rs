use crate::app::{config, foreground::INTERRUPTED, server as http};
use crate::domain::{
  graph::{graph_contract, graph_apply, starter},
  services,
};
use crate::runtime::{logs as log_store, owners::core_setup, process};
use crate::interfaces::manager::*;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
  fs,
  io::{BufRead, BufReader, Read, Write},
  os::unix::{
    fs::PermissionsExt,
    net::{UnixListener, UnixStream},
  },
  sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
  },
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) fn monotonic_millis() -> u64 {
  let mut clock = libc::timespec {
    tv_sec: 0,
    tv_nsec: 0,
  };
  let ok = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut clock) == 0 };
  if !ok {
    return 0;
  }
  (clock.tv_sec as u64)
    .saturating_mul(1000)
    .saturating_add((clock.tv_nsec as u64) / 1_000_000)
}
pub(crate) fn emit_phase_timing(op: &str, phase: &str, phase_ms: u64, elapsed_ms: u64) {
  if std::env::var_os("ISOS_PHASE_TIMING").is_some() {
    eprintln!(
      "q03_phase_timing operation={op} phase={phase} phase_ms={phase_ms} elapsed_ms={elapsed_ms} monotonic_ms={}",
      monotonic_millis()
    );
  }
}
pub(crate) fn manager_session_id() -> &'static str {
  static SESSION: OnceLock<String> = OnceLock::new();
  SESSION.get_or_init(|| {
    format!(
      "m{}-{:x}",
      std::process::id(),
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
    )
  })
}

pub(crate) fn next_correlation_id() -> String {
  static NEXT_CORRELATION: AtomicU64 = AtomicU64::new(1);
  format!("p{}", NEXT_CORRELATION.fetch_add(1, Ordering::Relaxed))
}

pub(crate) fn next_run_id() -> String {
  static NEXT_RUN: AtomicU64 = AtomicU64::new(1);
  format!(
    "r{}-{}",
    manager_session_id().trim_start_matches('m'),
    NEXT_RUN.fetch_add(1, Ordering::Relaxed)
  )
}

fn validate_saved_graph(paths: &config::Paths, name: &str, source: &str) -> Result<()> {
  let saved = graph_contract::parse(source.as_bytes())?;
  ensure!(
    saved.value["version"] == "0.2.0",
    "graph version must be 0.2.0"
  );
  let proposal = graph_apply::Proposal {
    graph: saved.value,
    bytes: source.as_bytes().to_vec(),
    revision: saved.revision,
  };
  graph_apply::prepare(paths, name, &proposal).map_err(anyhow::Error::new)?;
  Ok(())
}

pub(crate) fn manager_identity(paths: &config::Paths) -> Result<Value> {
  let executable = std::env::current_exe()?;
  let stat = fs::read_to_string("/proc/self/stat")?;
  let fields = stat
    .rsplit_once(") ")
    .context("read process start identity")?
    .1;
  let start_ticks = fields
    .split_whitespace()
    .nth(19)
    .context("process start identity is incomplete")?;
  Ok(json!({
      "pid": std::process::id(),
      "start_ticks": start_ticks,
      "mode": paths.mode()?,
      "executable": executable,
      // The source build can replace its path while this manager is still running.
      "executable_sha256": hex::encode(Sha256::digest(fs::read("/proc/self/exe")?)),
      "isos_version": env!("CARGO_PKG_VERSION"),
      "api_schema_version": "0.1.0",
      "service_contract_version": "1.0.0",
      "api_origin": http::local_origin()?,
      "ui_url": http::ui_url()?,
  }))
}

mod operations;
pub(crate) mod owner_deployment;
mod socket;
mod daemon_runtime;
mod client;

pub(crate) use operations::api_call;
pub(crate) use socket::{bind_daemon, handle_client};
pub(crate) use daemon_runtime::daemon;
pub(crate) use client::{
  RemoteDiagnosticError, manager_call, manager_call_revision, manager_status_for_http, service_call,
};
