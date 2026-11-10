//! System version route.

use super::*;

pub(crate) async fn version(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  let support = crate::domain::version::bundled()
    .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
  ok(
    json!({"api_schema_version":"0.1.0","isos_version":env!("CARGO_PKG_VERSION"),"version_support":support}),
  )
}

pub(crate) async fn health(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  let paths = app.paths.clone();
  let owners = tokio::task::spawn_blocking(move || {
    crate::runtime::owners::owner_services::health_report(&paths)
  })
  .await
  .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
  ok(
    json!({"service":"isos","api_schema_version":"0.1.0","state":"ready","owners":owners["owners"]}),
  )
}

pub(crate) async fn ui_config(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  let settings = crate::app::isosui::ui_settings()
    .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
  let origin = super::super::local_origin()
    .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
  ok(
    json!({"default_interface":settings.default_interface,"locale":settings.locale,"api_origin":origin,"poll_interval_ms":settings.connection.poll_interval_ms}),
  )
}
