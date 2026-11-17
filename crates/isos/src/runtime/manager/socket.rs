use super::*;

fn client_disconnected(error: &anyhow::Error) -> bool {
  fn expected(kind: std::io::ErrorKind) -> bool {
    matches!(
      kind,
      std::io::ErrorKind::BrokenPipe
        | std::io::ErrorKind::ConnectionReset
        | std::io::ErrorKind::ConnectionAborted
    )
  }
  error.chain().any(|cause| {
    cause
      .downcast_ref::<std::io::Error>()
      .is_some_and(|error| expected(error.kind()))
      || cause
        .downcast_ref::<serde_json::Error>()
        .and_then(serde_json::Error::io_error_kind)
        .is_some_and(expected)
  })
}

pub(crate) fn handle_client(
  paths: &config::Paths,
  service_manager: &mut services::ServiceManager,
  stream: UnixStream,
) -> Result<()> {
  let accepted_at = Instant::now();
  let accept_monotonic_ms = monotonic_millis();
  stream.set_read_timeout(Some(Duration::from_secs(2)))?;
  stream.set_write_timeout(Some(Duration::from_secs(2)))?;
  let mut line = String::new();
  BufReader::new(stream.try_clone()?)
    .take(2_000_001)
    .read_line(&mut line)?;
  if line.is_empty() {
    return Ok(());
  }
  ensure!(line.len() <= 2_000_000, "API request exceeds 2 MB");
  let request = serde_json::from_str::<WireRequest>(&line)
    .map_err(anyhow::Error::from)
    .and_then(|request| {
      request.validate()?;
      Ok(request.command)
    });
  if let Ok(request) = &request
    && (matches!(request.op(), "deployment_start" | "deployment_restart")
      || request.op() == "deployment_status")
    && std::env::var_os("ISOS_PHASE_TIMING").is_some()
  {
    eprintln!(
      "q03_phase_timing operation={} phase=manager_decode phase_ms={} elapsed_ms={} monotonic_ms={} accept_monotonic_ms={accept_monotonic_ms}",
      request.op(),
      accepted_at.elapsed().as_millis(),
      accepted_at.elapsed().as_millis(),
      monotonic_millis()
    );
  }
  if let Ok(command) = &request
    && matches!(
      command,
      WireCommand::DeploymentStart(_) | WireCommand::DeploymentRestart(_)
    )
  {
    let name = config::deployment_name(command.deployment().map(str::to_owned));
    if let Err(error) = process::validate_deployment_name(&name) {
      return complete_client(paths, service_manager, stream, Err(error));
    }
    let mut startup_manager = match services::ServiceManager::new(paths, &name) {
      Ok(manager) => manager,
      Err(error) => return complete_client(paths, service_manager, stream, Err(error)),
    };
    if let Err(error) = owner_deployment::register_startup(&name) {
      return complete_client(paths, service_manager, stream, Err(error));
    }
    let paths = paths.clone();
    std::thread::spawn(move || {
      if let Err(error) = complete_client(&paths, &mut startup_manager, stream, request) {
        tracing::warn!(
          event_code = "manager_start_client_failed",
          "startup client: {error:#}"
        );
      }
      owner_deployment::finish_startup(&name);
    });
    return Ok(());
  }
  complete_client(paths, service_manager, stream, request)
}

