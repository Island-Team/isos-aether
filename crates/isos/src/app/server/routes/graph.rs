//! Graph API handlers.
use super::*;

fn graph_slots() -> &'static Arc<Semaphore> {
  static SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
  SLOTS.get_or_init(|| Arc::new(Semaphore::new(2)))
}

pub(super) async fn graph_work<T, F>(work: F) -> std::result::Result<T, (StatusCode, Json<Value>)>
where
  T: Send + 'static,
  F: FnOnce() -> std::result::Result<T, (StatusCode, Json<Value>)> + Send + 'static,
{
  let permit = graph_slots()
    .clone()
    .acquire_owned()
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  tokio::task::spawn_blocking(move || {
    let _permit = permit;
    work()
  })
  .await
  .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?
}

pub(crate) async fn node_graph(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  process::validate_deployment_name(&app.deployment)
    .map_err(|error| internal_err(StatusCode::BAD_REQUEST, &error))?;
  let pool_name = app.paths.bound_graph_name(&app.deployment);
  let bytes = match fs::read(app.paths.node_graph_path(&app.deployment)) {
    Ok(bytes) => bytes,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
      return ok(json!({
          "revision":"",
          "graph":crate::domain::graph::starter::draft(&app.paths, &app.deployment)
              .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?,
          "read_only":false,"diagnostics":[],"pool_name":pool_name,"missing_file":true
      }));
    }
    Err(error) => return Err(internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error)),
  };
  let saved = graph_contract::parse(&bytes)
    .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?;
  if let Some(diagnostic) = graph_contract::version_diagnostic(&saved.value) {
    return Err(diagnostic_failure(
      StatusCode::UNPROCESSABLE_ENTITY,
      "saved graph version is unsupported",
      json!(diagnostic),
    ));
  }
  let catalog = selected_catalog_read(app.paths.clone(), saved.value.clone())
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  let diagnostics: Vec<_> = graph_contract::admission_diagnostic(&saved.value, Some(&catalog))
    .into_iter()
    .collect();
  ok(
    json!({"revision":saved.revision,"graph":saved.value,"pool_name":pool_name,"missing_file":false,
    "read_only":!diagnostics.is_empty(),"diagnostics":diagnostics}),
  )
}

pub(crate) async fn new_node_graph(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  process::validate_deployment_name(&app.deployment)
    .map_err(|error| internal_err(StatusCode::BAD_REQUEST, &error))?;
  let revision = match fs::read(app.paths.node_graph_path(&app.deployment)) {
    Ok(bytes) => graph_contract::revision(&bytes),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
    Err(error) => return Err(internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error)),
  };
  ok(json!({
    "revision":revision,
    "graph":crate::domain::graph::starter::draft(&app.paths, &app.deployment)
      .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?,
    "read_only":false,
    "diagnostics":[]
  }))
}

#[cfg(test)]
mod new_graph_tests {
  use super::*;

  #[tokio::test]
  async fn replacement_draft_contains_all_three_cores_and_registered_chain() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: crate::app::config::Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    let result = new_node_graph(State(app), HeaderMap::new())
      .await
      .unwrap()
      .0;
    let graph = &result["data"]["graph"];
    assert_eq!(result["data"]["revision"], "");
    assert_eq!(graph["cores"].as_object().unwrap().len(), 3);
    assert_eq!(
      graph["cores"]["ei"]["layers"]["io"]["groups"]["input"]["nodes"]["trigger"]["node_type"],
      "ei.io.input.trigger@v0.2"
    );
    assert_eq!(
      graph["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm"]["node_type"],
      "eos.hardware.mock.arm@v0.2"
    );
    let observer =
      &graph["cores"]["eos"]["layers"]["hardware"]["groups"]["mock"]["nodes"]["arm_observer"];
    assert_eq!(observer["node_type"], "eos.hardware.mock.arm_observer@v0.2");
    assert!(observer.get("inputs").is_none());
    assert_eq!(
      graph["cores"]["eos"]["layers"]["dispatcher"]["groups"]["arm"]["nodes"]["monitor"]["inputs"]
        ["observation"]["node"],
      "eos/hardware/mock/arm_observer"
    );
  }
}

