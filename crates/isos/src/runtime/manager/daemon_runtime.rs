use super::*;

pub(crate) fn daemon(
  paths: &config::Paths,
  deployment: &str,
  listener: UnixListener,
) -> Result<()> {
  // A manager session boundary is explicit; one-shot CLI reads must not
  // silently rotate the pre-run scope between commands.
  log_store::LogStore::new(paths, deployment)?.begin_manager_session()?;
  let mut service_manager = services::ServiceManager::new(paths, deployment)?;
  let mut owner_services =
    crate::runtime::owners::owner_services::OwnerServices::start(paths, deployment);
  println!("IsOS API listening on {}", paths.socket().display());
  while !INTERRUPTED.load(Ordering::SeqCst) {
    owner_services.poll();
    if let Err(error) = service_manager.poll() {
      tracing::error!(
        event_code = "service_supervisor_failed",
        "service supervisor: {error:#}"
      );
      eprintln!("service supervisor: {error:#}");
    }
    match listener.accept() {
      Ok((client, _)) => {
        if let Err(error) = handle_client(paths, &mut service_manager, client) {
          tracing::warn!(
            event_code = "manager_client_failed",
            "API client: {error:#}"
          );
          eprintln!("API client: {error}");
        }
      }
      Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
        std::thread::sleep(Duration::from_millis(50));
      }
      Err(error) => {
        tracing::error!(
          event_code = "manager_accept_failed",
          "API accept: {error:#}"
        );
        eprintln!("API accept: {error}");
      }
    }
  }
  service_manager.stop_all();
  tracing::info!(event_code = "manager_stopped", "IsOS manager stopped");
  fs::remove_file(paths.socket())?;
  Ok(())
}
