use crate::{
  app::config,
  app::server as http,
  app::isosui,
  runtime::logs as log_store,
  runtime::manager::{bind_daemon, daemon, manager_call},
  runtime::logs::manager_log,
  runtime::process::preflight,
  domain::graph::starter,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use std::{
  net::TcpStream,
  os::unix::{net::UnixStream, process::CommandExt},
  path::{Path, PathBuf},
  process::{Child, Command, Stdio},
  sync::atomic::{AtomicBool, Ordering},
  time::{Duration, Instant},
};

fn server_healthy() -> Result<bool> {
  let url = format!("{}/api/isos/v1/system/health", http::local_origin()?);
  let client = reqwest::blocking::Client::builder()
    .timeout(Duration::from_secs(2))
    .build()?;
  let reply = match client.get(url).send() {
    Ok(reply) => reply,
    Err(error) if error.is_connect() => return Ok(false),
    Err(error) => return Err(error.into()),
  };
  ensure!(
    reply.status().is_success(),
    "IsOS API port is occupied by an unhealthy server"
  );
  let body: Value = reply.json()?;
  ensure!(
    body["ok"] == true
      && body["data"]["service"] == "isos"
      && body["data"]["api_schema_version"] == "0.1.0",
    "IsOS API port is occupied by a different or incompatible server"
  );
  Ok(true)
}

/// Start only the control plane; UI clients and deployment workers are separate.
pub(crate) fn ensure_server(paths: &config::Paths, name: &str) -> Result<()> {
  if server_healthy()? {
    let origin = http::local_origin()?;
    let identity = manager_call(paths, name, "manager_identity").with_context(|| {
      format!(
        "IsOS HTTP is ready at {origin}, but the selected manager IPC at {} is unavailable; stop the server using this API port or select unused ISOS_HTTP_PORT and ISOSUI_PORT values",
        paths.socket().display()
      )
    })?;
    ensure!(
      identity["api_origin"] == origin,
      "IsOS HTTP at {origin} belongs to a different manager than the selected IPC at {} (manager API: {}); stop the conflicting server or select unused ISOS_HTTP_PORT and ISOSUI_PORT values",
      paths.socket().display(),
      identity["api_origin"]
    );
    if paths.mode()? == "dev" {
      let running = identity["executable"].as_str().unwrap_or_default();
      let selected = std::env::current_exe()?;
      ensure!(
        running == selected.to_string_lossy(),
        "IsOS at {origin} runs {running}; stop that manager before using the local build at {}",
        selected.display()
      );
      use sha2::{Digest, Sha256};
      let selected_sha256 = hex::encode(Sha256::digest(std::fs::read(&selected)?));
      ensure!(
        identity["executable_sha256"] == selected_sha256,
        "IsOS at {origin} runs an older local build; run ./isos stop, then ./isos web"
      );
    }
    return Ok(());
  }
  ensure!(
    UnixStream::connect(paths.socket()).is_err(),
    "IsOS manager IPC is active but HTTP is unavailable"
  );
  paths.prepare()?;
  let store = log_store::LogStore::new(paths, name)?;
  store.append_process(
    "ui-server",
    "info",
    "ui_server_spawning",
    "Starting local Web server",
  )?;
  let mut command = Command::new(std::env::current_exe()?);
  command
    .arg("--deployment")
    .arg(name)
    .arg("serve")
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
  unsafe {
    command.pre_exec(|| {
      if libc::setsid() < 0 {
        return Err(std::io::Error::last_os_error());
      }
      Ok(())
    });
  }
  let mut child = command.spawn().context("start IsOS server")?;
  let deadline =
    Instant::now() + Duration::from_millis(isosui::ui_settings()?.connection.startup_timeout_ms);
  loop {
    if server_healthy()? && UnixStream::connect(paths.socket()).is_ok() {
      store.append_process(
        "ui-server",
        "info",
        "ui_server_ready",
        "Local Web server ready",
      )?;
      return Ok(());
    }
    if let Some(status) = child.try_wait()? {
      let _ = store.append_process(
        "ui-server",
        "error",
        "ui_server_exited",
        &format!("Local Web server exited during startup: {status}"),
      );
      bail!("IsOS server exited during startup ({status}); inspect IsOS JSONL log");
    }
    if Instant::now() >= deadline {
      let _ = store.append_process(
        "ui-server",
        "error",
        "ui_server_timeout",
        "Local Web server did not become healthy",
      );
      bail!("IsOS server did not become healthy; inspect IsOS JSONL log");
    }
    std::thread::sleep(Duration::from_millis(100));
  }
}

pub(crate) fn open_tui() -> Result<()> {
  let tui_logs = match config::Paths::discover().and_then(|paths| {
    log_store::LogStore::new(
      &paths,
      &std::env::var("ISOS_DEPLOYMENT").unwrap_or_else(|_| "default".into()),
    )
  }) {
    Ok(logs) => Some(logs),
    Err(error) => {
      eprintln!("IsOS TUI log unavailable: {error:#}");
      None
    }
  };
  if let Some(logs) = &tui_logs {
    let _ = logs.append_event(
      "isos",
      "info",
      "tui_starting",
      "IsOS terminal workbench starting",
    );
  }
  let result = (|| -> Result<()> {
    let entry = std::env::var_os("ISOSUI_TUI_ENTRY")
      .map(PathBuf::from)
      .unwrap_or_else(|| {
        let binary = std::env::current_exe().unwrap_or_default();
        let installed = binary
          .parent()
          .and_then(Path::parent)
          .map(|prefix| prefix.join("share/isos/ui/tui/index.js"))
          .unwrap_or_default();
        if installed.is_file() {
          installed
        } else {
          Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/dist/tui/index.js")
        }
      });
    ensure!(
      entry.is_file(),
      "IsOS TUI is missing at {}; reinstall the product",
      entry.display()
    );
    let status = Command::new("node")
      .arg(&entry)
      .arg("--origin")
      .arg(http::local_origin()?)
      .status()
      .with_context(|| format!("start IsOS TUI at {}", entry.display()))?;
    ensure!(status.success(), "IsOS TUI exited with {status}");
    Ok(())
  })();
  if let Some(logs) = &tui_logs {
    let (level, code, message) = if result.is_ok() {
      ("info", "tui_stopped", "IsOS terminal workbench stopped")
    } else {
      ("error", "tui_failed", "IsOS terminal workbench failed")
    };
    let _ = logs.append_event("isos", level, code, message);
  }
  result
}

pub(crate) static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn interrupt_handler(_: libc::c_int) {
  INTERRUPTED.store(true, Ordering::SeqCst);
}

pub(crate) fn catch_interrupts() -> Result<()> {
  INTERRUPTED.store(false, Ordering::SeqCst);
  unsafe {
    let handler = interrupt_handler as *const () as libc::sighandler_t;
    ensure!(
      libc::signal(libc::SIGINT, handler) != libc::SIG_ERR,
      "install SIGINT handler"
    );
    ensure!(
      libc::signal(libc::SIGTERM, handler) != libc::SIG_ERR,
      "install SIGTERM handler"
    );
  }
  Ok(())
}

pub(crate) fn ui_source() -> Result<PathBuf> {
  let source = isosui::source_root()?;
  ensure!(
    source.join("node_modules").is_dir(),
    "IsosUI dependencies missing at {}; run npm ci there",
    source.display()
  );
  Ok(source)
}

pub(crate) struct Frontend(Child);

impl Frontend {
  fn spawn(source: &Path, logs: log_store::LogStore) -> Result<Self> {
    let port = http::ui_port()?;
    ensure!(
      TcpStream::connect(("127.0.0.1", port)).is_err(),
      "IsosUI port 127.0.0.1:{port} is already in use"
    );
    let mut command = Command::new("npm");
    command
      .arg("start")
      .current_dir(source)
      .env("ISOSUI_HOST", "127.0.0.1")
      .env("ISOSUI_PORT", port.to_string())
      .env("ISOSUI_API_ORIGIN", http::local_origin()?)
      .stdin(Stdio::null())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped());
    let parent_pid = std::process::id();
    unsafe {
      command.pre_exec(move || {
        if libc::setsid() < 0 {
          return Err(std::io::Error::last_os_error());
        }
        if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) < 0 {
          return Err(std::io::Error::last_os_error());
        }
        if libc::getppid() as u32 != parent_pid {
          return Err(std::io::Error::from_raw_os_error(libc::ESRCH));
        }
        Ok(())
      });
    }
    let mut child = command.spawn()?;
    let pid = child.id();
    if let Some(stdout) = child.stdout.take() {
      log_store::capture(
        stdout,
        logs.clone(),
        "ui".into(),
        "stdout",
        crate::runtime::manager::manager_session_id().into(),
        pid,
        None,
      );
    }
    if let Some(stderr) = child.stderr.take() {
      log_store::capture(
        stderr,
        logs,
        "ui".into(),
        "stderr",
        crate::runtime::manager::manager_session_id().into(),
        pid,
        None,
      );
    }
    Ok(Self(child))
  }

  fn ready(&mut self, port: u16) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
      if let Some(status) = self.0.try_wait()? {
        bail!("IsosUI npm server exited before ready: {status}");
      }
      if TcpStream::connect(("127.0.0.1", port)).is_ok() {
        return Ok(());
      }
      if INTERRUPTED.load(Ordering::SeqCst) {
        return Ok(());
      }
      std::thread::sleep(Duration::from_millis(50));
    }
    bail!("IsosUI npm server did not listen on 127.0.0.1:{port} within 30 seconds")
  }
}

