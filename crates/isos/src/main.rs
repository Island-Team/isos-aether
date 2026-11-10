mod app;
mod domain;
mod interfaces;
mod runtime;

use clap::Parser;

fn main() {
  // Clap exits while rendering help/version/completions. Keep those read-only
  // commands from creating a portable home just to record an invocation.
  let informational = std::env::args_os().skip(1).any(|arg| {
    let arg = arg.to_string_lossy();
    matches!(
      arg.as_ref(),
      "--help" | "-h" | "--version" | "-V" | "--generate-completion"
    ) || arg.starts_with("--generate-completion=")
  });
  let command_logs = (!informational)
    .then(|| app::config::Paths::discover().ok())
    .flatten()
    .and_then(|paths| {
      let deployment = app::cli::Cli::try_parse()
        .map(|cli| cli.deployment)
        .unwrap_or_else(|_| std::env::var("ISOS_DEPLOYMENT").unwrap_or_else(|_| "default".into()));
      match runtime::logs::LogStore::new(&paths, &deployment) {
        Ok(logs) => Some(logs),
        Err(error) => {
          eprintln!("isos log sink unavailable: {error:#}");
          None
        }
      }
    });
  if let Some(logs) = &command_logs
    && let Err(error) =
      logs.append_event("isos", "info", "command_starting", "IsOS command starting")
  {
    eprintln!("isos event sink failed: {error:#}");
  }
  if let Err(error) = app::entry::run() {
    if let Some(logs) = &command_logs {
      if let Err(log_error) =
        logs.append_diagnostic("error", "command_failed", &format!("{error:#}"))
      {
        eprintln!("isos log sink failed: {log_error:#}");
      }
      // Only copy known, bounded validation reasons into persistent logs. Other
      // errors may contain private paths, URLs, or credentials in their chain.
      let detail = error.to_string();
      let reason = detail
        .contains("invalid_graph: graph requires the isos core")
        .then_some("graph requires the isos core");
      let command = std::env::args().nth(1).unwrap_or_else(|| "unknown".into());
      if let Err(log_error) = logs.append_event_context(
        "isos",
        "error",
        "command_failed",
        "IsOS command failed",
        runtime::logs::LogContext {
          run_id: None,
          correlation_id: None,
          node_path: None,
          fields: serde_json::json!({
            "command": if command.len() <= 48 && command.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { command.as_str() } else { "unknown" },
            "error_code": if reason.is_some() { "invalid_graph" } else { "command_error" },
            "reason": reason.unwrap_or("see command stderr for details"),
          }),
        },
      ) { eprintln!("isos event sink failed: {log_error:#}"); }
    }
    eprintln!("isos: {error:#}");
    std::process::exit(1);
  }
  if let Some(logs) = &command_logs
    && let Err(error) = logs.append_event(
      "isos",
      "info",
      "command_completed",
      "IsOS command completed",
    )
  {
    eprintln!("isos event sink failed: {error:#}");
  }
}