fn complete_client(
  paths: &config::Paths,
  service_manager: &mut services::ServiceManager,
  mut stream: UnixStream,
  request: Result<WireCommand>,
) -> Result<()> {
  let failed_deployment = request
    .as_ref()
    .ok()
    .filter(|request| {
      matches!(
        request.op(),
        "deployment_start" | "deployment_restart" | "deployment_stop"
      )
    })
    .map(|request| config::deployment_name(request.deployment().map(str::to_owned)));
  let readonly_status = request
    .as_ref()
    .is_ok_and(|request| request.op() == "deployment_status");
  let clear_error = request
    .as_ref()
    .ok()
    .is_some_and(|request| matches!(request.op(), "deployment_start" | "deployment_restart"));
  let request_correlation = next_correlation_id();
  let outcome = request.and_then(|request| api_call(paths, service_manager, request));
  if let Err(error) = &outcome {
    tracing::warn!(
      event_code = "manager_request_failed",
      "manager request failed: {error:#}"
    );
  }
  if let Some(name) =
    failed_deployment.filter(|name| process::validate_deployment_name(name).is_ok())
  {
    let path = paths.runtime.join(format!("{name}.last-error"));
    match &outcome {
      Err(error) => {
        let message: String = error.to_string().chars().take(4096).collect();
        let _ = config::atomic_write(&path, message.as_bytes());
      }
      Ok(_) if clear_error => {
        let _ = fs::remove_file(path);
      }
      _ => {}
    }
  }
  let response = match outcome {
    Ok(data) => ApiResponse {
      schema_version: "0.1.0".into(),
      ok: true,
      data,
      error: None,
    },
    Err(error) => ApiResponse {
      schema_version: "0.1.0".into(),
      ok: false,
      data: error
        .chain()
        .find_map(|cause| {
          cause
            .downcast_ref::<graph_contract::Diagnostic>()
            .map(|diagnostic| json!({"diagnostics":[diagnostic]}))
            .or_else(|| {
              cause
                .downcast_ref::<crate::domain::packages::PackageFailure>()
                .map(|failure| json!({"diagnostics":[failure.diagnostic()]}))
            })
            .or_else(|| {
              cause
                .downcast_ref::<crate::domain::graph::generated::Diagnostic>()
                .map(|diagnostic| json!({"diagnostics":[diagnostic]}))
            })
            .or_else(|| {
              cause
                .downcast_ref::<config::StopBudgetDiagnostic>()
                .map(|diagnostic| json!({"diagnostics":[diagnostic]}))
            })
            .or_else(|| {
              cause
                .downcast_ref::<owner_deployment::OwnerRunConflict>()
                .map(|conflict| json!({"diagnostics":[{
                  "path":format!("/owners/{}", conflict.core),
                  "code":"run_conflict",
                  "message":conflict.to_string(),
                  "run_id":conflict.run_id.as_str().filter(|value| value.len() <= 128 && value.is_ascii()),
                  "revision":conflict.revision.as_str().filter(|value| value.len() <= 128 && value.is_ascii())
                }]}))
            })
        })
        .unwrap_or(Value::Null),
      error: Some(WireError {
        code: if error.to_string().starts_with("unsupported_schema_version:") {
          "unsupported_schema_version".to_owned()
        } else {
          "manager_operation_failed".to_owned()
        },
        message: error.to_string(),
        correlation_id: request_correlation,
      }),
    },
  };
  let mut response = response;
  if let Some(error) = response.error.as_mut()
    && let Some(code) = response.data["diagnostics"][0]["code"].as_str()
  {
    error.code = code.to_owned();
  }
  if let Err(error) = serde_json::to_writer(&mut stream, &response) {
    let error = anyhow::Error::new(error);
    if readonly_status && client_disconnected(&error) {
      return Ok(());
    }
    return Err(error);
  }
  if let Err(error) = stream.write_all(b"\n") {
    let error = anyhow::Error::new(error);
    if readonly_status && client_disconnected(&error) {
      return Ok(());
    }
    return Err(error);
  }
  Ok(())
}

pub(crate) fn bind_daemon(paths: &config::Paths) -> Result<UnixListener> {
  paths.prepare()?;
  let path = paths.socket();
  if path.exists() {
    ensure!(
      UnixStream::connect(&path).is_err(),
      "manager socket is already active"
    );
    fs::remove_file(&path)?;
  }
  let listener = UnixListener::bind(&path)?;
  fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
  listener.set_nonblocking(true)?;
  Ok(listener)
}

#[cfg(test)]
mod disconnect_tests {
  use super::*;

  #[test]
  fn expected_peer_close_does_not_hide_other_client_errors() {
    let closed = anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
    assert!(client_disconnected(&closed));
    let reset = anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::ConnectionReset));
    assert!(client_disconnected(&reset));
    let invalid = anyhow::anyhow!("invalid request");
    assert!(!client_disconnected(&invalid));
  }
}