impl Drop for Frontend {
  fn drop(&mut self) {
    let group = -(self.0.id() as i32);
    unsafe { libc::kill(group, libc::SIGTERM) };
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
      if self.0.try_wait().ok().flatten().is_some() {
        break;
      }
      std::thread::sleep(Duration::from_millis(20));
    }
    if unsafe { libc::kill(group, 0) } == 0 {
      unsafe { libc::kill(group, libc::SIGKILL) };
    }
    let _ = self.0.wait();
  }
}

pub(crate) fn open_browser() -> Result<()> {
  let locale = crate::app::isosui::ui_settings()?.locale;
  let url = http::ui_url_for(&locale)?;
  Command::new("xdg-open")
    .arg(&url)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .context("open IsosUI in browser")?;
  Ok(())
}

pub(crate) fn ensure_setup(paths: &config::Paths, name: &str) -> Result<()> {
  // Setup may run before the manager tracing subscriber exists.
  let setup_logs = match log_store::LogStore::new(paths, name) {
    Ok(logs) => Some(logs),
    Err(error) => {
      eprintln!("IsOS setup log unavailable: {error:#}");
      None
    }
  };
  if let Some(logs) = &setup_logs {
    let _ = logs.append_event("isos", "info", "setup_starting", "IsOS setup starting");
  }
  let result = ensure_setup_inner(paths, name, setup_logs.as_ref());
  if let Some(logs) = &setup_logs {
    let (level, code, message) = if result.is_ok() {
      ("info", "setup_completed", "IsOS setup completed")
    } else {
      ("error", "setup_failed", "IsOS setup failed")
    };
    let _ = logs.append_event("isos", level, code, message);
  }
  result
}

