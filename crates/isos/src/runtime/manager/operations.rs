use super::*;

pub(crate) fn api_call(
  paths: &config::Paths,
  service_manager: &mut services::ServiceManager,
  command: WireCommand,
) -> Result<Value> {
  let name = config::deployment_name(command.deployment().map(str::to_owned));
  if command.op().starts_with("deployment_") {
    process::validate_deployment_name(&name)?;
  }
  match command {
    WireCommand::ManagerIdentity => manager_identity(paths),
    WireCommand::ManagerShutdown => {
      tracing::info!(
        event_code = "manager_shutdown",
        "IsOS manager shutdown requested"
      );
      let services = service_manager.stop_all();
      INTERRUPTED.store(true, Ordering::SeqCst);
      Ok(json!({"stopped":[],"terminal":[],"services":services,"manager":"stopping"}))
    }
    WireCommand::ServicesList => Ok(json!({"services":service_manager.list()?})),
    WireCommand::ServicesValidate => Ok(json!({"services":service_manager.validate()?})),
    WireCommand::ServicesStatus => Ok(json!({"services":service_manager.status()?})),
    WireCommand::ServicesStart(value) => {
      Ok(json!({"started":service_manager.start(&value.service_id)?}))
    }
    WireCommand::ServicesStop(value) => {
      Ok(json!({"stopped":service_manager.stop(&value.service_id)?}))
    }
    WireCommand::ServicesRestart(value) => {
      Ok(json!({"started":service_manager.restart(&value.service_id)?}))
    }
    WireCommand::ServicesLogs(value) => service_manager.logs(
      &value.service_id,
      value.cursor.as_deref(),
      value.limit.unwrap_or(100),
    ),
    WireCommand::DeploymentActivePackages(_) => Ok(json!({"packages":[]})),
    WireCommand::DeploymentRestart(value) => {
      let source = fs::read_to_string(paths.node_graph_path(&name))?;
      ensure!(
        owner_deployment::is_current(&source),
        "unsupported graph; Apply a three-core 0.2.0 graph"
      );
      owner_deployment::run(paths, &name, value.revision.as_deref(), true)
    }
    WireCommand::DeploymentStart(value) => {
      let source = fs::read_to_string(paths.node_graph_path(&name))?;
      ensure!(
        owner_deployment::is_current(&source),
        "unsupported graph; Apply a three-core 0.2.0 graph"
      );
      owner_deployment::run(paths, &name, value.revision.as_deref(), false)
    }
    WireCommand::DeploymentStop(value) => {
      if let Some(expected) = value.revision.as_deref() {
        ensure!(
          starter::revision(&paths.node_graph_path(&name))? == expected,
          "saved graph revision changed; reload before stop"
        );
      }
      owner_deployment::stop(paths, &name)
    }
    WireCommand::DeploymentStatus(_) => owner_deployment::status(paths, &name),
  }
}
