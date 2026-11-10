use std::process::Command;

fn isos(home: &std::path::Path, args: &[&str]) -> std::process::Output {
  Command::new(env!("CARGO_BIN_EXE_isos"))
    .args(args)
    .env("ISOS_HOME", home)
    .env("XDG_CONFIG_HOME", home.join("xdg-config"))
    .env_remove("ISOS_CONFIG")
    .env_remove("ISOS_MODE")
    .output()
    .expect("run IsOS CLI")
}

#[test]
fn failed_cli_apply_is_the_manager_scope_anchor_but_validate_is_not() {
  let temp = tempfile::tempdir().unwrap();
  let home = temp.path().join("home");
  let path = temp.path().join("invalid.json");
  std::fs::write(&path, br#"{"version":"2.0.0","cores":{}}"#).unwrap();
  let name = path.to_str().unwrap();
  assert!(!isos(&home, &["config", "validate", name]).status.success());
  let before = isos(&home, &["logs", "all", "--current"]);
  assert!(before.status.success());
  let before_lines: Vec<serde_json::Value> = before
    .stdout
    .split(|byte| *byte == b'\n')
    .filter(|line| !line.is_empty())
    .map(|line| serde_json::from_slice(line).unwrap())
    .collect();
  assert!(
    before_lines
      .iter()
      .any(|line| line["event_code"] == "command_failed")
  );
  assert!(
    !before_lines
      .iter()
      .any(|line| line["event_code"] == "deployment_preflight_failed")
  );
  assert!(!isos(&home, &["config", "apply", name]).status.success());
  let after = isos(&home, &["logs", "all", "--current", "--level", "error"]);
  assert!(
    after.status.success(),
    "{}",
    String::from_utf8_lossy(&after.stderr)
  );
  let after_lines: Vec<serde_json::Value> = after
    .stdout
    .split(|byte| *byte == b'\n')
    .filter(|line| !line.is_empty())
    .map(|line| serde_json::from_slice(line).unwrap())
    .collect();
  let failure = after_lines
    .iter()
    .find(|line| line["event_code"] == "deployment_preflight_failed")
    .unwrap();
  assert_eq!(failure["source"], "isos.runtime.isos.event@aether-v0.2.1");
  let status = isos(&home, &["status", "--json"]);
  assert!(status.status.success());
  let body: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
  assert_eq!(body["run_id_state"], "none");
}

#[test]
fn help_prints_quick_start_without_creating_home() {
  let temp = tempfile::tempdir().unwrap();
  let home = temp.path().join("isos-home");
  let output = isos(&home, &["--help"]);
  assert!(output.status.success());
  let stdout = String::from_utf8(output.stdout).unwrap();
  assert!(stdout.contains("workbench"));
  assert!(stdout.contains("dev"));
  assert!(stdout.contains("product"));
  assert!(!home.exists());
  assert!(isos(&home, &["--version"]).status.success());
  assert!(!home.exists());
  assert!(
    isos(&home, &["--generate-completion", "bash"])
      .status
      .success()
  );
  assert!(!home.exists());
}

#[test]
fn init_prints_next_steps_without_starting_manager() {
  let temp = tempfile::tempdir().unwrap();
  let home = temp.path().join("isos-home");
  let output = isos(&home, &["init", "--mode", "dev"]);
  assert!(output.status.success());
  let stdout = String::from_utf8(output.stdout).unwrap();
  assert!(stdout.contains("editable graph:"));
  assert!(stdout.contains("Next steps:"));
  assert!(stdout.contains("isos doctor"));
  assert!(stdout.contains("isos                               # open the workbench"));
  assert!(home.join("config/graph-pool/demo.nodes.json").exists());
  assert_eq!(
    std::fs::read_to_string(home.join("config/isos/launch.toml")).unwrap(),
    "[launch]\nmode = \"dev\"\n"
  );
  assert!(!home.join("data/isos/run/manager.sock").exists());
}

#[test]
fn product_init_mentions_configured_builds_without_starting_manager() {
  let temp = tempfile::tempdir().unwrap();
  let home = temp.path().join("isos-home");
  let output = isos(&home, &["init", "--mode", "product"]);
  assert!(output.status.success());
  let stdout = String::from_utf8(output.stdout).unwrap();
  assert!(stdout.contains("ISOS_AETHER_BIN_DIR"));
  assert!(stdout.contains("ISOSUI_DIST_DIR"));
  assert_eq!(
    std::fs::read_to_string(home.join("config/isos/launch.toml")).unwrap(),
    "[launch]\nmode = \"product\"\n"
  );
  assert!(stdout.contains("isos doctor"));
  assert!(!home.join("data/isos/run/manager.sock").exists());
}

#[test]
fn doctor_reports_local_readiness_as_json() {
  let temp = tempfile::tempdir().unwrap();
  let home = temp.path().join("isos-home");
  let machine = isos(&home, &["doctor"]);
  assert!(machine.status.success());
  let report: serde_json::Value = serde_json::from_slice(&machine.stdout).unwrap();
  assert_eq!(report["manager"], "stopped");
  assert!(report.get("graph").is_none());
  assert!(report.get("owners").is_none());
  assert_eq!(report["mode"], "product");
  assert_eq!(report["interfaces"]["product_version"], "0.2.1");
  assert_eq!(report["interfaces"]["target_graph_format"], "0.2.0");
}