fn ensure_setup_inner(
  paths: &config::Paths,
  name: &str,
  logs: Option<&log_store::LogStore>,
) -> Result<()> {
  let old_server = Command::new("systemctl")
    .args(["--user", "is-active", "--quiet", "isos-server.service"])
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status();
  ensure!(
    !old_server.is_ok_and(|status| status.success()),
    "obsolete isos-server.service is active; stop it with `systemctl --user stop isos-server.service`, then use `isos serve`"
  );
  paths.prepare()?;
  if let Some(logs) = logs {
    let _ = logs.append_event("isos", "debug", "setup_paths_ready", "IsOS paths prepared");
  }
  if !paths.config.join("isos/launch.toml").exists() && std::env::var_os("ISOS_MODE").is_none() {
    paths.set_mode("product")?;
  }
  starter::init(paths, name)?;
  if let Some(logs) = logs {
    let _ = logs.append_event("isos", "debug", "setup_graph_ready", "IsOS graph prepared");
  }
  Ok(())
}

pub(crate) fn serve_foreground(
  paths: config::Paths,
  name: String,
  ui_live: bool,
  browser: bool,
  run_graph: bool,
) -> Result<()> {
  ensure_setup(&paths, &name)?;
  preflight::verify_active_isos(&paths)?;
  ensure!(
    !ui_live || paths.mode()? == "dev",
    "--ui-live is available only in dev mode"
  );
  if UnixStream::connect(paths.socket()).is_ok() {
    let identity = manager_call(&paths, &name, "manager_identity")?;
    bail!("IsOS manager is already running: {identity}");
  }
  let port = http::ui_port()?;
  ensure!(
    TcpStream::connect(("127.0.0.1", http::local_port()?)).is_err(),
    "IsOS API port belongs to another listener; stop it or configure a different port"
  );
  catch_interrupts()?;
  let logs = log_store::LogStore::new(&paths, &name)?;
  logs.begin_manager_session()?;
  if paths.mode()? == "dev" {
    logs.replace_dev_history(&paths, &name)?;
  }
  manager_log::install(logs.clone())?;
  logs.append_event(
    "isos",
    "info",
    "workbench_starting",
    "IsOS workbench starting",
  )?;
  let mut owner_services =
    crate::runtime::owners::owner_services::OwnerServices::start(&paths, &name);
  let main_log = logs.root().join("isos.jsonl");
  let mut frontend = None;
  let mut ui_issue = None;
  let static_ui = if ui_live {
    match isosui::bundle_for_serve(&paths) {
      Ok(_) => frontend = Some(Frontend::spawn(&ui_source()?, logs.clone())?),
      Err(error) => {
        ui_issue = Some(format!(
          "IsosUI bundle unavailable or incompatible: {error}"
        ));
      }
    }
    None
  } else {
    match isosui::bundle_for_serve(&paths) {
      Ok(directory) => {
        ensure!(
          TcpStream::connect(("127.0.0.1", port)).is_err(),
          "IsosUI port belongs to another listener; stop it or configure a different port"
        );
        let ui_logs = logs.clone();
        Some(std::thread::spawn(move || {
          http::serve_ui(directory, ui_logs)
        }))
      }
      Err(error) => {
        ui_issue = Some(format!(
          "IsosUI bundle unavailable or incompatible: {error}"
        ));
        None
      }
    }
  };
  let listener = bind_daemon(&paths)?;
  let api_paths = paths.clone();
  let manager_name = name.clone();
  let manager = std::thread::spawn(move || daemon(&api_paths, &manager_name, listener));
  let http_paths = paths.clone();
  let http_name = name.clone();
  let http_server = std::thread::spawn(move || http::serve(http_paths, http_name));
  let mut result = if let Some(frontend) = frontend.as_mut() {
    frontend.ready(port)
  } else if static_ui.is_some() {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
      if TcpStream::connect(("127.0.0.1", port)).is_ok() {
        break Ok(());
      }
      if static_ui.as_ref().is_some_and(|ui| ui.is_finished()) {
        break Err(anyhow::anyhow!("IsosUI static server exited before ready"));
      }
      if Instant::now() >= deadline {
        break Err(anyhow::anyhow!("IsosUI static server did not become ready"));
      }
      std::thread::sleep(Duration::from_millis(50));
    }
  } else {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
      if TcpStream::connect(("127.0.0.1", http::local_port()?)).is_ok()
        && UnixStream::connect(paths.socket()).is_ok()
      {
        break Ok(());
      }
      if http_server.is_finished() || manager.is_finished() {
        break Err(anyhow::anyhow!("IsOS API or manager exited before ready"));
      }
      if Instant::now() >= deadline {
        break Err(anyhow::anyhow!("IsOS API or manager did not become ready"));
      }
      std::thread::sleep(Duration::from_millis(50));
    }
  };
  if result.is_ok() {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
      if let Some(frontend) = frontend.as_mut()
        && let Some(status) = frontend.0.try_wait()?
      {
        result = Err(anyhow::anyhow!(
          "IsosUI npm server exited before ready: {status}"
        ));
        break;
      }
      if static_ui.as_ref().is_some_and(|ui| ui.is_finished()) {
        result = Err(anyhow::anyhow!("IsosUI static server exited before ready"));
        break;
      }
      if TcpStream::connect(("127.0.0.1", http::local_port()?)).is_ok()
        && UnixStream::connect(paths.socket()).is_ok()
      {
        break;
      }
      if http_server.is_finished() || manager.is_finished() {
        result = Err(anyhow::anyhow!("IsOS API or manager exited before ready"));
        break;
      }
      if Instant::now() >= deadline {
        result = Err(anyhow::anyhow!("IsOS API or manager did not become ready"));
        break;
      }
      std::thread::sleep(Duration::from_millis(50));
    }
  }
  if result.is_ok() && !INTERRUPTED.load(Ordering::SeqCst) {
    let _ = logs.append_event("isos", "info", "workbench_ready", "IsOS workbench ready");
    if static_ui.is_some() || frontend.is_some() {
      println!("{}", http::ui_url()?);
    } else if let Some(issue) = &ui_issue {
      eprintln!("{issue}; IsOS API and manager are available");
    }
    println!("IsOS API: {}", http::local_origin()?);
    println!("Unified log: {}", main_log.display());
    println!("IsOS workbench is running; press Ctrl+C to stop it.");
    if browser
      && (static_ui.is_some() || frontend.is_some())
      && let Err(error) = open_browser()
    {
      eprintln!(
        "Could not open browser: {error}. Open {} manually.",
        http::ui_url()?
      );
    }
    if run_graph {
      match manager_call(&paths, &name, "deployment_start") {
        Ok(started) => {
          if let Err(error) = print_run_startup(&paths, &name, &started) {
            eprintln!("Could not display graph status: {error}");
          }
        }
        Err(error) => {
          let _ = logs.append_event(
            "isos",
            "error",
            "deployment_start_failed",
            "IsOS deployment did not start",
          );
          eprintln!("Graph did not start: {error}. Use the workbench to finish setup.")
        }
      }
    }
    while !INTERRUPTED.load(Ordering::SeqCst) {
      owner_services.poll();
      if let Some(frontend) = frontend.as_mut()
        && let Some(status) = frontend.0.try_wait()?
      {
        result = Err(anyhow::anyhow!("IsosUI npm server exited: {status}"));
        break;
      }
      if static_ui.as_ref().is_some_and(|ui| ui.is_finished()) {
        result = Err(anyhow::anyhow!("IsosUI static server exited"));
        break;
      }
      if http_server.is_finished() || manager.is_finished() {
        break;
      }
      std::thread::sleep(Duration::from_millis(100));
    }
  }
  INTERRUPTED.store(true, Ordering::SeqCst);
  drop(frontend);
  if let Some(ui) = static_ui {
    ui.join()
      .map_err(|_| anyhow::anyhow!("IsosUI server thread panicked"))??;
  }
  let http_result = http_server
    .join()
    .map_err(|_| anyhow::anyhow!("IsOS HTTP thread panicked"))?;
  let manager_result = manager
    .join()
    .map_err(|_| anyhow::anyhow!("IsOS manager thread panicked"))?;
  manager_result?;
  http_result?;
  if let Err(error) = &result {
    tracing::error!(
      event_code = "workbench_failed",
      "IsOS workbench failed: {error:#}"
    );
  }
  let _ = logs.append_event(
    "isos",
    "info",
    "workbench_stopped",
    "IsOS workbench stopped",
  );
  result
}

