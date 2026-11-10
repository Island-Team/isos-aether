//! One local JSONL stream shared by the EOS owner and health worker.
use std::{
  env,
  fs::{self, OpenOptions},
  io::{self, Write},
  os::unix::fs::{OpenOptionsExt, PermissionsExt},
  path::PathBuf,
};

pub fn append_with_run(
  level: &str,
  source: &str,
  message: &str,
  run_id: Option<&str>,
) -> io::Result<()> {
  let dir = match env::var_os("EOS_LOG_DIR") {
    Some(value) => PathBuf::from(value),
    None => env::current_dir()?.join("data/eos/log/local"),
  };
  if !dir.is_absolute() {
    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      "EOS_LOG_DIR must be absolute",
    ));
  }
  fs::create_dir_all(&dir)?;
  fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
  let mut file = OpenOptions::new()
    .create(true)
    .append(true)
    .mode(0o600)
    .open(dir.join("eos.jsonl"))?;
  let mut record = serde_json::json!({
    "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    "level": level,
    "source": format!("{source}@aether-v{}", env!("CARGO_PKG_VERSION")),
    "message": message,
  });
  if let Some(run_id) = run_id {
    record["run_id"] = serde_json::json!(run_id);
  }
  let mut line = serde_json::to_vec(&record)?;
  line.push(b'\n');
  file.write_all(&line)
}
