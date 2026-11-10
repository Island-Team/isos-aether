//! Bounded, owner-only per-deployment logs and cursor reads.

pub(crate) mod manager_log;
use crate::app::config::Paths;
use crate::runtime::owners::{dependent_stop, event_policy};
mod cursor;
mod core_files;
use anyhow::{Context, Result, ensure};
use chrono::{SecondsFormat, Utc};
pub use cursor::{CurrentError, CurrentQuery};
use serde_json::{Value, json};
#[cfg(test)]
use std::sync::mpsc::Receiver;
use std::{
  collections::BTreeMap,
  fs::{self, File, OpenOptions},
  io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
  os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
  },
};
#[cfg(test)]
use std::collections::BTreeSet;
#[cfg(test)]
use std::time::{Duration, Instant};

const DEFAULT_MAX_BYTES: u64 = 8_388_608;
const DEFAULT_BACKUPS: u8 = 5;
const MAX_READ_BYTES: u64 = 65_536;
static WRITE_FAILED: AtomicU64 = AtomicU64::new(0);
static DEPLOYMENT_LOSSES: OnceLock<Mutex<BTreeMap<PathBuf, Arc<AtomicU64>>>> = OnceLock::new();

fn saturating_increment(counter: &AtomicU64) {
  let mut current = counter.load(Ordering::Relaxed);
  loop {
    let next = current.saturating_add(1);
    match counter.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
      Ok(_) => return,
      Err(observed) => current = observed,
    }
  }
}

pub struct LogContext<'a> {
  pub run_id: Option<&'a str>,
  pub correlation_id: Option<&'a str>,
  pub node_path: Option<&'a str>,
  pub fields: Value,
}

#[derive(Clone)]
pub struct LogStore {
  root: PathBuf,
  runtime: PathBuf,
  max_bytes: u64,
  backups: u8,
  level: u8,
  lock: Arc<Mutex<()>>,
  #[cfg(test)]
  suppression: Arc<Mutex<BTreeMap<String, (Instant, u32)>>>,
  losses: Arc<AtomicU64>,
}