pub(crate) fn apply_error(error: graph_apply::ApplyError) -> (StatusCode, Json<Value>) {
  match error {
    graph_apply::ApplyError::NotFound => err(StatusCode::NOT_FOUND, "graph pool file is missing"),
    graph_apply::ApplyError::Conflict(_) => err(StatusCode::CONFLICT, "graph revision changed"),
    graph_apply::ApplyError::Diagnostic(diagnostic) => diagnostic_failure(
      StatusCode::UNPROCESSABLE_ENTITY,
      "graph Apply rejected",
      json!(graph_apply::public_diagnostic(&diagnostic)),
    ),
    graph_apply::ApplyError::Generated(diagnostic) => {
      let status = if diagnostic.code == "generated_commit_uncertain" {
        StatusCode::INTERNAL_SERVER_ERROR
      } else {
        StatusCode::UNPROCESSABLE_ENTITY
      };
      diagnostic_failure(
        status,
        "generated configuration rejected",
        json!(diagnostic),
      )
    }
    graph_apply::ApplyError::Filesystem(error) => {
      internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error)
    }
    graph_apply::ApplyError::OutcomeUncertain => diagnostic_failure(
      StatusCode::INTERNAL_SERVER_ERROR,
      "generated configuration publication is uncertain",
      json!(crate::domain::graph::generated_v2::commit_uncertain()),
    ),
  }
}

#[derive(Deserialize)]
pub(crate) struct NodeGraphEdit {
  #[serde(default)]
  pub(crate) schema_version: Value,
  #[serde(default)]
  pub(crate) graph: Value,
  #[serde(default)]
  pub(crate) revision: Option<String>,
  #[serde(default)]
  pub(crate) pool_name: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ConnectionPreview {
  pub(crate) schema_version: String,
  pub(crate) graph: Value,
  pub(crate) source: String,
  pub(crate) output: String,
  pub(crate) target: String,
  pub(crate) input: String,
}

pub(crate) async fn validate_connection(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(preview): Json<ConnectionPreview>,
) -> ApiResult {
  authorized(&app, &headers)?;
  if preview.schema_version != "0.1.0" {
    return Err(fail(
      StatusCode::BAD_REQUEST,
      "unsupported_schema_version",
      "API schema_version must be 0.1.0",
      Value::Null,
    ));
  }
  graph_work(move || {
    let catalog = process::catalog_for_option_graph(&app.paths, &preview.graph)
      .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
    let edges = match crate::domain::graph::edge_admission::admit_partial(&preview.graph, &catalog) {
      Ok(edges) => edges,
      Err(diagnostic) => return ok(json!({"allowed":false,"diagnostic":diagnostic})),
    };
    let edge = edges.into_iter().find(|edge| edge.source == preview.source
      && edge.output == preview.output && edge.target == preview.target && edge.input == preview.input);
    match edge {
      Some(edge) => ok(json!({"allowed":true,"edge":edge})),
      None => ok(json!({"allowed":false,"diagnostic":{"path":"/inputs","code":"edge_missing","message":"proposed connection is absent from the draft"}})),
    }
  }).await
}

pub(crate) async fn validate_node_graph(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(edit): Json<NodeGraphEdit>,
) -> ApiResult {
  let result = validate_node_graph_inner(State(app.clone()), headers, Json(edit)).await;
  record_graph_failure(&app, &result, false);
  result
}

pub(crate) async fn validate_node_graph_inner(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(edit): Json<NodeGraphEdit>,
) -> ApiResult {
  authorized(&app, &headers)?;
  if edit.schema_version != "0.1.0" {
    return Err(fail(
      StatusCode::BAD_REQUEST,
      "unsupported_schema_version",
      "API schema_version must be 0.1.0",
      Value::Null,
    ));
  }
  graph_work(move || validate_node_graph_sync(&app, edit)).await
}

fn validate_node_graph_sync(app: &App, edit: NodeGraphEdit) -> ApiResult {
  if edit.graph["version"] != "0.2.0" {
    return ok(json!({"valid":false,"diagnostics":[
      graph_contract::version_diagnostic(&edit.graph).expect("unsupported graph version")]}));
  }
  let bytes = serde_json::to_vec_pretty(&edit.graph)
    .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?;
  let proposal = graph_apply::Proposal {
    revision: graph_contract::revision(&bytes),
    graph: edit.graph,
    bytes,
  };
  match graph_apply::prepare(&app.paths, &app.deployment, &proposal) {
    Ok(prepared) => ok(json!({"valid":true,"diagnostics":[],"edges":prepared.edges})),
    Err(diagnostic) => ok(json!({"valid":false,"diagnostics":[
      graph_apply::public_diagnostic(&diagnostic)]})),
  }
}

pub(crate) async fn apply_node_graph(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(edit): Json<NodeGraphEdit>,
) -> ApiResult {
  let result = apply_node_graph_inner(State(app.clone()), headers, Json(edit)).await;
  record_graph_failure(&app, &result, true);
  result
}

pub(crate) async fn apply_node_graph_inner(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  Json(edit): Json<NodeGraphEdit>,
) -> ApiResult {
  authorized(&app, &headers)?;
  process::validate_deployment_name(&app.deployment)
    .map_err(|error| internal_err(StatusCode::BAD_REQUEST, &error))?;
  if edit.schema_version != "0.1.0" {
    return Err(fail(
      StatusCode::BAD_REQUEST,
      "unsupported_schema_version",
      "API schema_version must be 0.1.0",
      Value::Null,
    ));
  }
  graph_work(move || apply_node_graph_sync(&app, edit)).await
}

fn apply_node_graph_sync(app: &App, edit: NodeGraphEdit) -> ApiResult {
  if let Some(pool_name) = edit.pool_name {
    let expected = edit
      .revision
      .as_deref()
      .ok_or_else(|| err(StatusCode::BAD_REQUEST, "Apply requires pool revision"))?;
    let proposal = graph_apply::apply_pool(&app.paths, &app.deployment, &pool_name, expected)
      .map_err(apply_error)?;
    record(app, "graph", "Applied graph pool file");
    return ok(json!({"revision":proposal.revision,"graph":proposal.graph,"pool_name":pool_name}));
  }
  let graph = edit.graph;
  if graph["version"] != "0.2.0" {
    let diagnostic = graph_contract::version_diagnostic(&graph).expect("unsupported graph version");
    return Err(diagnostic_failure(
      StatusCode::UNPROCESSABLE_ENTITY,
      "graph admission rejected",
      json!(diagnostic),
    ));
  }
  let expected = edit.revision.as_deref().ok_or_else(|| {
    err(
      StatusCode::BAD_REQUEST,
      "Apply requires an explicit revision, including an empty revision for a new graph",
    )
  })?;
  let proposal =
    graph_apply::apply_new(&app.paths, &app.deployment, graph, expected).map_err(apply_error)?;
  record(app, "graph", "Applied node graph revision");
  ok(json!({"revision":proposal.revision,"graph":proposal.graph}))
}

#[cfg(test)]
mod blocking_tests {
  use super::*;

