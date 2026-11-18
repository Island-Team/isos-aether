//! Packages API handlers.
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageConfigQuery {
  id: String,
  version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageConfigEdit {
  schema_version: String,
  id: String,
  version: String,
  #[serde(default)]
  revision: Option<String>,
  #[serde(default)]
  toml: Option<String>,
  #[serde(default)]
  values: Option<toml::Value>,
}

fn config_source(edit: &PackageConfigEdit) -> Result<String> {
  anyhow::ensure!(
    edit.schema_version == "0.1.0",
    "unsupported API schema version"
  );
  match (&edit.toml, &edit.values) {
    (Some(source), None) => Ok(source.clone()),
    (None, Some(values)) => Ok(toml::to_string_pretty(values)?),
    _ => anyhow::bail!("provide exactly one of toml or values"),
  }
}

pub(crate) async fn package_config_get(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Query(query): Query<PackageConfigQuery>,
) -> ApiResult {
  authorized(&app, &headers)?;
  let saved =
    packages::package_config::show(&app.paths, &app.deployment, &query.id, &query.version)
      .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?;
  ok(json!(saved))
}

pub(crate) async fn package_config_put(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Query(query): Query<PackageConfigQuery>,
  Json(edit): Json<PackageConfigEdit>,
) -> ApiResult {
  authorized(&app, &headers)?;
  if query.id != edit.id || query.version != edit.version {
    return Err(err(StatusCode::BAD_REQUEST, "package identity mismatch"));
  }
  let source =
    config_source(&edit).map_err(|error| internal_err(StatusCode::BAD_REQUEST, &error))?;
  let revision = edit
    .revision
    .as_deref()
    .ok_or_else(|| err(StatusCode::BAD_REQUEST, "config revision required"))?;
  let saved = packages::package_config::save(
    &app.paths,
    &app.deployment,
    &edit.id,
    &edit.version,
    revision,
    &source,
  )
  .map_err(|error| internal_err(StatusCode::CONFLICT, &error))?;
  ok(json!(saved))
}

pub(crate) async fn package_config_validate(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(edit): Json<PackageConfigEdit>,
) -> ApiResult {
  authorized(&app, &headers)?;
  let source =
    config_source(&edit).map_err(|error| internal_err(StatusCode::BAD_REQUEST, &error))?;
  let saved = packages::package_config::show(&app.paths, &app.deployment, &edit.id, &edit.version)
    .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?;
  match packages::package_config::validate(&saved.descriptor, &source, &[]) {
    Ok(()) => ok(json!({"valid":true,"diagnostics":[]})),
    Err(error) => ok(
      json!({"valid":false,"diagnostics":[{"code":"package_config_invalid","message":error.to_string()}]}),
    ),
  }
}

pub(crate) async fn package_list(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  let installed = packages::listing(&app.paths)
    .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
  ok(json!({"installed":installed}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageRequest {
  id: String,
  version: String,
  approval: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageSourceRequest {
  id: String,
  version: String,
  path: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackagePreviewRequest {
  id: String,
  version: String,
}

pub(crate) async fn package_preview_download(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(request): Json<PackagePreviewRequest>,
) -> ApiResult {
  authorized(&app, &headers)?;
  let preview = tokio::task::spawn_blocking(move || {
    let release = packages::catalog_release(&request.id, &request.version)?;
    anyhow::ensure!(
      release.id == request.id && release.version == request.version,
      "catalog returned a different package identity"
    );
    packages::download_preview(&release)
  })
  .await
  .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?
  .map_err(|error| internal_err(StatusCode::BAD_REQUEST, &error))?;
  ok(json!({"preview":preview}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageLinkRequest {
  id: String,
  version: String,
  path: PathBuf,
  #[serde(default)]
  manifest: Option<PathBuf>,
}

pub(crate) async fn package_install(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(request): Json<PackageRequest>,
) -> ApiResult {
  authorized(&app, &headers)?;
  packages::package_path(&app.paths, &request.id, &request.version)
    .map_err(|error| internal_err(StatusCode::BAD_REQUEST, &error))?;
  let id = request.id.clone();
  let version = request.version.clone();
  let paths = app.paths.clone();
  let result = tokio::task::spawn_blocking(move || -> Result<PathBuf> {
    let release = packages::catalog_release(&request.id, &request.version)?;
    anyhow::ensure!(
      release.id == request.id && release.version == request.version,
      "catalog returned a different package identity"
    );
    packages::install_approved(&paths, &release, &request.approval)
  })
  .await
  .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
  let installed = match result {
    Ok(path) => path,
    Err(error) => {
      let code = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<packages::PackageFailure>())
        .map_or("package_invalid_manifest", |failure| failure.code);
      record_package(&app, "error", code, &id, &version, "approved_catalog", None);
      return Err(package_err(&error, &id, &version));
    }
  };
  record_package(
    &app,
    "info",
    "package_verified",
    &id,
    &version,
    "approved_catalog",
    Some(&installed),
  );
  ok(
    json!({"path":installed,"verified":true,"source_kind":"approved_catalog","installed":packages::listing(&app.paths).map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?}),
  )
}

pub(crate) async fn package_install_source(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(request): Json<PackageSourceRequest>,
) -> ApiResult {
  authorized(&app, &headers)?;
  if app
    .paths
    .mode()
    .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?
    != "dev"
  {
    return Err(err(
      StatusCode::FORBIDDEN,
      "source installation requires dev mode",
    ));
  }
  packages::package_path(&app.paths, &request.id, &request.version)
    .map_err(|error| internal_err(StatusCode::BAD_REQUEST, &error))?;
  if !request.path.is_absolute() {
    return Err(err(
      StatusCode::BAD_REQUEST,
      "source checkout path must be absolute",
    ));
  }
  let id = request.id.clone();
  let version = request.version.clone();
  let paths = app.paths.clone();
  let result = tokio::task::spawn_blocking(move || {
    packages::install_source(
      &paths,
      &request.path,
      Some(&request.id),
      Some(&request.version),
    )
  })
  .await
  .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
  let (_, installed) = match result {
    Ok(installed) => installed,
    Err(error) => {
      record_package(
        &app,
        "error",
        "package_source_build_failed",
        &id,
        &version,
        "local_source",
        None,
      );
      tracing::warn!(package_id = %id, package_version = %version, %error, "source package installation failed");
      return Err(fail(
        StatusCode::UNPROCESSABLE_ENTITY,
        "package_source_build_failed",
        "source package build or verification failed; check IsOS logs",
        json!({"id":id,"version":version}),
      ));
    }
  };
  record_package(
    &app,
    "info",
    "package_verified",
    &id,
    &version,
    "local_source",
    Some(&installed),
  );
  ok(
    json!({"path":installed,"verified":true,"source_kind":"local_source","installed":packages::listing(&app.paths).map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?}),
  )
}

pub(crate) async fn package_link(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(request): Json<PackageLinkRequest>,
) -> ApiResult {
  authorized(&app, &headers)?;
  packages::package_path(&app.paths, &request.id, &request.version)
    .map_err(|error| internal_err(StatusCode::BAD_REQUEST, &error))?;
  if !request.path.is_absolute() {
    return Err(err(
      StatusCode::BAD_REQUEST,
      "package link path must be absolute",
    ));
  }
  if request
    .manifest
    .as_ref()
    .is_some_and(|path| !path.is_absolute())
  {
    return Err(err(
      StatusCode::BAD_REQUEST,
      "package manifest path must be absolute",
    ));
  }
  let id = request.id.clone();
  let version = request.version.clone();
  let paths = app.paths.clone();
  let result = tokio::task::spawn_blocking(move || {
    packages::link_with_manifest(
      &paths,
      &request.id,
      &request.version,
      &request.path,
      request.manifest.as_deref(),
    )
  })
  .await
  .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
  let linked = match result {
    Ok(path) => path,
    Err(error) => {
      let code = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<packages::PackageFailure>())
        .map_or("package_invalid_manifest", |failure| failure.code);
      record_package(&app, "error", code, &id, &version, "local_link", None);
      return Err(package_err(&error, &id, &version));
    }
  };
  record_package(
    &app,
    "info",
    if linked.join("release.json").is_file() {
      "package_verified"
    } else {
      "package_linked"
    },
    &id,
    &version,
    "local_link",
    Some(&linked),
  );
  ok(
    json!({"path":linked,"verified":linked.join("release.json").is_file(),"source_kind":"local_link","installed":packages::listing(&app.paths).map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?}),
  )
}