fn token_valid(value: &str, max: usize) -> bool {
  !value.is_empty()
    && value.len() <= max
    && value
      .bytes()
      .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn source_valid(source: &str) -> bool {
  token_valid(source, 48)
}

fn event_code_valid(code: &str) -> bool {
  !code.is_empty()
    && code.len() <= 64
    && code
      .bytes()
      .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

pub fn valid_source(source: &str) -> bool {
  source_valid(source) && !matches!(source, "all" | "services" | "events")
}

fn shared_lock() -> Arc<Mutex<()>> {
  static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
  LOCK.get_or_init(|| Arc::new(Mutex::new(()))).clone()
}

fn shared_losses(root: &Path) -> Arc<AtomicU64> {
  DEPLOYMENT_LOSSES
    .get_or_init(|| Mutex::new(BTreeMap::new()))
    .lock()
    .unwrap()
    .entry(root.to_path_buf())
    .or_insert_with(|| Arc::new(AtomicU64::new(0)))
    .clone()
}

pub fn level_rank(level: &str) -> Option<u8> {
  match level {
    "trace" => Some(0),
    "debug" => Some(1),
    "info" => Some(2),
    "warn" => Some(3),
    "error" => Some(4),
    _ => None,
  }
}

fn private_dir(path: &Path) -> Result<()> {
  fs::create_dir_all(path)?;
  fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
  Ok(())
}

fn ordered_event(value: &Value) -> Result<Vec<u8>> {
  let mut event = value.clone();
  let source = value["source"].as_str().unwrap_or("manager");
  if !source.contains('@') {
    event["source"] = json!(format!("isos.runtime.{source}.event@aether-v0.2.1"));
  }
  event["core"] = json!("isos");
  Ok(serde_json::to_vec(&event)?)
}

pub fn jsonl_line(value: &Value) -> Result<String> {
  Ok(String::from_utf8(ordered_event(value)?)?)
}

impl LogStore {
  pub fn new(paths: &Paths, deployment: &str) -> Result<Self> {
    Self::try_new(paths, deployment).inspect_err(|_| saturating_increment(&WRITE_FAILED))
  }

  fn try_new(paths: &Paths, deployment: &str) -> Result<Self> {
    ensure!(source_valid(deployment), "invalid deployment for logs");
    let max_bytes = std::env::var("ISOS_LOG_MAX_BYTES")
      .ok()
      .map(|v| v.parse())
      .transpose()?
      .unwrap_or(DEFAULT_MAX_BYTES);
    let backups = std::env::var("ISOS_LOG_BACKUPS")
      .ok()
      .map(|v| v.parse())
      .transpose()?
      .unwrap_or(DEFAULT_BACKUPS);
    let level = std::env::var("ISOS_LOG_LEVEL").unwrap_or_else(|_| "debug".into());
    let level =
      level_rank(&level).context("ISOS_LOG_LEVEL must be trace, debug, info, warn, or error")?;
    ensure!(
      (65_536..=100_000_000).contains(&max_bytes),
      "ISOS_LOG_MAX_BYTES must be 65536..100000000"
    );
    ensure!(
      (1..=10).contains(&backups),
      "ISOS_LOG_BACKUPS must be 1..10"
    );
    let root = paths.data.join("log/deployments").join(deployment);
    let fresh_root = !root.exists() && !paths.data.join("log").join(deployment).exists();
    private_dir(&root)?;
    // Old log trees are read-only history. New sessions write one JSONL file per core.
    let old_ui = root.join("ui-server.log");
    if old_ui.is_file() {
      let bytes = fs::read(&old_ui)?;
      if !bytes.is_empty() && !bytes.starts_with(b"20") {
        private_dir(&root.join("capture"))?;
        let target = root.join("capture/ui-server.legacy.txt");
        ensure!(!target.exists(), "legacy UI server log migration conflict");
        fs::rename(old_ui, target)?;
      }
    }
    let file = OpenOptions::new()
      .create(true)
      .append(true)
      .mode(0o600)
      .open(root.join("isos.jsonl"))?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    let store = Self {
      root,
      runtime: paths.runtime.clone(),
      max_bytes,
      backups,
      level,
      lock: shared_lock(),
      #[cfg(test)]
      suppression: Arc::new(Mutex::new(BTreeMap::new())),
      losses: shared_losses(&paths.data.join("log/deployments").join(deployment)),
    };
    if let Err(error) = cursor::initialize(&store, fresh_root) {
      saturating_increment(&WRITE_FAILED);
      saturating_increment(&store.losses);
      eprintln!("isos cursor initialization failed: {error:#}");
    }
    Ok(store)
  }

  pub fn root(&self) -> &Path {
    &self.root
  }

  fn current_run_id(&self) -> String {
    let name = self.root.file_name().unwrap_or_default().to_string_lossy();
    let path = self.runtime.join(format!("{name}.owner-run.json"));
    fs::read(path)
      .ok()
      .filter(|bytes| bytes.len() <= 32_768)
      .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
      .and_then(|value| {
        value["run_id"]
          .as_str()
          .filter(|id| token_valid(id, 64))
          .map(str::to_owned)
      })
      .unwrap_or_else(|| crate::runtime::manager::manager_session_id().to_owned())
  }

  #[cfg(test)]
  pub fn publish_run(&self, run_id: &str) -> Result<()> {
    cursor::publish_run(self, run_id)
  }

  pub fn run_identity(&self) -> (Option<String>, &'static str) {
    cursor::status(self)
  }

  pub fn manager_scope_id(&self) -> Option<String> {
    cursor::manager_scope_id(self)
  }

  pub fn begin_manager_session(&self) -> Result<()> {
    cursor::begin_manager_session(self)
  }

  /// A new dev manager starts with fresh diagnostics only when no prior run
  /// needs its event history for Stop or cursor recovery.
  pub fn replace_dev_history(&self, paths: &Paths, deployment: &str) -> Result<bool> {
    match self.run_identity() {
      (None, "none") => {}
      (Some(run_id), "known")
        if dependent_stop::recover(paths, deployment, &run_id).outcome == "confirmed" =>
      {
        cursor::reset_confirmed_dev_history(self)?;
      }
      _ => return Ok(false),
    }
    let _guard = self.lock.lock().unwrap();
    for directory in [
      self.root.clone(),
      self.root.join("events"),
      self.root.join("capture"),
      self.root.join("supervisor"),
    ] {
      if !directory.is_dir() {
        continue;
      }
      for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
          continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let log = if directory == self.root {
          name == "isos.jsonl"
            || name.starts_with("isos.jsonl.")
            || name == "events.jsonl"
            || name.starts_with("events.jsonl.")
            || name == "isos.log"
            || name.starts_with("isos.log.")
            || name == "ui-server.log"
            || name.starts_with("ui-server.log.")
            || name.starts_with("legacy-")
        } else {
          true
        };
        if log {
          fs::remove_file(entry.path())?;
        }
      }
    }
    OpenOptions::new()
      .create_new(true)
      .write(true)
      .mode(0o600)
      .open(self.root.join("isos.jsonl"))?;
    for owner in ["eos", "ei"] {
      if let Some(data_root) = paths.data.parent() {
        remove_dev_owner_logs(&data_root.join(owner).join("log").join(deployment))?;
      }
    }
    for owner in ["eos-owner.log", "ei-owner.log"] {
      let path = paths.data.join("log").join(owner);
      if path.is_file() {
        fs::remove_file(path)?;
      }
    }
    let launch = paths.data.join("log/launch/dev/launcher.jsonl");
    if launch.exists() {
      ensure!(launch.is_file(), "dev launcher log is not a regular file");
      fs::remove_file(&launch)?;
    }
    private_dir(launch.parent().context("dev launcher log has no parent")?)?;
    let mut file = OpenOptions::new()
      .create_new(true)
      .write(true)
      .mode(0o600)
      .open(launch)?;
    writeln!(
      file,
      "{}",
      json!({"timestamp":Utc::now().to_rfc3339_opts(SecondsFormat::Millis,true),"level":"info","source":"isos.launcher.isos.sh@aether-v0.2.1","message":"Dev log history replaced"})
    )?;
    Ok(true)
  }

  #[cfg(test)]
  pub fn read_current(&self, query: CurrentQuery<'_>) -> std::result::Result<Value, CurrentError> {
    cursor::read_current(self, query)
  }

  pub fn read_local_current(
    &self,
    paths: &Paths,
    run_id: Option<&str>,
    query: CurrentQuery<'_>,
  ) -> std::result::Result<Value, CurrentError> {
    core_files::read_current(self, paths, run_id, query)
  }

  fn path(&self, source: &str) -> Result<PathBuf> {
    ensure!(source_valid(source), "invalid log source");
    Ok(self.root.join("isos.jsonl"))
  }

  fn append_file(&self, path: &Path, line: &[u8]) -> Result<()> {
    let old_len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if old_len.saturating_add(line.len() as u64) > self.max_bytes {
      for index in (1..=self.backups).rev() {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("log");
        let dest = path.with_extension(format!("{ext}.{index}"));
        let src = if index == 1 {
          path.to_path_buf()
        } else {
          path.with_extension(format!("{ext}.{}", index - 1))
        };
        if dest.exists() {
          fs::remove_file(&dest)?;
        }
        if src.exists() {
          fs::rename(src, dest)?;
        }
      }
    }
    let mut file = OpenOptions::new()
      .create(true)
      .append(true)
      .mode(0o600)
      .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(line)?;
    Ok(())
  }

  pub fn append(&self, source: &str, level: &str, message: &str) -> Result<()> {
    self.append_event(source, level, "message", message)
  }

  /// Detailed owner-only diagnostics are kept outside the public event cursor.
  pub fn append_diagnostic(&self, level: &str, event_code: &str, message: &str) -> Result<()> {
    ensure!(event_code_valid(event_code), "invalid log event code");
    let rank = level_rank(level).context("invalid log level")?;
    if rank < self.level {
      return Ok(());
    }
    let message = if message.len() > 4096 {
      &message[..message.floor_char_boundary(4096)]
    } else {
      message
    };
    let line = serde_json::to_vec(
      &json!({"timestamp":Utc::now().to_rfc3339_opts(SecondsFormat::Millis,true),"level":level,"source":"isos.runtime.manager.diagnostic@aether-v0.2.1","message":message,"event_code":event_code,"run_id":self.current_run_id()}),
    )?;
    let _guard = self.lock.lock().unwrap();
    let result = self.append_file(&self.root.join("isos.jsonl"), &[line, vec![b'\n']].concat());
    if result.is_err() {
      saturating_increment(&WRITE_FAILED);
      saturating_increment(&self.losses);
    }
    result
  }

  #[cfg(test)]
  pub fn append_supervisor(
    &self,
    role: &str,
    level: &str,
    code: &str,
    message: &str,
  ) -> Result<()> {
    ensure!(
      source_valid(role) && event_code_valid(code),
      "invalid supervisor log identity"
    );
    self.append_process(role, level, code, message)
  }

  pub fn append_process(
    &self,
    process: &str,
    level: &str,
    code: &str,
    message: &str,
  ) -> Result<()> {
    ensure!(
      source_valid(process) && event_code_valid(code),
      "invalid process log identity"
    );
    let message = &message[..message.floor_char_boundary(4096)];
    let line = serde_json::to_vec(
      &json!({"timestamp":Utc::now().to_rfc3339_opts(SecondsFormat::Millis,true),"level":level,"source":format!("isos.runtime.{process}.event@aether-v0.2.1"),"event_code":code,"message":message,"run_id":self.current_run_id()}),
    )?;
    let _guard = self.lock.lock().unwrap();
    let result = self.append_file(&self.root.join("isos.jsonl"), &[line, vec![b'\n']].concat());
    if result.is_err() {
      saturating_increment(&WRITE_FAILED);
      saturating_increment(&self.losses);
    }
    result
  }

  pub fn append_event(
    &self,
    source: &str,
    level: &str,
    event_code: &str,
    message: &str,
  ) -> Result<()> {
    self.append_event_context(
      source,
      level,
      event_code,
      message,
      LogContext {
        run_id: None,
        correlation_id: None,
        node_path: None,
        fields: json!({}),
      },
    )
  }

  pub fn append_event_context(
    &self,
    source: &str,
    level: &str,
    event_code: &str,
    message: &str,
    context: LogContext<'_>,
  ) -> Result<()> {
    self.append_event_context_marked(source, level, event_code, message, context, false)
  }

  pub fn append_apply_failure_context(
    &self,
    source: &str,
    level: &str,
    event_code: &str,
    message: &str,
    context: LogContext<'_>,
  ) -> Result<()> {
    self.append_event_context_marked(source, level, event_code, message, context, true)
  }

  fn append_event_context_marked(
    &self,
    source: &str,
    level: &str,
    event_code: &str,
    message: &str,
    context: LogContext<'_>,
    apply_failure: bool,
  ) -> Result<()> {
    let LogContext {
      run_id,
      correlation_id,
      node_path,
      fields,
    } = context;
    ensure!(valid_source(source), "invalid log source");
    ensure!(event_code_valid(event_code), "invalid log event code");
    for id in [run_id, correlation_id].into_iter().flatten() {
      ensure!(token_valid(id, 64), "invalid log correlation ID");
    }
    ensure!(
      node_path.is_none_or(|path| event_policy::node_pointer(path, "isos")),
      "invalid log node path"
    );
    let object = fields.as_object().context("log fields must be an object")?;
    ensure!(object.len() <= 12, "too many log fields");
    ensure!(
      serde_json::to_vec(&fields)?.len() <= 4096,
      "log fields too large"
    );
    for (key, value) in object {
      ensure!(
        source_valid(key) && key.len() <= 48,
        "invalid log field name"
      );
      ensure!(
        value.is_null()
          || value.is_boolean()
          || value.is_number()
          || value
            .as_str()
            .is_some_and(|s| s.len() <= 256 && !s.chars().any(char::is_control)),
        "invalid log field value"
      );
    }
    let rank = level_rank(level).context("invalid log level")?;
    if rank < self.level
      && !matches!(
        event_code,
        "deployment_starting"
          | "deployment_stopping"
          | "deployment_ready"
          | "deployment_failed"
          | "deployment_stopped"
          | "child_restart_attempt"
          | "child_restart_ready"
          | "package_load_failed"
          | "package_verified"
          | "events_suppressed"
      )
    {
      return Ok(());
    }
    let _guard = self.lock.lock().unwrap();
    let path = self.path(source)?;
    let message = if message.len() > 4096 {
      &message[..message.floor_char_boundary(4096)]
    } else {
      message
    };
    let mut safe_message = message.to_owned();
    let value = loop {
      let candidate = json!({"timestamp":Utc::now().to_rfc3339_opts(SecondsFormat::Millis,true),"level":level,"source":source,
                "core":if source == "eos" || source == "bridge" || source == "config" { "eos" } else if source == "ei" { "ei" } else { "isos" },
                "node_path":node_path,"event_code":event_code,"message":safe_message,"pid":std::process::id(),
                "run_id":run_id.map(str::to_owned).unwrap_or_else(|| self.current_run_id()),"correlation_id":correlation_id,"fields":fields});
      if ordered_event(&candidate)?.len() < event_policy::MAX_STORED_LINE {
        break candidate;
      }
      ensure!(
        !safe_message.is_empty(),
        "log event exceeds stored line limit"
      );
      safe_message.pop();
    };
    self.append_record(&path, source, &value, apply_failure)
  }

  fn append_record(
    &self,
    path: &Path,
    _source: &str,
    value: &Value,
    _apply_failure: bool,
  ) -> Result<()> {
    let mut line = ordered_event(value)?;
    line.push(b'\n');
    ensure!(
      line.len() <= event_policy::MAX_STORED_LINE,
      "log event exceeds stored line limit"
    );
    let result = (|| {
      self.append_file(path, &line)?;
      Ok(())
    })();
    if result.is_err() {
      saturating_increment(&WRITE_FAILED);
      saturating_increment(&self.losses);
    }
    result
  }

  #[cfg(test)]
  pub fn append_selected(&self, value: &Value) -> Result<()> {
    let source = value["source"].as_str().context("selected event source")?;
    ensure!(
      matches!(source, "eos" | "bridge" | "config" | "ei"),
      "invalid selected source"
    );
    level_rank(value["level"].as_str().context("selected event level")?)
      .context("selected event level")?;
    let code = value["event_code"]
      .as_str()
      .context("selected event code")?;
    let run_id = value["run_id"].as_str().context("selected event run")?;
    let eligible = matches!(code, "ei_discovery_lost" | "ei_safety_rejected");
    let terminal = code.ends_with("_stopped")
      || code.ends_with("_start_failed")
      || code.ends_with("_terminal")
      || code == "ei_operation_unknown";
    // Serialize admission and the append. A failed first write must not
    // become the representative event for the next sixty seconds.
    let _guard = self.lock.lock().unwrap();
    let mut summaries = Vec::new();
    let mut suppress = false;
    let mut pending_key = None;
    {
      let mut table = self.suppression.lock().unwrap();
      let now = Instant::now();
      if eligible {
        let key = format!(
          "{run_id}|{source}|{}|{code}|{}|{}",
          value["node_path"].as_str().unwrap_or(""),
          value["correlation_id"].as_str().unwrap_or(""),
          value["fields"]["reason_code"].as_str().unwrap_or("")
        );
        if let Some((at, count)) = table.get_mut(&key) {
          if now.duration_since(*at) < Duration::from_secs(60) {
            *count = count.saturating_add(1);
            suppress = true;
          } else {
            if *count > 0 {
              summaries.push((source.to_owned(), code.to_owned(), *count));
            }
            pending_key = Some((key, now));
          }
        } else if table.len() < 256 {
          pending_key = Some((key, now));
        }
      }
      if terminal {
        for (key, (_, count)) in table.iter_mut() {
          if key.starts_with(&format!("{run_id}|")) && *count > 0 {
            let parts: Vec<_> = key.split('|').collect();
            summaries.push((parts[1].to_owned(), parts[3].to_owned(), *count));
            *count = 0;
          }
        }
      }
    }
    if suppress {
      return Ok(());
    }
    for (suppressed_source, suppressed_event_code, count) in summaries {
      let summary = json!({"timestamp":Utc::now().to_rfc3339_opts(SecondsFormat::Millis,true),
                "level":"info","source":"isos","core":"isos","node_path":null,
                "event_code":"events_suppressed","run_id":run_id,"correlation_id":null,
                "message":"Repeated child events suppressed","pid":std::process::id(),
                "fields":{"suppressed_source":suppressed_source,"suppressed_event_code":suppressed_event_code,"count":count}});
      let _ = self.append_record(&self.path("isos")?, "isos", &summary, false);
    }
    let result = self.append_record(&self.path(source)?, source, value, false);
    if result.is_ok()
      && let Some((key, now)) = pending_key
    {
      self.suppression.lock().unwrap().insert(key, (now, 0));
    }
    result
  }

  #[cfg(test)]
  fn append_raw(&self, path: &Path, raw: &[u8]) {
    let _guard = self.lock.lock().unwrap();
    if let Err(error) = self.append_file(path, raw) {
      saturating_increment(&WRITE_FAILED);
      saturating_increment(&self.losses);
      eprintln!("isos capture sink failed at {}: {error:#}", path.display());
    }
  }

  pub fn read(&self, source: &str, cursor: Option<&str>, limit: usize) -> Result<Value> {
    self.read_filtered(source, cursor, limit, None)
  }

  pub fn read_filtered(
    &self,
    source: &str,
    cursor: Option<&str>,
    limit: usize,
    min_level: Option<&str>,
  ) -> Result<Value> {
    ensure!((1..=200).contains(&limit), "log limit must be 1..200");
    let min_rank = min_level
      .map(|level| level_rank(level).context("invalid log level"))
      .transpose()?;
    let source_path = self.path(source)?;
    let unified_fallback = !source_path.exists() && !matches!(source, "all" | "services");
    let path = if unified_fallback {
      self.path("all")?
    } else {
      source_path
    };
    let mut file =
      File::open(&path).with_context(|| format!("log source {source} is unavailable"))?;
    let meta = file.metadata()?;
    // Inodes can be reused during rapid rotation. Birth time is stable
    // while a file is appended and changes when a replacement is created.
    let birth_ns = meta
      .created()
      .ok()
      .and_then(|created| created.duration_since(std::time::UNIX_EPOCH).ok())
      .map_or(0, |duration| duration.as_nanos());
    let identity = format!("{:x}-{:x}-{birth_ns:x}", meta.dev(), meta.ino());
    let mut offset = 0_u64;
    let mut rotated = false;
    if let Some(cursor) = cursor {
      let (id, position) = cursor.rsplit_once(':').context("invalid log cursor")?;
      let position = position
        .parse::<u64>()
        .context("invalid log cursor offset")?;
      if id == identity && position <= meta.len() {
        offset = position;
      } else {
        rotated = true;
      }
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut reader = BufReader::new(file.take(MAX_READ_BYTES));
    let mut lines = Vec::new();
    let mut consumed = 0_u64;
    while lines.len() < limit {
      let mut raw = Vec::new();
      let size = reader.read_until(b'\n', &mut raw)?;
      if size == 0 {
        break;
      }
      if raw.last() != Some(&b'\n') && raw.len() == 8193 {
        break;
      }
      consumed += size as u64;
      if let Ok(value) = serde_json::from_slice::<Value>(&raw)
        && (!unified_fallback || value["source"].as_str() == Some(source))
        && (source != "services"
          || value["source"]
            .as_str()
            .is_some_and(|name| name.starts_with("service-")))
        && min_rank.is_none_or(|rank| {
          value["level"]
            .as_str()
            .and_then(level_rank)
            .is_some_and(|level| level >= rank)
        })
      {
        lines.push(value);
      }
    }
    let next_cursor = format!("{identity}:{}", offset + consumed);
    Ok(json!({"lines":lines,"next_cursor":next_cursor,"rotated":rotated}))
  }

  pub fn tail_filtered(
    &self,
    source: &str,
    limit: usize,
    min_level: Option<&str>,
  ) -> Result<Vec<Value>> {
    core_files::tail(self, source, min_level.unwrap_or("trace"), limit)
  }

  pub fn tail_human(&self, source: &str, limit: usize, min_level: &str) -> Result<Vec<String>> {
    Ok(
      core_files::tail(self, source, min_level, limit)?
        .into_iter()
        .map(|value| {
          format!(
            "{} [{}] {} {}",
            value["timestamp"].as_str().unwrap_or(""),
            value["level"]
              .as_str()
              .unwrap_or("info")
              .to_ascii_uppercase(),
            value["source"].as_str().unwrap_or(""),
            value["message"].as_str().unwrap_or("")
          )
        })
        .collect(),
    )
  }
}

fn remove_dev_owner_logs(dir: &Path) -> Result<()> {
  if !dir.is_dir() {
    return Ok(());
  }
  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    let path = entry.path();
    let kind = entry.file_type()?;
    if kind.is_dir() {
      remove_dev_owner_logs(&path)?;
    } else if kind.is_file()
      && path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
          name.ends_with(".log")
            || name.contains(".log.")
            || name.ends_with(".jsonl")
            || name.contains(".jsonl.")
        })
    {
      fs::remove_file(path)?;
    }
  }
  Ok(())
}

