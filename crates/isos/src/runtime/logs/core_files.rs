//! Local, per-core JSONL reads for the workbench. Owner sockets carry health only.

use super::{CurrentError, CurrentQuery, LogStore, cursor, level_rank, valid_source};
use crate::app::config::Paths;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
  fs::OpenOptions,
  io::{BufRead, BufReader, Read, Seek, SeekFrom},
  os::unix::fs::{MetadataExt, OpenOptionsExt},
  path::PathBuf,
};

#[derive(Default, Serialize, Deserialize)]
struct Cursor {
  filter: String,
  run_id: String,
  files: [FileCursor; 3],
}

#[derive(Default, Serialize, Deserialize)]
struct FileCursor {
  identity: String,
  offset: u64,
  discarding: bool,
}

struct Row {
  core: usize,
  end: u64,
  value: Value,
}

fn core_paths(store: &LogStore, paths: &Paths) -> [PathBuf; 3] {
  let deployment = store.root().file_name().unwrap_or_default();
  [
    store.root().join("isos.jsonl"),
    paths
      .owner_data("eos")
      .join("log")
      .join(deployment)
      .join("eos.jsonl"),
    paths
      .owner_data("ei")
      .join("log")
      .join(deployment)
      .join("ei.jsonl"),
  ]
}

fn invalid(code: &'static str, path: &'static str, message: &'static str) -> CurrentError {
  CurrentError {
    code,
    path,
    message,
    status: 400,
  }
}

fn source_matches(selected: &str, core: &str, actual: &str) -> bool {
  selected == "all"
    || selected == core
    || (selected == "services" && actual.starts_with("isos.runtime.service-"))
    || actual.contains(&format!(".{selected}."))
}

