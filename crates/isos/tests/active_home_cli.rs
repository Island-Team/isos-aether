use std::{fs, path::PathBuf, process::Command};

fn fixture() -> (PathBuf, PathBuf, PathBuf) {
  let nonce = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  let root = std::env::temp_dir().join(format!("isos-config-home-{}-{nonce}", std::process::id()));
  let xdg = root.join("xdg");
  let old = xdg.join("isos/deployments/default.nodes.json");
  fs::create_dir_all(old.parent().unwrap()).unwrap();
  fs::write(&old, b"old graph must not change").unwrap();
  let selected = root.join("selected");
  fs::create_dir_all(selected.join("config")).unwrap();
  fs::write(
    selected.join("config/isos.toml"),
    b"[server]\nhttp_port=37125\nui_port=37126\n",
  )
  .unwrap();
  (root, xdg, selected)
}

fn doctor(xdg: &PathBuf, selected: &PathBuf) -> std::process::Output {
  Command::new(env!("CARGO_BIN_EXE_isos"))
    .arg("doctor")
    .env("XDG_CONFIG_HOME", xdg)
    .env("ISOS_HOME", selected)
    .env("ISOS_CONFIG", selected.join("config/isos.toml"))
    .env("ISOS_HTTP_PORT", "37127")
    .env("ISOSUI_PORT", "37128")
    .output()
    .unwrap()
}

#[test]
fn process_environment_selects_home_without_touching_old_graph() {
  let (root, xdg, selected) = fixture();
  let output = doctor(&xdg, &selected);
  assert!(
    output.status.success(),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
  let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(report["interfaces"]["target_graph_format"], "0.2.0");
  let initialized = Command::new(env!("CARGO_BIN_EXE_isos"))
    .args(["config", "init"])
    .env("XDG_CONFIG_HOME", &xdg)
    .env("ISOS_HOME", &selected)
    .env("ISOS_CONFIG", selected.join("config/isos.toml"))
    .output()
    .unwrap();
  assert!(initialized.status.success());
  assert!(selected.join("config/graph-pool/demo.nodes.json").is_file());
  assert_eq!(
    fs::read(xdg.join("isos/deployments/default.nodes.json")).unwrap(),
    b"old graph must not change"
  );
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn retired_activation_file_is_ignored() {
  let (root, xdg, selected) = fixture();
  let active = xdg.join("isos/active.env");
  fs::write(
    &active,
    "export ISOS_HOME=/wrong\nexport ISOS_HTTP_PORT=80\n",
  )
  .unwrap();
  let output = doctor(&xdg, &selected);
  assert!(
    output.status.success(),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_process_override_fails_before_graph_access() {
  let (root, xdg, selected) = fixture();
  let output = Command::new(env!("CARGO_BIN_EXE_isos"))
    .arg("doctor")
    .env("XDG_CONFIG_HOME", &xdg)
    .env("ISOS_HOME", &selected)
    .env("ISOS_CONFIG", selected.join("config/isos.toml"))
    .env("ISOS_HTTP_PORT", "80")
    .output()
    .unwrap();
  assert!(!output.status.success());
  assert!(String::from_utf8_lossy(&output.stderr).contains("ISOS_HTTP_PORT must be 1024..65535"));
  assert_eq!(
    fs::read(xdg.join("isos/deployments/default.nodes.json")).unwrap(),
    b"old graph must not change"
  );
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_stop_and_vision_overrides_fail_at_startup() {
  let (root, xdg, selected) = fixture();
  for (key, value, expected) in [
    ("ISOS_STOP_TIMEOUT_MS", "4999", "stop_budget_invalid"),
    ("ISOS_EOS_STOP_GRACE_MS", "30000", "stop_budget_invalid"),
    ("ISOS_VISION_GATE_PATH", "bad/gate", "ISOS_VISION_GATE_PATH"),
  ] {
    let output = Command::new(env!("CARGO_BIN_EXE_isos"))
      .arg("doctor")
      .env("XDG_CONFIG_HOME", &xdg)
      .env("ISOS_HOME", &selected)
      .env("ISOS_CONFIG", selected.join("config/isos.toml"))
      .env(key, value)
      .output()
      .unwrap();
    assert!(!output.status.success(), "{key}");
    assert!(
      String::from_utf8_lossy(&output.stderr).contains(expected),
      "{key}: {}",
      String::from_utf8_lossy(&output.stderr)
    );
  }
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn valid_process_values_replace_invalid_toml_ranges() {
  let (root, xdg, selected) = fixture();
  fs::write(selected.join("config/isos.toml"), b"[server]\nhttp_port=80\nui_port=81\n[deployment]\nstop_timeout_ms=4999\n[logging]\nlevel='verbose'\n").unwrap();
  let selected_config = selected.join("config/isos.toml");
  let output = Command::new(env!("CARGO_BIN_EXE_isos"))
    .arg("doctor")
    .env("XDG_CONFIG_HOME", &xdg)
    .env("ISOS_HOME", &selected)
    .env("ISOS_CONFIG", &selected_config)
    .env("ISOS_HTTP_PORT", "37125")
    .env("ISOSUI_PORT", "37126")
    .env("ISOS_STOP_TIMEOUT_MS", "30000")
    .env("ISOS_LOG_LEVEL", "info")
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
  let output = Command::new(env!("CARGO_BIN_EXE_isos"))
    .arg("doctor")
    .env("XDG_CONFIG_HOME", &xdg)
    .env("ISOS_HOME", &selected)
    .env("ISOS_CONFIG", &selected_config)
    .env("ISOS_HTTP_PORT", "37125")
    .env("ISOSUI_PORT", "37126")
    .env("ISOS_STOP_TIMEOUT_MS", "30000")
    .env_remove("ISOS_LOG_LEVEL")
    .output()
    .unwrap();
  assert!(!output.status.success());
  assert!(String::from_utf8_lossy(&output.stderr).contains("logging.level"));
  fs::write(
    &selected_config,
    b"[server]\nhttp_port='not-a-port'\nui_port=37126\n",
  )
  .unwrap();
  let output = doctor(&xdg, &selected);
  assert!(!output.status.success());
  assert!(String::from_utf8_lossy(&output.stderr).contains("parse"));
  fs::remove_dir_all(root).unwrap();
}
