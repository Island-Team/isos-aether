use crate::{
  app::auth,
  domain::packages,
  app::cli::{
    AuthCommand, Cli, CommandLine, DemoCommand, DeploymentCommand, config_command, library,
    package_command, print_init_next_steps, services_command,
  },
  app::config,
  runtime::owners::core_setup,
  app::foreground::{
    catch_interrupts, ensure_server, ensure_setup, open_browser, open_tui, print_run_startup,
    serve_foreground, wait_for_interrupt,
  },
  app::server as http,
  runtime::logs as log_store,
  runtime::manager::{bind_daemon, daemon, manager_call, service_call},
  runtime::logs::manager_log,
  app::physical_stop,
  domain::graph::starter,
};
use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use serde_json::json;
use std::{os::unix::net::UnixStream, time::Instant};

pub(crate) fn run() -> Result<()> {
  let cli = Cli::parse();
  if let Some(shell) = cli.generate_completion {
    generate(shell, &mut Cli::command(), "isos", &mut std::io::stdout());
    return Ok(());
  }
  match cli.command.as_ref() {
    Some(CommandLine::Install { source }) => return crate::app::cli::install(source),
    Some(CommandLine::Update) => return crate::app::cli::update(),
    Some(CommandLine::Upgrade { version }) => return crate::app::cli::upgrade(version.as_deref()),
    Some(CommandLine::Stop { code_stdin }) => {
      return crate::app::emergency_stop::run(&cli.deployment, *code_stdin);
    }
    _ => {}
  }
  config::load_runtime_config()?;
  let paths = config::Paths::discover()?;
  let name = cli.deployment;
  let Some(command) = cli.command else {
    ensure_setup(&paths, &name)?;
    ensure_server(&paths, &name)?;
    if crate::app::isosui::ui_settings()?.default_interface == "tui" {
      open_tui()?;
    } else {
      open_browser()?;
      println!("{}", http::ui_url()?);
    }
    return Ok(());
  };
  match command {
    CommandLine::Web => {
      ensure_setup(&paths, &name)?;
      ensure_server(&paths, &name)?;
      open_browser()?;
      println!("{}", http::ui_url()?);
    }
    CommandLine::Tui => {
      ensure_setup(&paths, &name)?;
      ensure_server(&paths, &name)?;
      open_tui()?;
    }
    CommandLine::Install { .. } | CommandLine::Update | CommandLine::Upgrade { .. } => {
      unreachable!()
    }
    CommandLine::Init { mode } => {
      paths.prepare()?;
      if let Some(mode) = mode {
        paths.set_mode(&mode)?;
      } else if !paths.config.join("isos/launch.toml").exists() {
        paths.set_mode("product")?;
      }
      let graph_path = starter::init(&paths, &name)?;
      let refreshed = core_setup::refresh_after_init(&paths, &name)?;
      println!("editable graph: {}", graph_path.display());
      if let Some(revision) = refreshed {
        println!(
          "refreshed core launch inputs for graph {revision}; Run or Restart to use the configured build"
        );
        if let Err(error) = crate::app::cli::clear_saved_setup_error(&paths, &name) {
          eprintln!("core inputs refreshed, but could not clear prior setup error: {error}");
        }
        println!("Run `isos doctor` to confirm readiness.");
      } else {
        if packages::list(&paths)?.is_empty() {
          println!(
            "No package release is installed; install or link an exact release before selecting package-contributed nodes."
          );
        }
        print_init_next_steps(paths.mode()?);
      }
    }
    CommandLine::Open => {
      crate::app::foreground::ensure_server(&paths, &name)?;
      crate::app::foreground::open_browser()?;
    }
    CommandLine::Run { no_ui } => {
      let setup_started = Instant::now();
      ensure_setup(&paths, &name)?;
      crate::runtime::manager::emit_phase_timing(
        "start",
        "cli_setup_complete",
        setup_started.elapsed().as_millis() as u64,
        setup_started.elapsed().as_millis() as u64,
      );
      if UnixStream::connect(paths.socket()).is_err() {
        return serve_foreground(paths, name, false, !no_ui, true, no_ui);
      }
      anyhow::ensure!(
        !no_ui,
        "IsOS manager is already running; stop it before `isos run --no-ui`"
      );
      catch_interrupts()?;
      let started = manager_call(&paths, &name, "deployment_start")?;
      print_run_startup(&paths, &name, &started, !no_ui)?;
      wait_for_interrupt(
        &paths,
        &name,
        started["run_id"]
          .as_str()
          .context("start result lacks run ID")?,
      )?;
    }
    CommandLine::Serve { ui_live, no_ui } => {
      serve_foreground(paths, name, ui_live, false, false, no_ui)?;
    }
    CommandLine::Auth { command } => match command {
      AuthCommand::Login => auth::login(&paths)?,
      AuthCommand::Logout => auth::logout(&paths)?,
      AuthCommand::Whoami => auth::status(&paths)?,
    },
    CommandLine::Start => {
      catch_interrupts()?;
      let started = manager_call(&paths, &name, "deployment_start")?;
      print_run_startup(&paths, &name, &started, true)?;
      wait_for_interrupt(
        &paths,
        &name,
        started["run_id"]
          .as_str()
          .context("start result lacks run ID")?,
      )?;
    }
    CommandLine::Stop { .. } => unreachable!(),
    CommandLine::PhysicalStop { command } => {
      physical_stop::run(&paths, &name, command)?;
    }
    CommandLine::Deployment { command } => match command {
      DeploymentCommand::Stop => {
        let result = manager_call(&paths, &name, "deployment_stop")?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        if result["terminal"]["outcome"] == "unknown" {
          anyhow::bail!("dependent stop unconfirmed");
        }
      }
      DeploymentCommand::Status => println!(
        "{}",
        serde_json::to_string_pretty(&manager_call(&paths, &name, "deployment_status")?)?
      ),
    },
    CommandLine::Restart => {
      catch_interrupts()?;
      let started = manager_call(&paths, &name, "deployment_restart")?;
      print_run_startup(&paths, &name, &started, true)?;
      wait_for_interrupt(
        &paths,
        &name,
        started["run_id"]
          .as_str()
          .context("start result lacks run ID")?,
      )?;
    }
    CommandLine::Status { json: _ } => {
      let mut status = if UnixStream::connect(paths.socket()).is_ok() {
        manager_call(&paths, &name, "deployment_status")?
      } else {
        physical_stop::offline_status(&paths, &name)
      };
      if status.get("run_id_state").is_none() {
        let (run_id, run_id_state) = log_store::LogStore::new(&paths, &name)
          .map(|store| store.run_identity())
          .unwrap_or((None, "unavailable"));
        status["run_id"] = json!(run_id);
        status["run_id_state"] = json!(run_id_state);
      }
      status["manager"] = if UnixStream::connect(paths.socket()).is_ok() {
        manager_call(&paths, &name, "manager_identity")?
      } else {
        json!({"state":"stopped"})
      };
      status["services"] = if UnixStream::connect(paths.socket()).is_ok() {
        service_call(&paths, &name, "services_status", None, None)?["services"].clone()
      } else {
        json!([])
      };
      println!("{}", serde_json::to_string_pretty(&status)?);
    }
    CommandLine::Demo { command } => {
      let origin = http::local_origin()?;
      let identity = manager_call(&paths, &name, "manager_identity")?;
      anyhow::ensure!(
        identity["api_origin"] == origin,
        "selected local API does not match this IsOS manager"
      );
      let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
      let activate = matches!(&command, DemoCommand::Activate { .. });
      let (route, body) = match command {
        DemoCommand::Monitor => ("nodes/monitor", None),
        DemoCommand::Activate { event_id } | DemoCommand::Result { event_id } => {
          let status = manager_call(&paths, &name, "deployment_status")?;
          let run_id = status["run_id"].as_str().context("no active demo run")?;
          let revision = status["active_revision"]
            .as_str()
            .context("active demo revision unavailable")?;
          if activate {
            (
              "nodes/events",
              Some(
                json!({"schema_version":"0.2.0","source_node":"isos/ui/button/trigger","output":"activation","data_type":"button_activation@v0.2","run_id":run_id,"active_revision":revision,"payload":{"event_id":event_id,"kind":"activate"}}),
              ),
            )
          } else {
            (
              "nodes/events/result",
              Some(
                json!({"schema_version":"0.2.0","source_node":"isos/ui/button/trigger","run_id":run_id,"active_revision":revision,"event_id":event_id}),
              ),
            )
          }
        }
      };
      let url = format!("{origin}/api/isos/v1/{route}");
      let request = if let Some(body) = body {
        client.post(url).json(&body)
      } else {
        client.get(url)
      };
      let response = request.send()?;
      let status = response.status();
      let body: serde_json::Value = response.json()?;
      println!("{}", serde_json::to_string_pretty(&body)?);
      anyhow::ensure!(status.is_success(), "demo API returned HTTP {status}");
    }
    CommandLine::Logs {
      source,
      lines,
      level,
      current,
      format,
      follow,
    } => {
      let store = log_store::LogStore::new(&paths, &name)?;
      if current {
        anyhow::ensure!(!follow, "--follow cannot be combined with --current");
        let status = manager_call(&paths, &name, "deployment_status").ok();
        let active_revision = status
          .as_ref()
          .and_then(|status| status["active_revision"].as_str().map(str::to_owned));
        let run_id = status.as_ref().and_then(|status| status["run_id"].as_str());
        let page = store
          .read_local_current(
            &paths,
            run_id,
            log_store::CurrentQuery {
              source: &source,
              level: &level,
              node_path: None,
              correlation_id: None,
              cursor: None,
              limit: lines as usize,
              active_revision: active_revision.as_deref(),
            },
          )
          .map_err(|error| anyhow::anyhow!("{}: {}", error.code, error.message))?;
        if let Some(anchor) = page["anchor"]["event"].as_object() {
          println!("{}", serde_json::to_string(anchor)?);
        }
        for entry in page["lines"].as_array().into_iter().flatten() {
          println!("{}", serde_json::to_string(entry)?);
        }
        for diagnostic in page["diagnostics"].as_array().into_iter().flatten() {
          eprintln!("{}", serde_json::to_string(diagnostic)?);
        }
        return Ok(());
      }
      if format == "log" {
        let mut previous = Vec::new();
        loop {
          let current = store.tail_human(&source, lines as usize, &level)?;
          let overlap = (0..=previous.len().min(current.len()))
            .rev()
            .find(|count| previous[previous.len() - *count..] == current[..*count])
            .unwrap_or(0);
          for line in current
            .iter()
            .skip(if previous.is_empty() { 0 } else { overlap })
          {
            println!("{line}");
          }
          if !follow {
            break;
          }
          previous = current;
          std::thread::sleep(std::time::Duration::from_millis(250));
        }
      } else {
        anyhow::ensure!(
          !(follow && format == "json"),
          "--follow cannot be combined with --format json"
        );
        let mut previous = Vec::new();
        loop {
          let events = store.tail_filtered(&source, lines as usize, Some(&level))?;
          if format == "json" {
            println!("{}", serde_json::to_string_pretty(&events)?);
          } else {
            let current = events
              .iter()
              .map(log_store::jsonl_line)
              .collect::<Result<Vec<_>>>()?;
            let overlap = (0..=previous.len().min(current.len()))
              .rev()
              .find(|count| previous[previous.len() - *count..] == current[..*count])
              .unwrap_or(0);
            for line in current.iter().skip(overlap) {
              println!("{line}");
            }
            previous = current;
          }
          if !follow {
            break;
          }
          std::thread::sleep(std::time::Duration::from_millis(250));
        }
      }
    }
    CommandLine::Doctor => crate::app::cli::doctor(&paths, &name)?,
    CommandLine::Config { command } => config_command(&paths, &name, command)?,
    CommandLine::Packages { command } => package_command(&paths, &name, command)?,
    CommandLine::Skills { command } => library(&paths, &name, "skills", command)?,
    CommandLine::Services { command } => services_command(&paths, &name, command)?,
    CommandLine::Backends { command } => library(&paths, &name, "backends", command)?,
    CommandLine::Daemon => {
      catch_interrupts()?;
      manager_log::install(log_store::LogStore::new(&paths, &name)?)?;
      let listener = bind_daemon(&paths)?;
      daemon(&paths, &name, listener)?;
    }
  }
  Ok(())
}