fn read(
  store: &LogStore,
  paths: &Paths,
  scope_run_id: Option<&str>,
  query: CurrentQuery<'_>,
) -> Result<Value, CurrentError> {
  if !matches!(query.source, "all" | "services") && !valid_source(query.source) {
    return Err(invalid(
      "log_source_invalid",
      "/query/source",
      "Log source invalid",
    ));
  }
  let rank = level_rank(query.level)
    .ok_or_else(|| invalid("log_level_invalid", "/query/level", "Log level invalid"))?;
  if !(1..=200).contains(&query.limit) {
    return Err(invalid(
      "log_limit_invalid",
      "/query/limit",
      "Log limit invalid",
    ));
  }
  if query
    .node_path
    .is_some_and(|path| path.len() > 256 || !path.starts_with('/'))
  {
    return Err(invalid(
      "log_node_path_invalid",
      "/query/node_path",
      "Log node path invalid",
    ));
  }
  if query
    .correlation_id
    .is_some_and(|id| id.len() > 64 || id.is_empty())
  {
    return Err(invalid(
      "log_correlation_id_invalid",
      "/query/correlation_id",
      "Log correlation ID invalid",
    ));
  }
  let (run_id, known) = store.run_identity();
  let (kind, run_id) = match (scope_run_id, run_id, known) {
    (Some(id), _, _) => ("deployment_run", id.to_owned()),
    (None, Some(id), "known") => ("deployment_run", id),
    _ => (
      "manager_session",
      store.manager_scope_id().unwrap_or_else(|| "m0".into()),
    ),
  };
  let filter = format!(
    "{}|{}|{}|{}",
    query.source,
    query.level,
    query.node_path.unwrap_or(""),
    query.correlation_id.unwrap_or("")
  );
  let mut cursor = if let Some(raw) = query.cursor {
    if raw.len() > 512 || !raw.is_ascii() {
      return Err(invalid(
        "log_cursor_invalid",
        "/query/cursor",
        "Log cursor invalid",
      ));
    }
    let parts: Vec<_> = raw.split('.').collect();
    if parts.len() != 3 || parts[0] != "lf1" {
      return Err(invalid(
        "log_cursor_invalid",
        "/query/cursor",
        "Log cursor invalid",
      ));
    }
    let token = parts[1];
    let actual = URL_SAFE_NO_PAD
      .decode(parts[2])
      .map_err(|_| invalid("log_cursor_invalid", "/query/cursor", "Log cursor invalid"))?;
    let key = cursor::key(store)
      .map_err(|_| invalid("log_cursor_invalid", "/query/cursor", "Log cursor invalid"))?;
    let expected = cursor::mac(&key, token.as_bytes());
    if actual.len() != 32
      || actual
        .iter()
        .zip(expected)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        != 0
    {
      return Err(invalid(
        "log_cursor_invalid",
        "/query/cursor",
        "Log cursor invalid",
      ));
    }
    let bytes = URL_SAFE_NO_PAD
      .decode(token)
      .map_err(|_| invalid("log_cursor_invalid", "/query/cursor", "Log cursor invalid"))?;
    if bytes.len() > 384 {
      return Err(invalid(
        "log_cursor_invalid",
        "/query/cursor",
        "Log cursor invalid",
      ));
    }
    serde_json::from_slice::<Cursor>(&bytes)
      .map_err(|_| invalid("log_cursor_invalid", "/query/cursor", "Log cursor invalid"))?
  } else {
    Cursor::default()
  };
  if query.cursor.is_some() && cursor.filter != filter {
    return Err(invalid(
      "log_cursor_filter_mismatch",
      "/query/cursor",
      "Log cursor filters changed",
    ));
  }
  let run_changed = query.cursor.is_some() && cursor.run_id != run_id;
  if run_changed {
    cursor.files = Default::default();
  }
  cursor.filter = filter;
  cursor.run_id = run_id.clone();
  let mut rows = Vec::new();
  let mut rotated = false;
  let mut gaps = Vec::new();
  for (index, path) in core_paths(store, paths).into_iter().enumerate() {
    let Ok(mut file) = OpenOptions::new()
      .read(true)
      .custom_flags(libc::O_NOFOLLOW)
      .open(&path)
    else {
      continue;
    };
    let Ok(meta) = file.metadata() else { continue };
    if !meta.is_file() {
      continue;
    }
    let identity = format!("{:x}-{:x}", meta.dev(), meta.ino());
    let state = &mut cursor.files[index];
    if !state.identity.is_empty() && (state.identity != identity || state.offset > meta.len()) {
      rotated = true;
      state.offset = 0;
      state.discarding = false;
    }
    state.identity = identity;
    if file.seek(SeekFrom::Start(state.offset)).is_err() {
      continue;
    }
    let mut reader = BufReader::new(file);
    let mut offset = state.offset;
    let mut scanned = 0usize;
    let mut matched = 0usize;
    let mut discarding = state.discarding;
    while scanned < 262_144 && matched < query.limit {
      let mut raw = Vec::new();
      let Ok(size) = Read::by_ref(&mut reader)
        .take(8193)
        .read_until(b'\n', &mut raw)
      else {
        break;
      };
      if size == 0 {
        break;
      }
      if discarding {
        offset += size as u64;
        scanned += size;
        if raw.last() == Some(&b'\n') {
          discarding = false;
        }
        continue;
      }
      if raw.last() != Some(&b'\n') {
        if size < 8193 {
          break;
        }
        offset += size as u64;
        scanned += size;
        discarding = true;
        gaps.push(json!({"code":"log_collector_gap","count":null}));
        continue;
      }
      offset += size as u64;
      scanned += size;
      if raw.len() > 8192 {
        gaps.push(json!({"code":"log_collector_gap","count":null}));
        continue;
      }
      let Ok(mut value) = serde_json::from_slice::<Value>(&raw) else {
        gaps.push(json!({"code":"log_collector_gap","count":null}));
        continue;
      };
      let core = ["isos", "eos", "ei"][index];
      let Some(source) = value["source"].as_str() else {
        continue;
      };
      let Some(timestamp) = value["timestamp"].as_str() else {
        continue;
      };
      if chrono::DateTime::parse_from_rfc3339(timestamp).is_err()
        || !source.starts_with(&format!("{core}."))
        || !source.ends_with("@aether-v0.2.1")
      {
        continue;
      }
      if !source_matches(query.source, core, source) {
        continue;
      }
      if !value["level"]
        .as_str()
        .and_then(level_rank)
        .is_some_and(|level| level >= rank)
      {
        continue;
      }
      if query
        .node_path
        .is_some_and(|path| value["node_path"].as_str() != Some(path))
      {
        continue;
      }
      if query
        .correlation_id
        .is_some_and(|id| value["correlation_id"].as_str() != Some(id))
      {
        continue;
      }
      if !value["message"]
        .as_str()
        .is_some_and(|message| message.len() <= 4096)
      {
        continue;
      }
      value["core"] = json!(core);
      if value["run_id"].as_str() != Some(&run_id) {
        continue;
      }
      if value["event_code"].is_null() {
        value["event_code"] = json!("message");
      }
      rows.push(Row {
        core: index,
        end: offset,
        value,
      });
      matched += 1;
    }
    if matched == 0 {
      state.offset = offset;
      state.discarding = discarding;
    }
  }
  rows.sort_by(|a, b| {
    a.value["timestamp"]
      .as_str()
      .cmp(&b.value["timestamp"].as_str())
      .then(a.core.cmp(&b.core))
      .then(a.end.cmp(&b.end))
  });
  let mut output = Vec::new();
  for row in rows.into_iter().take(query.limit) {
    cursor.files[row.core].offset = row.end;
    cursor.files[row.core].discarding = false;
    output.push(row.value);
  }
  if run_changed {
    gaps.push(json!({"code":"log_run_changed","count":null}));
  }
  if rotated {
    gaps.push(json!({"code":"log_cursor_expired","count":null}));
  }
  let token = URL_SAFE_NO_PAD.encode(
    serde_json::to_vec(&cursor)
      .map_err(|_| invalid("log_cursor_invalid", "/query/cursor", "Log cursor invalid"))?,
  );
  let key = cursor::key(store)
    .map_err(|_| invalid("log_cursor_invalid", "/query/cursor", "Log cursor invalid"))?;
  let signature = URL_SAFE_NO_PAD.encode(cursor::mac(&key, token.as_bytes()));
  Ok(json!({
    "scope":{"kind":kind,"run_id":run_id,"active_revision":query.active_revision},
    "anchor":{"kind":if kind == "deployment_run" {"first_ready_or_failure"} else {"first_prelaunch_failure"},"state":"pending","event":null},
    "lines":output,"next_cursor":format!("lf1.{token}.{signature}"),"rotated":rotated,"diagnostics":gaps
  }))
}

