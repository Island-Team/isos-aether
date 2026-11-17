//! Durable admission for package v2 actuators. One dispatcher belongs to one
//! IsOS deployment generation; the owner supplies the exact graph bindings.
use std::{
  collections::{BTreeMap, BTreeSet},
  fs::{File, OpenOptions},
  io::{BufRead, BufReader, Write},
  os::unix::{
    fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    io::AsRawFd,
  },
  path::Path,
  sync::Mutex,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const MAX_RECORD: usize = 4 * 1024;
const MAX_COMMANDS: usize = 4096;
const MAX_COMMAND_ID: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalOutcome {
  Completed,
  Rejected,
  Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalResult {
  pub command_id: String,
  pub outcome: ExternalOutcome,
  pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ExternalBinding {
  pub node: String,
  pub port: String,
}

#[derive(Serialize, Deserialize)]
struct Record {
  run_id: String,
  revision: String,
  command_id: String,
  node: String,
  port: String,
  payload_digest: String,
  phase: String,
  result: Option<ExternalResult>,
}

struct Inner {
  file: File,
  seen: BTreeMap<String, (String, String, String, ExternalResult)>,
  closed: bool,
  unknown: bool,
}

pub struct ExternalDispatcher {
  run_id: String,
  revision: String,
  bindings: BTreeSet<(String, String)>,
  inner: Mutex<Inner>,
}

fn valid_id(value: &str) -> bool {
  !value.is_empty()
    && value.len() <= MAX_COMMAND_ID
    && value
      .bytes()
      .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}
fn append(inner: &mut Inner, record: &Record) -> Result<(), String> {
  let mut bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
  if bytes.len() > MAX_RECORD {
    return Err("dispatcher record too large".into());
  }
  bytes.push(b'\n');
  inner.file.write_all(&bytes).map_err(|e| e.to_string())?;
  inner.file.sync_all().map_err(|e| e.to_string())
}
impl ExternalDispatcher {
  pub fn open(
    path: &Path,
    run_id: &str,
    revision: &str,
    bindings: Vec<ExternalBinding>,
  ) -> Result<Self, String> {
    if !valid_id(run_id)
      || revision.len() != 64
      || !revision
        .bytes()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
      || !path.is_absolute()
    {
      return Err("invalid dispatcher generation".into());
    }
    let mut allowed = BTreeSet::new();
    for binding in bindings {
      if !binding.node.starts_with("eos/")
        || binding.port.is_empty()
        || !allowed.insert((binding.node, binding.port))
      {
        return Err("invalid or duplicate external binding".into());
      }
    }
    if allowed.is_empty() {
      return Err("empty external bindings".into());
    }
    let file = OpenOptions::new()
      .create(true)
      .append(true)
      .read(true)
      .mode(0o600)
      .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
      .open(path)
      .map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.file_type().is_file()
      || metadata.nlink() != 1
      || metadata.uid() != unsafe { libc::geteuid() }
      || metadata.permissions().mode() & 0o077 != 0
      || metadata.len() > (MAX_RECORD * MAX_COMMANDS * 2) as u64
    {
      return Err("unsafe dispatcher journal path".into());
    }
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
      return Err("dispatcher journal already owned".into());
    }
    let parent = path.parent().ok_or("dispatcher journal parent missing")?;
    File::open(parent)
      .and_then(|dir| dir.sync_all())
      .map_err(|e| e.to_string())?;
    let mut seen = BTreeMap::new();
    let mut pending = BTreeSet::new();
    let mut closed = false;
    let mut unknown = false;
    let reader = file.try_clone().map_err(|e| e.to_string())?;
    let mut record_count = 0usize;
    for line in BufReader::new(reader).lines() {
      record_count += 1;
      if record_count > MAX_COMMANDS * 2 + 1 {
        return Err("dispatcher journal record limit exceeded".into());
      }
      let line = line.map_err(|e| e.to_string())?;
      if line.len() > MAX_RECORD {
        return Err("dispatcher journal record too large".into());
      }
      let record: Record = serde_json::from_str(&line).map_err(|e| e.to_string())?;
      if record.run_id != run_id || record.revision != revision {
        return Err("dispatcher journal generation mismatch".into());
      }
      if record.phase == "prepared" {
        pending.insert(record.command_id.clone());
      } else if record.phase == "closed" {
        closed = true;
      } else if record.phase == "result" {
        pending.remove(&record.command_id);
        let result = record.result.ok_or("missing dispatcher result")?;
        unknown |= result.outcome == ExternalOutcome::Unknown;
        seen.insert(
          record.command_id,
          (record.node, record.port, record.payload_digest, result),
        );
      } else {
        return Err("invalid dispatcher journal phase".into());
      }
    }
    Ok(Self {
      run_id: run_id.into(),
      revision: revision.into(),
      bindings: allowed,
      inner: Mutex::new(Inner {
        file,
        seen,
        closed,
        unknown: unknown || !pending.is_empty(),
      }),
    })
  }
  pub fn submit<F>(
    &self,
    command_id: &str,
    node: &str,
    port: &str,
    payload: &Value,
    deliver: F,
  ) -> ExternalResult
  where
    F: FnOnce() -> Result<(), String>,
  {
    let rejected = |reason: &str| ExternalResult {
      command_id: command_id.into(),
      outcome: ExternalOutcome::Rejected,
      reason: Some(reason.into()),
    };
    let Ok(payload_bytes) = serde_json::to_vec(payload) else {
      return rejected("invalid command payload");
    };
    if payload_bytes.len() > 64 * 1024 {
      return rejected("command payload too large");
    }
    let payload_digest = format!("{:x}", Sha256::digest(&payload_bytes));
    if !valid_id(command_id)
      || !self.bindings.contains(&(node.to_owned(), port.to_owned()))
      || payload.is_null()
    {
      return rejected("invalid command identity or binding");
    }
    let Ok(mut inner) = self.inner.lock() else {
      return rejected("dispatcher lock poisoned");
    };
    if let Some((old_node, old_port, old_digest, result)) = inner.seen.get(command_id) {
      return if old_node == node && old_port == port && old_digest == &payload_digest {
        result.clone()
      } else {
        rejected("command ID reused for another binding")
      };
    }
    if inner.closed || inner.unknown {
      return rejected("dispatcher admission closed");
    }
    if inner.seen.len() >= MAX_COMMANDS {
      return rejected("dispatcher command limit reached");
    }
    let prepared = Record {
      run_id: self.run_id.clone(),
      revision: self.revision.clone(),
      command_id: command_id.into(),
      node: node.into(),
      port: port.into(),
      payload_digest: payload_digest.clone(),
      phase: "prepared".into(),
      result: None,
    };
    if append(&mut inner, &prepared).is_err() {
      inner.unknown = true;
      return rejected("dispatcher reservation durability failed");
    }
    let result = match deliver() {
      Ok(()) => ExternalResult {
        command_id: command_id.into(),
        outcome: ExternalOutcome::Completed,
        reason: None,
      },
      Err(reason) => ExternalResult {
        command_id: command_id.into(),
        outcome: ExternalOutcome::Unknown,
        reason: Some(reason),
      },
    };
    let finished = Record {
      phase: "result".into(),
      result: Some(result.clone()),
      ..prepared
    };
    if append(&mut inner, &finished).is_err() {
      inner.unknown = true;
      return ExternalResult {
        command_id: command_id.into(),
        outcome: ExternalOutcome::Unknown,
        reason: Some("dispatcher result durability failed".into()),
      };
    }
    if result.outcome == ExternalOutcome::Unknown {
      inner.unknown = true;
    }
    inner.seen.insert(
      command_id.into(),
      (node.into(), port.into(), payload_digest, result.clone()),
    );
    result
  }
  pub fn close(&self) -> Result<(), String> {
    let mut inner = self.inner.lock().map_err(|_| "dispatcher lock poisoned")?;
    if inner.closed {
      return Ok(());
    }
    inner.closed = true;
    let record = Record {
      run_id: self.run_id.clone(),
      revision: self.revision.clone(),
      command_id: "stop".into(),
      node: String::new(),
      port: String::new(),
      payload_digest: String::new(),
      phase: "closed".into(),
      result: None,
    };
    append(&mut inner, &record)
  }
  pub fn uncertain(&self) -> bool {
    self.inner.lock().map_or(true, |inner| inner.unknown)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn durable_admission_dedup_and_stop() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal");
    let rev = "a".repeat(64);
    let binding = ExternalBinding {
      node: "eos/hardware/base/b".into(),
      port: "command".into(),
    };
    let dispatcher = ExternalDispatcher::open(&path, "run", &rev, vec![binding.clone()]).unwrap();
    let mut calls = 0;
    assert_eq!(
      dispatcher
        .submit(
          "cmd",
          "eos/hardware/base/b",
          "command",
          &serde_json::json!({}),
          || {
            calls += 1;
            Ok(())
          }
        )
        .outcome,
      ExternalOutcome::Completed
    );
    assert_eq!(
      dispatcher
        .submit(
          "cmd",
          "eos/hardware/base/b",
          "command",
          &serde_json::json!({}),
          || {
            calls += 1;
            Ok(())
          }
        )
        .outcome,
      ExternalOutcome::Completed
    );
    assert_eq!(calls, 1);
    assert_eq!(
      dispatcher
        .submit(
          "cmd",
          "eos/hardware/base/b",
          "command",
          &serde_json::json!({"changed":true}),
          || {
            calls += 1;
            Ok(())
          }
        )
        .outcome,
      ExternalOutcome::Rejected
    );
    assert_eq!(calls, 1);
    assert_eq!(
      dispatcher
        .submit(
          "cmd",
          "eos/hardware/base/b",
          "other",
          &serde_json::json!({}),
          || Ok(())
        )
        .outcome,
      ExternalOutcome::Rejected
    );
    dispatcher.close().unwrap();
    assert_eq!(
      dispatcher
        .submit(
          "next",
          "eos/hardware/base/b",
          "command",
          &serde_json::json!({}),
          || Ok(())
        )
        .outcome,
      ExternalOutcome::Rejected
    );
    drop(dispatcher);
    let reopened = ExternalDispatcher::open(&path, "run", &rev, vec![binding]).unwrap();
    assert_eq!(
      reopened
        .submit(
          "next",
          "eos/hardware/base/b",
          "command",
          &serde_json::json!({}),
          || Ok(())
        )
        .outcome,
      ExternalOutcome::Rejected
    );
  }
  #[test]
  fn ambiguous_delivery_inhibits_following_commands() {
    let dir = tempfile::tempdir().unwrap();
    let rev = "a".repeat(64);
    let dispatcher = ExternalDispatcher::open(
      &dir.path().join("journal"),
      "run",
      &rev,
      vec![ExternalBinding {
        node: "eos/hardware/base/b".into(),
        port: "command".into(),
      }],
    )
    .unwrap();
    assert_eq!(
      dispatcher
        .submit(
          "cmd",
          "eos/hardware/base/b",
          "command",
          &serde_json::json!({}),
          || Err("timeout".into())
        )
        .outcome,
      ExternalOutcome::Unknown
    );
    assert!(dispatcher.uncertain());
    assert_eq!(
      dispatcher
        .submit(
          "next",
          "eos/hardware/base/b",
          "command",
          &serde_json::json!({}),
          || Ok(())
        )
        .outcome,
      ExternalOutcome::Rejected
    );
  }
  #[test]
  fn journal_symlink_and_concurrent_owner_are_rejected() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let rev = "a".repeat(64);
    let binding = ExternalBinding {
      node: "eos/hardware/base/b".into(),
      port: "command".into(),
    };
    let target = dir.path().join("target");
    File::create(&target).unwrap();
    let link = dir.path().join("link");
    symlink(&target, &link).unwrap();
    assert!(ExternalDispatcher::open(&link, "run", &rev, vec![binding.clone()]).is_err());
    let path = dir.path().join("journal");
    let first = ExternalDispatcher::open(&path, "run", &rev, vec![binding.clone()]).unwrap();
    assert!(ExternalDispatcher::open(&path, "run", &rev, vec![binding]).is_err());
    drop(first);
  }
}
