//! Local EOS diagnostics in the shared per-core JSONL stream.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static WRITE_LOCK: Mutex<()> = Mutex::new(());
static WRITE_LOSSES: AtomicU64 = AtomicU64::new(0);

pub fn write_losses() -> u64 {
  WRITE_LOSSES.load(Ordering::Relaxed)
}

/// Select a local EOS diagnostic directory. Managed callers provide EOS_LOG_DIR;
/// standalone tools use the selected home, checkout, or executable data tree.
pub fn directory() -> PathBuf {
  std::env::var_os("EOS_LOG_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|| {
      if let Some(home) = std::env::var_os("ISOS_HOME") {
        return PathBuf::from(home).join("data/eos/log/local");
      }
      if let Ok(cwd) = std::env::current_dir() {
        if let Some(project) = cwd.ancestors().find(|candidate| {
          candidate.join("Cargo.toml").is_file() && candidate.join("crates/aether-common").is_dir()
        }) {
          return project.join("data/eos/log/local");
        }
      }
      std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("data/eos/log/local")
    })
}

pub fn write(
  dir: &Path,
  process: &str,
  level: &str,
  code: &str,
  message: &str,
  run_id: Option<&str>,
) -> io::Result<()> {
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default();
  let mut utc = unsafe { std::mem::zeroed::<libc::tm>() };
  let secs = now.as_secs() as libc::time_t;
  unsafe { libc::gmtime_r(&secs, &mut utc) };
  let timestamp = format!(
    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
    utc.tm_year + 1900,
    utc.tm_mon + 1,
    utc.tm_mday,
    utc.tm_hour,
    utc.tm_min,
    utc.tm_sec,
    now.subsec_millis()
  );
  let clean = |value: &str| {
    let bounded = value
      .chars()
      .take(4096)
      .map(|c| if c.is_control() { ' ' } else { c })
      .collect::<String>();
    bounded
      .split_whitespace()
      .map(|word| {
        let lower = word.to_ascii_lowercase();
        for marker in [
          "password=",
          "token=",
          "secret=",
          "authorization:",
          "api_key=",
        ] {
          if let Some(index) = lower.find(marker) {
            return format!(
              "{}{}[REDACTED]",
              &word[..index],
              &word[index..index + marker.len()]
            );
          }
        }
        word.to_owned()
      })
      .collect::<Vec<_>>()
      .join(" ")
  };
  let mut record = serde_json::json!({
    "timestamp": timestamp,
    "level": level.to_ascii_lowercase(),
    "source": format!("eos.{process}@aether-v{}", env!("CARGO_PKG_VERSION")),
    "message": format!("{}: {}", clean(code), clean(message)),
  });
  if let Some(run_id) = run_id {
    record["run_id"] = serde_json::json!(clean(run_id));
  }
  append_record(dir, &record)
}

pub fn append_record(dir: &Path, record: &serde_json::Value) -> io::Result<()> {
  let _guard = WRITE_LOCK
    .lock()
    .map_err(|_| io::Error::other("diagnostic lock poisoned"))?;
  fs::create_dir_all(dir)?;
  fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
  let path = dir.join("eos.jsonl");
  let mut line = serde_json::to_vec(&record)?;
  line.push(b'\n');
  let mut file = OpenOptions::new()
    .create(true)
    .append(true)
    .mode(0o600)
    .open(path)?;
  file.write_all(&line)
}

pub fn report(
  dir: &Path,
  process: &str,
  level: &str,
  code: &str,
  message: &str,
  run_id: Option<&str>,
) {
  if let Err(error) = write(dir, process, level, code, message, run_id) {
    let lost = WRITE_LOSSES
      .fetch_add(1, Ordering::Relaxed)
      .saturating_add(1);
    eprintln!("eos diagnostic sink failed ({process}), lost_count={lost}: {error}");
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn diagnostic_file_uses_shared_jsonl_envelope_and_bounds_lines() {
    let dir = std::env::temp_dir().join(format!("eos-diagnostic-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    write(
      &dir,
      "runtime",
      "error",
      "profile_invalid",
      &"x".repeat(8000),
      Some("r_test"),
    )
    .unwrap();
    let output = fs::read_to_string(dir.join("eos.jsonl")).unwrap();
    let first: serde_json::Value = serde_json::from_str(output.lines().next().unwrap()).unwrap();
    assert_eq!(first["level"], "error");
    assert_eq!(first["source"], "eos.runtime@aether-v0.2.1");
    assert_eq!(first["run_id"], "r_test");
    assert!(first["message"]
      .as_str()
      .unwrap()
      .starts_with("profile_invalid:"));
    assert!(output.len() < 5000);
    write(
      &dir,
      "runtime",
      "error",
      "config_failed",
      "token=abc123 password=private",
      None,
    )
    .unwrap();
    let output = fs::read_to_string(dir.join("eos.jsonl")).unwrap();
    assert!(!output.contains("abc123"));
    assert!(!output.contains("private"));
    fs::remove_dir_all(dir).unwrap();
  }
}