pub(super) fn read_current(
  store: &LogStore,
  paths: &Paths,
  run_id: Option<&str>,
  query: CurrentQuery<'_>,
) -> Result<Value, CurrentError> {
  read(store, paths, run_id, query)
}

pub(super) fn tail(
  store: &LogStore,
  source: &str,
  level: &str,
  limit: usize,
) -> anyhow::Result<Vec<Value>> {
  anyhow::ensure!((1..=1000).contains(&limit), "log limit must be 1..1000");
  let rank = level_rank(level).ok_or_else(|| anyhow::anyhow!("invalid log level"))?;
  anyhow::ensure!(
    matches!(
      source,
      "all" | "isos" | "eos" | "ei" | "launcher" | "install"
    ),
    "invalid log source"
  );
  let data = store
    .root()
    .ancestors()
    .nth(3)
    .ok_or_else(|| anyhow::anyhow!("log root unavailable"))?;
  let paths = Paths {
    config: PathBuf::new(),
    data: data.to_path_buf(),
    runtime: PathBuf::new(),
  };
  let mut files = core_paths(store, &paths).to_vec();
  for (category, filename) in [("launch", "launcher.jsonl"), ("install", "install.jsonl")] {
    let directory = data.join("log").join(category);
    if directory.is_dir() {
      for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
          files.push(entry.path().join(filename));
        }
      }
    }
  }
  let mut output = Vec::new();
  for path in files {
    let core = path
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or("")
      .trim_end_matches(".jsonl");
    if source != "all" && source != core {
      continue;
    }
    let Ok(file) = OpenOptions::new()
      .read(true)
      .custom_flags(libc::O_NOFOLLOW)
      .open(&path)
    else {
      continue;
    };
    let mut reader = BufReader::new(file);
    let mut consumed = 0usize;
    while consumed < 8_388_608 {
      let mut raw = Vec::new();
      let size = Read::by_ref(&mut reader)
        .take(8193)
        .read_until(b'\n', &mut raw)?;
      if size == 0 {
        break;
      }
      consumed += size;
      if raw.last() != Some(&b'\n') || raw.len() > 8192 {
        continue;
      }
      let Ok(value) = serde_json::from_slice::<Value>(&raw) else {
        continue;
      };
      if !value["source"].as_str().is_some_and(|text| {
        text.starts_with(&format!("{core}."))
          || matches!(core, "launcher" | "install") && text.starts_with(&format!("isos.{core}."))
      }) {
        continue;
      }
      if !value["level"]
        .as_str()
        .and_then(level_rank)
        .is_some_and(|value| value >= rank)
      {
        continue;
      }
      if value["timestamp"]
        .as_str()
        .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
        .is_none()
      {
        continue;
      }
      output.push(value);
    }
  }
  output.sort_by(|a, b| a["timestamp"].as_str().cmp(&b["timestamp"].as_str()));
  Ok(
    output
      .into_iter()
      .rev()
      .take(limit)
      .collect::<Vec<_>>()
      .into_iter()
      .rev()
      .collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{fs, io::Write};

  fn query<'a>(cursor: Option<&'a str>, limit: usize) -> CurrentQuery<'a> {
    CurrentQuery {
      source: "all",
      level: "info",
      node_path: None,
      correlation_id: None,
      cursor,
      limit,
      active_revision: None,
    }
  }

  #[test]
  fn merges_three_local_files_with_signed_cursor() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data/isos"),
      runtime: temp.path().join("run"),
    };
    let store = LogStore::new(&paths, "demo").unwrap();
    let run_id = store.manager_scope_id().unwrap();
    for (core, time) in [
      ("isos", "2026-11-09T09:00:00Z"),
      ("eos", "2026-11-09T09:00:01Z"),
      ("ei", "2026-11-09T09:00:02Z"),
    ] {
      let path = core_paths(&store, &paths)
        .into_iter()
        .find(|path| path.file_name().unwrap().to_string_lossy() == format!("{core}.jsonl"))
        .unwrap();
      fs::create_dir_all(path.parent().unwrap()).unwrap();
      let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
      writeln!(file, "{}", json!({"timestamp":time,"level":"info","source":format!("{core}.runtime.health.worker@aether-v0.2.1"),"message":core,"run_id":run_id})).unwrap();
    }
    let mut cursor = None;
    let mut seen = Vec::new();
    for _ in 0..3 {
      let page = read_current(&store, &paths, None, query(cursor.as_deref(), 1)).unwrap();
      seen.push(page["lines"][0]["message"].as_str().unwrap().to_owned());
      cursor = page["next_cursor"].as_str().map(str::to_owned);
      assert!(cursor.as_ref().unwrap().len() <= 512);
    }
    assert_eq!(seen, ["isos", "eos", "ei"]);
    let page = read_current(&store, &paths, None, query(cursor.as_deref(), 1)).unwrap();
    assert!(page["lines"].as_array().unwrap().is_empty());
    let mut bad = cursor.unwrap();
    bad.push('x');
    assert_eq!(
      read_current(&store, &paths, None, query(Some(&bad), 1))
        .unwrap_err()
        .code,
      "log_cursor_invalid"
    );
  }

  #[test]
  fn ignores_other_core_prefix_and_symlink() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data/isos"),
      runtime: temp.path().join("run"),
    };
    let store = LogStore::new(&paths, "demo").unwrap();
    let eos = core_paths(&store, &paths)[1].clone();
    fs::create_dir_all(eos.parent().unwrap()).unwrap();
    let target = temp.path().join("outside");
    fs::write(&target, format!("{}\n", json!({"timestamp":"2026-11-09T09:00:00Z","level":"error","source":"eos.fake@aether-v0.2.1","message":"outside"}))).unwrap();
    std::os::unix::fs::symlink(&target, &eos).unwrap();
    let page = read_current(&store, &paths, None, query(None, 10)).unwrap();
    assert!(page["lines"].as_array().unwrap().is_empty());
  }

  #[test]
  fn current_scope_skips_previous_run_and_oversized_line() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data/isos"),
      runtime: temp.path().join("run"),
    };
    let store = LogStore::new(&paths, "demo").unwrap();
    store.publish_run("rnew").unwrap();
    let eos = core_paths(&store, &paths)[1].clone();
    fs::create_dir_all(eos.parent().unwrap()).unwrap();
    let mut file = fs::OpenOptions::new()
      .append(true)
      .create(true)
      .open(eos)
      .unwrap();
    writeln!(file, "{}", json!({"timestamp":"2026-11-09T09:00:00Z","level":"error","source":"eos.owner_service.rs@aether-v0.2.1","message":"old","run_id":"rold"})).unwrap();
    writeln!(file, "{}", "x".repeat(20_000)).unwrap();
    writeln!(file, "{}", json!({"timestamp":"2026-11-09T09:00:01Z","level":"error","source":"eos.owner_service.rs@aether-v0.2.1","message":"new","run_id":"rnew"})).unwrap();
    let page = read_current(&store, &paths, None, query(None, 10)).unwrap();
    assert_eq!(page["lines"].as_array().unwrap().len(), 1);
    assert_eq!(page["lines"][0]["message"], "new");
    assert_eq!(page["diagnostics"][0]["code"], "log_collector_gap");
  }

  #[test]
  fn isos_records_attach_owner_run_identity() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data/isos"),
      runtime: temp.path().join("run"),
    };
    fs::create_dir_all(&paths.runtime).unwrap();
    fs::write(
      paths.runtime.join("demo.owner-run.json"),
      br#"{"run_id":"ractive"}"#,
    )
    .unwrap();
    let store = LogStore::new(&paths, "demo").unwrap();
    store
      .append_process("ui", "error", "ui_exit", "frontend exited")
      .unwrap();
    store
      .append_diagnostic("error", "owner_failed", "owner stopped")
      .unwrap();
    let page = read_current(&store, &paths, Some("ractive"), query(None, 10)).unwrap();
    let lines = page["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| line["run_id"] == "ractive"));
  }
}