  #[tokio::test(flavor = "current_thread")]
  async fn graph_work_does_not_stall_http_worker() {
    let work = tokio::spawn(graph_work(|| {
      std::thread::sleep(std::time::Duration::from_millis(150));
      ok(json!({"complete":true}))
    }));
    tokio::time::timeout(
      std::time::Duration::from_millis(50),
      tokio::time::sleep(std::time::Duration::from_millis(20)),
    )
    .await
    .expect("Tokio worker stalled on graph operation");
    assert!(work.await.unwrap().is_ok());
  }
}

#[cfg(test)]
mod pool_binding_tests {
  use super::*;

  #[test]
  fn missing_pool_file_and_stale_revision_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let app = App {
      paths: crate::app::config::Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    };
    app.paths.prepare().unwrap();
    let edit = |revision: &str| NodeGraphEdit {
      schema_version: json!("0.1.0"),
      graph: Value::Null,
      revision: Some(revision.into()),
      pool_name: Some("demo".into()),
    };
    let missing = apply_node_graph_sync(&app, edit(&"0".repeat(64))).unwrap_err();
    assert_eq!(missing.0, StatusCode::NOT_FOUND);
    crate::app::config::atomic_write(&app.paths.graph_pool_path("demo"), b"{}").unwrap();
    let stale = apply_node_graph_sync(&app, edit(&"0".repeat(64))).unwrap_err();
    assert_eq!(stale.0, StatusCode::CONFLICT);
  }
}