pub struct ChildAdmission {
  #[cfg(test)]
  #[expect(
    dead_code,
    reason = "test-only admission metadata retained for owner regression coverage"
  )]
  pub nodes: Option<Arc<BTreeSet<String>>>,
  #[cfg(test)]
  #[expect(
    dead_code,
    reason = "test-only admission metadata retained for owner regression coverage"
  )]
  pub skill_pins: BTreeSet<(String, String)>,
}

pub fn capture<R: std::io::Read + Send + 'static>(
  reader: R,
  store: LogStore,
  source: String,
  stream: &'static str,
  run_id: String,
  pid: u32,
  admission: Option<Arc<ChildAdmission>>,
) {
  std::thread::spawn(move || {
    capture_stream(
      reader,
      store,
      source,
      stream,
      &run_id,
      pid,
      admission.as_deref(),
    )
  });
}

/// The selected EOS stdout is inspected as raw bytes before any P06 log processing.
/// The channel receives one bounded transcript only after EOF; a descendant-held
/// writer therefore cannot accidentally satisfy the owner proof.
#[cfg(test)]
pub struct EosStdout {
  pub frame: Option<Vec<u8>>,
  pub at: Instant,
  pub eof: bool,
  pub invalid: bool,
}

#[cfg(test)]
pub fn capture_eos_stdout<R: Read + Send + 'static>(
  reader: R,
  store: LogStore,
  run_id: String,
  pid: u32,
) -> Receiver<EosStdout> {
  let (sender, receiver) = std::sync::mpsc::sync_channel(4);
  std::thread::spawn(move || {
    let raw_dir = store.root.join("capture");
    let raw_path = raw_dir.join("eos.stdout.txt");
    let raw_enabled = private_dir(&store.root.join("capture"))
      .and_then(|_| private_dir(&raw_dir))
      .is_ok();
    let mut reader = BufReader::new(reader);
    let mut count = 0;
    loop {
      let mut line = Vec::new();
      let result = Read::by_ref(&mut reader)
        .take(4097)
        .read_until(b'\n', &mut line);
      match result {
        Ok(0) => break,
        Err(_) => {
          let _ = sender.try_send(EosStdout {
            frame: None,
            at: Instant::now(),
            eof: false,
            invalid: true,
          });
          return;
        }
        Ok(_) => {
          let mut candidate = line
            .windows(b"eos_terminal_stop".len())
            .any(|bytes| bytes == b"eos_terminal_stop");
          let mut invalid = line.len() > 4096 || line.last() != Some(&b'\n');
          if line.last() != Some(&b'\n') {
            loop {
              let mut rest = Vec::new();
              match Read::by_ref(&mut reader)
                .take(4097)
                .read_until(b'\n', &mut rest)
              {
                Ok(0) | Err(_) => {
                  invalid = true;
                  break;
                }
                Ok(_) => {
                  if raw_enabled {
                    store.append_raw(&raw_path, &rest);
                  }
                  candidate |= rest
                    .windows(b"eos_terminal_stop".len())
                    .any(|bytes| bytes == b"eos_terminal_stop");
                  if rest.last() == Some(&b'\n') {
                    break;
                  }
                }
              }
            }
          }
          if candidate {
            count += 1;
            let _ = sender.try_send(EosStdout {
              frame: if !invalid && count <= 2 {
                Some(line.clone())
              } else {
                None
              },
              at: Instant::now(),
              eof: false,
              invalid: invalid || count > 2,
            });
          }
          if raw_enabled {
            store.append_raw(&raw_path, &line);
          }
        }
      }
    }
    let _ = (run_id, pid); // Pipe ownership is bound by its creating spawn.
    let _ = sender.try_send(EosStdout {
      frame: None,
      at: Instant::now(),
      eof: true,
      invalid: false,
    });
  });
  receiver
}

fn capture_stream<R: std::io::Read>(
  reader: R,
  store: LogStore,
  source: String,
  stream: &str,
  _run_id: &str,
  _pid: u32,
  _admission: Option<&ChildAdmission>,
) {
  let mut reader = BufReader::new(reader);
  let mut discarding = false;
  loop {
    let mut raw = Vec::new();
    let Ok(size) = Read::by_ref(&mut reader)
      .take(4097)
      .read_until(b'\n', &mut raw)
    else {
      break;
    };
    if size == 0 {
      break;
    }
    let complete = raw.last() == Some(&b'\n');
    if discarding {
      if complete {
        discarding = false;
      }
      continue;
    }
    if !complete {
      discarding = true;
      continue;
    }
    let message = String::from_utf8_lossy(&raw);
    let _ = store.append_process(
      &source,
      if stream == "stderr" { "warn" } else { "info" },
      stream,
      message.trim_end(),
    );
  }
}
