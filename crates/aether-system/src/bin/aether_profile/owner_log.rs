//! Bounded owner log projection for IsOS reconnects.
use std::{
  env, fs,
  io::{self, Read, Seek, SeekFrom},
  path::{Path, PathBuf},
};

/// Read a bounded page of owner and worker diagnostics for IsOS. The cursor is
/// a byte offset into the append-only JSONL file and advances only past whole
/// records, so reconnecting callers can resume without losing a partial line.
pub fn read_after(after: u64) -> io::Result<serde_json::Value> {
  let dir = match env::var_os("EOS_LOG_DIR") {
    Some(value) => PathBuf::from(value),
    None => env::current_dir()?.join("data/eos/log/local"),
  };
  read_after_path(&dir.join("eos.jsonl"), after)
}

fn read_after_path(path: &Path, after: u64) -> io::Result<serde_json::Value> {
  let mut file = match fs::File::open(path) {
    Ok(file) => file,
    Err(error) if error.kind() == io::ErrorKind::NotFound => {
      return Ok(serde_json::json!({"schema_version":1,"core":"eos","next_cursor":0,"entries":[]}));
    }
    Err(error) => return Err(error),
  };
  let length = file.metadata()?.len();
  let start = if after <= length { after } else { 0 };
  file.seek(SeekFrom::Start(start))?;
  let mut bytes = vec![0; 64 * 1024];
  let count = file.read(&mut bytes)?;
  bytes.truncate(count);
  let complete = bytes
    .iter()
    .rposition(|byte| *byte == b'\n')
    .map_or(0, |at| at + 1);
  let mut entries = Vec::new();
  for line in bytes[..complete]
    .split(|byte| *byte == b'\n')
    .filter(|line| !line.is_empty())
  {
    if let Ok(record) = serde_json::from_slice::<serde_json::Value>(line) {
      let level = record["level"].as_str().unwrap_or("");
      let message = record["message"].as_str().unwrap_or("");
      if matches!(level, "warning" | "warn" | "error" | "critical")
        || (level == "info"
          && (message.starts_with("EOS owner service")
            || message.starts_with("EOS run")
            || message.starts_with("EOS health worker")))
      {
        entries.push(record);
      }
    }
  }
  Ok(
    serde_json::json!({"schema_version":1,"core":"eos","next_cursor":start + complete as u64,"entries":entries}),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn log_pages_keep_warnings_errors_and_lifecycle_info() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("eos.jsonl");
    fs::write(&path, b"{\"level\":\"info\",\"message\":\"EOS owner service listening\"}\n{\"level\":\"info\",\"message\":\"detail\"}\n{\"level\":\"warn\",\"message\":\"warning\"}\n{\"level\":\"error\",\"message\":\"failure\"}\n").unwrap();
    let page = read_after_path(&path, 0).unwrap();
    let entries = page["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0]["message"], "EOS owner service listening");
    assert_eq!(entries[1]["message"], "warning");
    assert_eq!(entries[2]["message"], "failure");
    let next = page["next_cursor"].as_u64().unwrap();
    assert_eq!(
      read_after_path(&path, next).unwrap()["entries"],
      serde_json::json!([])
    );
  }
}