pub(crate) fn print_run_startup(paths: &config::Paths, name: &str, result: &Value) -> Result<()> {
  println!("{}", serde_json::to_string_pretty(result)?);
  println!("IsosUI: {}", http::ui_url()?);
  println!("IsOS API: {}", http::local_origin()?);
  println!(
    "Active revision: {}",
    result["active_revision"]
      .as_str()
      .context("start result lacks active revision")?
  );
  println!(
    "Unified log: {}",
    paths
      .data
      .join("log/deployments")
      .join(name)
      .join("isos.jsonl")
      .display()
  );
  Ok(())
}

pub(crate) fn wait_for_interrupt(paths: &config::Paths, name: &str, run_id: &str) -> Result<()> {
  println!("Deployment is running; press Ctrl+C to stop it.");
  loop {
    if INTERRUPTED.load(Ordering::SeqCst) {
      let result = match manager_call(paths, name, "deployment_stop") {
        Ok(result) => result,
        Err(_error)
          if crate::runtime::owners::dependent_stop::recover(paths, name, run_id).outcome
            == "confirmed" =>
        {
          return Ok(());
        }
        Err(error) => return Err(error),
      };
      println!("{}", serde_json::to_string_pretty(&result)?);
      if result["terminal"]["outcome"] == "unknown" {
        anyhow::bail!("dependent stop unconfirmed");
      }
      return Ok(());
    }
    std::thread::sleep(Duration::from_millis(200));
    let status = match manager_call(paths, name, "deployment_status") {
      Ok(status) => status,
      Err(_error)
        if crate::runtime::owners::dependent_stop::recover(paths, name, run_id).outcome
          == "confirmed" =>
      {
        return Ok(());
      }
      Err(error) => return Err(error),
    };
    if status["terminal"]["outcome"] == "unknown" {
      anyhow::bail!("dependent stop unconfirmed");
    }
    if status["state"] == "stopped" {
      return Ok(());
    }
  }
}
