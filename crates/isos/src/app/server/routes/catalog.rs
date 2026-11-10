//! Catalog API handlers.
use super::*;

pub(crate) async fn node_types(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  let paths = app.paths.clone();
  let catalog = bounded_catalog_read(move || process::catalog_at(&paths))
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  ok(catalog)
}

pub(crate) fn selected_catalog_diagnostic() -> graph_contract::Diagnostic {
  graph_contract::Diagnostic {
    path: "/cores".into(),
    code: "selected_catalog_unavailable",
    message: "Selected owner catalog is unavailable; repair the selected owner and reload".into(),
    context: std::collections::BTreeMap::new(),
  }
}

pub(crate) async fn selected_node_types(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
) -> ApiResult {
  authorized(&app, &headers)?;
  let (graph, graph_revision) = match fs::read(app.paths.node_graph_path(&app.deployment)) {
    Ok(bytes) => {
      let saved = graph_contract::parse(&bytes)
        .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?;
      (saved.value, saved.revision)
    }
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
      crate::domain::graph::starter::draft(&app.paths, &app.deployment)
        .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?,
      String::new(),
    ),
    Err(error) => return Err(internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error)),
  };
  if graph["version"] != "0.2.0" {
    let diagnostic = graph_contract::admission_diagnostic(&graph, None).unwrap_or_else(|| {
      graph_contract::Diagnostic {
        path: "/version".into(),
        code: "selected_catalog_version_unsupported",
        message: "Selected node catalog requires a 0.2.0 graph".into(),
        context: std::collections::BTreeMap::new(),
      }
    });
    let code = diagnostic.code;
    return Err(fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      code,
      "selected node catalog version is unsupported",
      json!({"graph_revision":graph_revision,"diagnostics":[diagnostic]}),
    ));
  }
  let catalog = selected_catalog_read(app.paths.clone(), graph.clone())
    .await
    .map_err(|error| {
      tracing::warn!("selected catalog unavailable: {error:#}");
      fail(
        StatusCode::SERVICE_UNAVAILABLE,
        "selected_catalog_unavailable",
        "selected owner catalog is unavailable",
        json!({"graph_revision":graph_revision,
                    "diagnostics":[selected_catalog_diagnostic()]}),
      )
    })?;
  if let Some(diagnostic) = graph_contract::admission_diagnostic(&graph, Some(&catalog)) {
    let code = diagnostic.code;
    return Err(fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      code,
      "saved graph is not editable",
      json!({"graph_revision":graph_revision,"diagnostics":[diagnostic]}),
    ));
  }
  ok(json!({"graph_revision":graph_revision,"catalog":catalog}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeOptionsRequest {
  #[serde(default)]
  schema_version: Value,
  graph: Value,
}

pub(crate) async fn node_options_for_graph(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(request): Json<NodeOptionsRequest>,
) -> ApiResult {
  authorized(&app, &headers)?;
  if request.schema_version != "0.1.0" {
    return Err(fail(
      StatusCode::BAD_REQUEST,
      "unsupported_schema_version",
      "schema_version must be 0.1.0",
      Value::Null,
    ));
  }
  let paths = app.paths.clone();
  let graph = request.graph;
  let mut options = bounded_catalog_read(move || node_options::for_graph(&paths, &graph))
    .await
    .map_err(|error| node_option_err(&error))?;
  options["graph_revision"] = Value::Null;
  options["read_only"] = json!(false);
  ok(options)
}

pub(crate) async fn node_options_current(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
) -> ApiResult {
  authorized(&app, &headers)?;
  let bytes = match fs::read(app.paths.node_graph_path(&app.deployment)) {
    Ok(bytes) => bytes,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
    Err(error) => return Err(internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error)),
  };
  let graph: Value = if bytes.is_empty() {
    crate::domain::graph::starter::draft(&app.paths, &app.deployment)
      .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?
  } else {
    serde_json::from_slice(&bytes)
      .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?
  };
  let paths = app.paths.clone();
  let option_graph = graph.clone();
  let mut options = bounded_catalog_read(move || node_options::for_graph(&paths, &option_graph))
    .await
    .map_err(|error| node_option_err(&error))?;
  options["graph_revision"] = json!(if bytes.is_empty() {
    String::new()
  } else {
    graph_contract::revision(&bytes)
  });
  // for_graph already admitted the selected owner catalog and this graph.
  options["read_only"] = json!(false);
  ok(options)
}

pub(crate) async fn core_readiness(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  let paths = app.paths.clone();
  let deployment = app.deployment.clone();
  let status = tokio::task::spawn_blocking(move || {
    crate::runtime::manager::manager_status_for_http(&paths, &deployment)
  })
  .await
  .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?
  .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  ok(readiness::report(
    &app.paths,
    &app.deployment,
    Some(&status),
  ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewLinkRequest {
  path: PathBuf,
  manifest: Option<PathBuf>,
}

pub(crate) async fn preview_link(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(request): Json<PreviewLinkRequest>,
) -> ApiResult {
  authorized(&app, &headers)?;
  if !request.path.is_absolute() || !request.path.is_dir() {
    return Err(err(
      StatusCode::BAD_REQUEST,
      "bundle path must be an absolute directory",
    ));
  }
  let manifest_path = request
    .manifest
    .unwrap_or_else(|| request.path.join("release.json"));
  if !manifest_path.is_absolute() {
    return Err(err(
      StatusCode::BAD_REQUEST,
      "manifest path must be absolute",
    ));
  }
  let manifest = packages::inspect_link_candidate(&request.path, &manifest_path)
    .map_err(|error| package_err(&error, "unknown/package", "unknown"))?;
  let mut issues = Vec::new();
  if crate::domain::packages::package_path(&app.paths, &manifest.id, &manifest.version)
    .is_ok_and(|path| path.exists())
  {
    issues.push("this package version is already linked or installed".to_owned());
  }
  ok(json!({"id":manifest.id,"version":manifest.version,"ready":issues.is_empty(),"issues":issues}))
}

pub(crate) async fn package_node_sets(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
) -> ApiResult {
  authorized(&app, &headers)?;
  let paths = app.paths.clone();
  ok(
    bounded_catalog_read(move || node_sets::list(&paths))
      .await
      .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?,
  )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeSetPreview {
  package: crate::domain::launch::PackageIdentity,
  set_id: String,
  graph: Value,
  #[serde(default)]
  values: std::collections::BTreeMap<String, String>,
}

pub(crate) async fn preview_node_set(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(request): Json<NodeSetPreview>,
) -> ApiResult {
  authorized(&app, &headers)?;
  let preview = node_sets::compose(
    &app.paths,
    &request.package.id,
    &request.package.version,
    &request.set_id,
    &request.graph,
    &request.values,
  )
  .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?;
  process::validate_value(&app.paths, &preview["graph"]).map_err(|error| admission_err(&error))?;
  record(&app, "graph", "Previewed package node set");
  ok(preview)
}
