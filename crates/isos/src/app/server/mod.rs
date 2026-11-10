//! Loopback configuration API and verified static IsosUI server.
use crate::{
  app::config::Paths, runtime::owners::core_setup, domain::graph::graph_contract,
  domain::graph::graph_apply, runtime::logs::LogStore, domain::graph::node_options,
  domain::graph::node_sets, domain::packages, runtime::process, runtime::process::readiness,
};
#[cfg(test)]
use crate::app::config;
use anyhow::{Context, Result};
use axum::{
  Json, Router,
  body::Body,
  extract::{Path as UrlPath, State},
  extract::MatchedPath,
  http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header},
  middleware::{self, Next},
  response::{IntoResponse, Response},
  routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
  fs,
  net::SocketAddr,
  path::PathBuf,
  sync::{Arc, OnceLock},
  time::Instant,
};
use tokio::sync::Semaphore;
use tower_http::services::{ServeDir, ServeFile};

struct App {
  paths: Paths,
  deployment: String,
}

// Keep expensive verification and owner catalog calls off Tokio workers and
// bound simultaneous hashing when several workbench panels refresh together.
fn catalog_slots() -> &'static Arc<Semaphore> {
  static SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
  SLOTS.get_or_init(|| Arc::new(Semaphore::new(2)))
}

async fn selected_catalog_read(paths: Paths, graph: Value) -> Result<Value> {
  bounded_catalog_read(move || process::catalog_for_graph(&paths, &graph)).await
}

async fn bounded_catalog_read<F>(work: F) -> Result<Value>
where
  F: FnOnce() -> Result<Value> + Send + 'static,
{
  let slots = catalog_slots().clone();
  let permit = slots.acquire_owned().await?;
  tokio::task::spawn_blocking(move || {
    let _permit = permit;
    work()
  })
  .await?
}

mod transport;
pub(crate) use transport::{local_origin, local_port, ui_port, ui_url, ui_url_for};
use transport::{authorized, cors, preflight};
mod response;
use response::*;
fn package_err(error: &anyhow::Error, id: &str, version: &str) -> (StatusCode, Json<Value>) {
  let failure = error
    .chain()
    .find_map(|cause| cause.downcast_ref::<packages::PackageFailure>());
  let diagnostic = failure.map_or_else(|| json!({"code":"package_invalid_manifest","path":"",
        "message":format!("Reinstall or relink {id}@{version}"),"id":id,"version":version,"stage":"manifest"}),
        packages::PackageFailure::diagnostic);
  let status = if diagnostic["code"] == "package_version_exists" {
    StatusCode::CONFLICT
  } else {
    StatusCode::UNPROCESSABLE_ENTITY
  };
  let code = diagnostic["code"]
    .as_str()
    .unwrap_or("package_invalid_manifest")
    .to_owned();
  fail(
    status,
    code,
    format!("Reinstall or relink {id}@{version}"),
    json!({"diagnostics":[diagnostic]}),
  )
}
fn admission_err(error: &anyhow::Error) -> (StatusCode, Json<Value>) {
  if let Some(diagnostic) = error
    .chain()
    .find_map(|cause| cause.downcast_ref::<crate::runtime::owners::ei_state::Diagnostic>())
  {
    return fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      diagnostic.code,
      "EI test environment rejected",
      json!({"diagnostics":[diagnostic]}),
    );
  }
  if let Some(diagnostic) = error
    .chain()
    .find_map(|cause| cause.downcast_ref::<crate::runtime::process::skill_selection::Diagnostic>())
  {
    return fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      diagnostic.code,
      "skill selection rejected",
      json!({"diagnostics":[diagnostic]}),
    );
  }
  if let Some(failure) = error
    .chain()
    .find_map(|cause| cause.downcast_ref::<packages::PackageFailure>())
  {
    let diagnostic = failure.diagnostic();
    let code = diagnostic["code"]
      .as_str()
      .unwrap_or("package_invalid")
      .to_owned();
    return fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      code,
      "package admission rejected",
      json!({"diagnostics":[diagnostic]}),
    );
  }
  internal_err(StatusCode::UNPROCESSABLE_ENTITY, error)
}
fn admission_diagnostic(error: &anyhow::Error) -> Value {
  if let Some(diagnostic) = error
    .chain()
    .find_map(|cause| cause.downcast_ref::<crate::runtime::owners::ei_state::Diagnostic>())
  {
    return json!(diagnostic);
  }
  if let Some(diagnostic) = error
    .chain()
    .find_map(|cause| cause.downcast_ref::<crate::runtime::process::skill_selection::Diagnostic>())
  {
    return json!(diagnostic);
  }
  error
    .chain()
    .find_map(|cause| cause.downcast_ref::<graph_contract::Diagnostic>())
    .map(|diagnostic| json!(diagnostic))
    .or_else(|| {
      error
        .chain()
        .find_map(|cause| cause.downcast_ref::<packages::PackageFailure>())
        .map(packages::PackageFailure::diagnostic)
    })
    .unwrap_or_else(
      || json!({"path":"/","code":"invalid_graph","message":"graph admission rejected"}),
    )
}
fn node_option_err(error: &anyhow::Error) -> (StatusCode, Json<Value>) {
  if let Some(diagnostic) = error
    .chain()
    .find_map(|cause| cause.downcast_ref::<graph_contract::Diagnostic>())
  {
    let code = diagnostic.code;
    return fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      code,
      "graph admission rejected",
      json!({"diagnostics":[diagnostic]}),
    );
  }
  admission_err(error)
}
fn record(app: &App, source: &str, message: &str) {
  if let Ok(logs) = LogStore::new(&app.paths, &app.deployment) {
    let _ = logs.append(source, "info", message);
  }
}

fn record_graph_failure(app: &App, result: &ApiResult, apply: bool) {
  let body = match result {
    Ok(Json(body)) if body["data"]["valid"] == false => body,
    Err((StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED, _)) => return,
    Err((_, Json(body))) => body,
    _ => return,
  };
  if let Ok(logs) = LogStore::new(&app.paths, &app.deployment) {
    let diagnostic = if body["ok"] == false {
      &body["error"]["details"]["diagnostics"][0]
    } else {
      &body["data"]["diagnostics"][0]
    };
    let mut fields = serde_json::Map::new();
    if let Some(code) = diagnostic["code"].as_str().filter(|s| s.len() <= 256) {
      fields.insert("diagnostic_code".into(), json!(code));
    }
    if let Some(path) = diagnostic["path"]
      .as_str()
      .filter(|s| s.len() <= 256 && !s.chars().any(char::is_control))
    {
      fields.insert("diagnostic_path".into(), json!(path));
    }
    if let Some(message) = diagnostic["message"]
      .as_str()
      .filter(|s| s.len() <= 512 && !s.chars().any(char::is_control))
    {
      fields.insert("diagnostic_message".into(), json!(message));
    }
    let correlation = body["error"]["correlation_id"]
      .as_str()
      .filter(|s| s.len() <= 128 && !s.chars().any(char::is_control))
      .map(str::to_owned)
      .unwrap_or_else(crate::runtime::manager::next_correlation_id);
    let manager_scope = logs
      .manager_scope_id()
      .unwrap_or_else(|| crate::runtime::manager::manager_session_id().to_owned());
    let append = if apply {
      LogStore::append_apply_failure_context
    } else {
      LogStore::append_event_context
    };
    let _ = append(
      &logs,
      "isos",
      "error",
      "deployment_preflight_failed",
      "Deployment preflight failed before launch",
      crate::runtime::logs::LogContext {
        run_id: Some(&manager_scope),
        correlation_id: Some(&correlation),
        node_path: None,
        fields: Value::Object(fields),
      },
    );
  }
}
fn record_package(
  app: &App,
  level: &str,
  code: &str,
  id: &str,
  version: &str,
  _source_kind: &str,
  path: Option<&std::path::Path>,
) {
  if let Ok(logs) = LogStore::new(&app.paths, &app.deployment) {
    let receipt = if path.is_some() {
      match packages::verify(&app.paths, id, version) {
        Ok(verified) => Some(verified.receipt),
        Err(error) => {
          tracing::warn!(%error, "package receipt unavailable while recording event");
          None
        }
      }
    } else {
      None
    };
    let correlation = crate::runtime::manager::next_correlation_id();
    let manager_scope = logs
      .manager_scope_id()
      .unwrap_or_else(|| crate::runtime::manager::manager_session_id().to_owned());
    let _ = logs.append_event_context("packages", level, code,
            if level == "error" { "Package verification or installation failed" } else if code == "package_verified" { "Verified package release recorded" } else { "Local package link recorded without a release manifest" },
            crate::runtime::logs::LogContext { run_id: Some(&manager_scope), correlation_id: Some(&correlation), node_path: None,
            fields: json!({"package_id":id,"package_version":version,
                "source_commit":receipt.as_ref().and_then(|receipt| receipt.source_commit.as_deref()),
                "asset_sha256":receipt.as_ref().and_then(|receipt| receipt.asset_sha256.as_deref()),
                "release_sha256":receipt.as_ref().and_then(|receipt| receipt.release_sha256.as_deref()),
                "payload_sha256":receipt.as_ref().and_then(|receipt| receipt.payload_sha256.as_deref()),
                "source_kind":receipt.as_ref().and_then(|receipt| receipt.source_kind.as_deref())}) });
  }
}
fn revision(bytes: &[u8]) -> String {
  hex::encode(Sha256::digest(bytes))
}

use routes::graph::*;

use routes::catalog::*;

use routes::presentations::*;

use routes::package_handlers::*;

use routes::runtime::*;

pub fn serve(paths: Paths, deployment: String) -> Result<()> {
  paths.prepare()?;
  let logs = LogStore::new(&paths, &deployment)?;
  let app = Arc::new(App { paths, deployment });
  let router = router(app).layer(middleware::from_fn_with_state(
    (logs.clone(), "http_request"),
    log_request,
  ));
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()?;
  runtime.block_on(async {
    let address: SocketAddr = format!("127.0.0.1:{}", local_port()?).parse()?;
    let listener = tokio::net::TcpListener::bind(address)
      .await
      .context("bind IsOS loopback API")?;
    logs.append_event("isos", "info", "http_listening", "IsOS API listening")?;
    axum::serve(listener, router)
      .with_graceful_shutdown(async {
        while !crate::app::foreground::INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst) {
          tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
      })
      .await?;
    Ok(())
  })
}

async fn log_request(
  State((logs, code)): State<(LogStore, &'static str)>,
  request: Request<Body>,
  next: Next,
) -> Response {
  let method = request.method().as_str().to_owned();
  // Matched route patterns omit user-controlled path segments and query data.
  let route = request
    .extensions()
    .get::<MatchedPath>()
    .map(MatchedPath::as_str)
    .unwrap_or("unmatched")
    .to_owned();
  let started = Instant::now();
  let response = next.run(request).await;
  let status = response.status().as_u16();
  let level = if status >= 500 {
    "error"
  } else if status >= 400 {
    "warn"
  } else {
    "debug"
  };
  if status >= 400
    && let Err(error) = logs.append_diagnostic(
      level,
      "http_request_failed",
      &format!("HTTP {method} {route} returned {status}"),
    )
  {
    eprintln!("isos HTTP diagnostic sink failed: {error:#}");
  }
  if let Err(error) = logs.append_event_context(
    "isos",
    level,
    code,
    "IsOS HTTP request completed",
    crate::runtime::logs::LogContext {
      run_id: None,
      correlation_id: None,
      node_path: None,
      fields: json!({
        "method": method,
        "route": route,
        "status": status,
        "duration_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
      }),
    },
  ) {
    eprintln!("isos HTTP event sink failed: {error:#}");
  }
  response
}

pub fn serve_ui(directory: PathBuf, logs: LogStore) -> Result<()> {
  let index = directory.join("index.html");
  anyhow::ensure!(index.is_file(), "selected IsosUI index.html is missing");
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()?;
  runtime.block_on(async {
    let api_origin = local_origin()?;
    let address: SocketAddr = format!("127.0.0.1:{}", ui_port()?).parse()?;
    let listener = tokio::net::TcpListener::bind(address)
      .await
      .context("bind IsosUI loopback server")?;
    let files = ServeDir::new(directory).fallback(ServeFile::new(index));
    let router = Router::new()
      .route(
        "/isosui-runtime.json",
        axum::routing::get(move || {
          let api_origin = api_origin.clone();
          async move { Json(json!({"api_origin": api_origin})) }
        }),
      )
      .fallback_service(files)
      .layer(middleware::from_fn_with_state(
        (logs.clone(), "ui_request"),
        log_request,
      ));
    logs.append_event("isos", "info", "ui_listening", "IsOS workbench listening")?;
    axum::serve(listener, router)
      .with_graceful_shutdown(async {
        while !crate::app::foreground::INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst) {
          tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
      })
      .await?;
    Ok(())
  })
}
mod routes;
use routes::router;

#[cfg(test)]
mod tests {
  use super::*;
  use axum::body::to_bytes;
  use tower::ServiceExt;

  use std::io::{BufRead, BufReader};
  use std::os::unix::net::UnixListener;
  use std::time::{Duration, Instant};

  #[tokio::test]
  async fn http_log_records_matched_route_without_query_values() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let logs = LogStore::new(&paths, "default").unwrap();
    let app = Arc::new(App {
      paths,
      deployment: "default".into(),
    });
    let request = Request::builder()
      .uri("/api/isos/v1/system/health?secret=never-log-this")
      .header(header::HOST, "127.0.0.1:16000")
      .body(Body::empty())
      .unwrap();
    let response = router(app)
      .layer(middleware::from_fn_with_state(
        (logs.clone(), "http_request"),
        log_request,
      ))
      .oneshot(request)
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let text = fs::read_to_string(logs.root().join("isos.jsonl")).unwrap();
    let event: Value = serde_json::from_str(text.lines().last().unwrap()).unwrap();
    assert_eq!(event["event_code"], "http_request");
    assert_eq!(event["fields"]["route"], "/api/isos/v1/system/health");
    assert!(!text.contains("never-log-this"));
  }

  #[tokio::test]
  async fn presentation_request_rejects_unregistered_or_ambiguous_method() {
    let app = Arc::new(App {
      paths: Paths {
        config: PathBuf::from("/tmp/isos-presentation-test/config"),
        data: PathBuf::from("/tmp/isos-presentation-test/data"),
        runtime: PathBuf::from("/tmp/isos-presentation-test/run"),
      },
      deployment: "default".into(),
    });
    let request = Request::builder()
      .method(Method::POST)
      .uri("/api/isos/v1/presentations/select")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from(
        json!({"schema_version":"0.1.0","node_path":"eos/interface/camera/front",
                    "port":"frame","accept":["image","image"],"run_id":"r_test",
                    "graph_revision":"a".repeat(64)})
        .to_string(),
      ))
      .unwrap();
    let response = router(app).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value =
      serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
    assert_eq!(body["schema_version"], "0.1.0");
    assert_eq!(body["error"]["code"], "invalid_request");
  }

  #[tokio::test(flavor = "current_thread")]
  async fn stalled_manager_status_returns_503_without_blocking_http_worker() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let listener = UnixListener::bind(paths.socket()).unwrap();
    let server = std::thread::spawn(move || {
      let (stream, _) = listener.accept().unwrap();
      let mut reader = BufReader::new(stream);
      let mut line = String::new();
      reader.read_line(&mut line).unwrap();
      assert!(line.contains("deployment_status"));
      std::thread::sleep(Duration::from_millis(2300));
    });
    let app = Arc::new(App {
      paths,
      deployment: "default".into(),
    });
    let request = Request::builder()
      .uri("/api/isos/v1/runtime/process-status")
      .header(header::HOST, "127.0.0.1:16000")
      .body(Body::empty())
      .unwrap();
    let started = Instant::now();
    let response = router(app).oneshot(request);
    tokio::pin!(response);
    assert!(
      tokio::time::timeout(Duration::from_millis(50), &mut response)
        .await
        .is_err()
    );
    assert!(started.elapsed() < Duration::from_millis(500));
    let response = response.await;
    assert!(started.elapsed() >= Duration::from_secs(2));
    let response = response.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: Value =
      serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
    assert_eq!(body["error"]["message"], "process status is unavailable");
    server.join().unwrap();
  }

  #[tokio::test(flavor = "current_thread")]
  async fn selected_catalog_queue_is_bounded_and_does_not_block_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let graph = crate::domain::graph::starter::draft(&paths, "default").unwrap();
    let first = catalog_slots().clone().acquire_owned().await.unwrap();
    let second = catalog_slots().clone().acquire_owned().await.unwrap();
    let pending = selected_catalog_read(paths, graph);
    tokio::pin!(pending);
    assert!(
      tokio::time::timeout(Duration::from_millis(25), &mut pending)
        .await
        .is_err()
    );
    drop(first);
    drop(second);
    // The request leaves the bounded queue. This isolated test has no owner
    // catalog, so the selected three-core catalog must fail closed.
    assert!(
      tokio::time::timeout(Duration::from_secs(5), pending)
        .await
        .unwrap()
        .is_err()
    );
  }

  #[tokio::test(flavor = "current_thread")]
  async fn global_type_and_node_set_reads_share_bounded_workers() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let app = Arc::new(App {
      paths,
      deployment: "default".into(),
    });
    let first = catalog_slots().clone().acquire_owned().await.unwrap();
    let second = catalog_slots().clone().acquire_owned().await.unwrap();
    for route in ["node-types", "node-sets"] {
      let request = Request::builder()
        .uri(format!("/api/isos/v1/catalog/{route}"))
        .header(header::HOST, "127.0.0.1:16000")
        .body(Body::empty())
        .unwrap();
      let response = router(app.clone()).oneshot(request);
      tokio::pin!(response);
      assert!(
        tokio::time::timeout(Duration::from_millis(25), &mut response)
          .await
          .is_err()
      );
    }
    drop(first);
    drop(second);
    for route in ["node-types", "node-sets"] {
      let request = Request::builder()
        .uri(format!("/api/isos/v1/catalog/{route}"))
        .header(header::HOST, "127.0.0.1:16000")
        .body(Body::empty())
        .unwrap();
      let response = router(app.clone()).oneshot(request).await.unwrap();
      assert_eq!(response.status(), StatusCode::OK);
    }
  }

  #[tokio::test]
  async fn future_graph_selected_catalog_reports_version_path_without_writing() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    let bytes = include_bytes!("../../../tests/fixtures/graph_compat/future_2_0_0.json");
    let path = app.paths.node_graph_path("default");
    config::atomic_write(&path, bytes).unwrap();
    let request = Request::builder()
      .uri("/api/isos/v1/catalog/node-types/selected")
      .header(header::HOST, "127.0.0.1:16000")
      .body(Body::empty())
      .unwrap();
    let response = router(app).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value =
      serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
    assert_eq!(
      body["error"]["details"]["graph_revision"],
      graph_contract::revision(bytes)
    );
    assert_eq!(
      body["error"]["details"]["diagnostics"][0]["path"],
      "/version"
    );
    assert_eq!(
      body["error"]["details"]["diagnostics"][0]["code"],
      "unsupported_graph_version"
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
  }

  #[test]
  fn graph_failure_event_keeps_structured_path_and_manager_session() {
    let temp = tempfile::tempdir().unwrap();
    let app = App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    };
    let result = Ok(Json(json!({"data":{"valid":false,"diagnostics":[
        {"code":"unsupported_core","path":"/cores/evil","message":"unsupported core evil"}
    ]}})));
    record_graph_failure(&app, &result, false);
    let lines = LogStore::new(&app.paths, "default")
      .unwrap()
      .read("isos", None, 10)
      .unwrap();
    let event = &lines["lines"][0];
    assert_eq!(event["event_code"], "deployment_preflight_failed");
    assert_eq!(
      event["run_id"],
      crate::runtime::manager::manager_session_id()
    );
    assert_eq!(event["fields"]["diagnostic_code"], "unsupported_core");
    assert_eq!(event["fields"]["diagnostic_path"], "/cores/evil");
    assert_eq!(
      event["fields"]["diagnostic_message"],
      "unsupported core evil"
    );
    record_graph_failure(
      &app,
      &Err((
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(
          json!({"ok":false,"error":{"correlation_id":"request-42","details":{"diagnostics":[
            {"code":"invalid_graph","path":"/cores/ei","message":"graph requires the ei core"}
          ]}}}),
        ),
      )),
      false,
    );
    let lines = LogStore::new(&app.paths, "default")
      .unwrap()
      .read("isos", None, 10)
      .unwrap();
    assert_eq!(lines["lines"][1]["correlation_id"], "request-42");
    assert_eq!(
      lines["lines"][1]["fields"]["diagnostic_message"],
      "graph requires the ei core"
    );
    record_graph_failure(
      &app,
      &Ok(Json(json!({"data":{"valid":false,"diagnostics":[
          {"code":"unsupported_core","path":"/cores/bad\nkey"}
      ]}}))),
      false,
    );
    let lines = LogStore::new(&app.paths, "default")
      .unwrap()
      .read("isos", None, 10)
      .unwrap();
    assert_eq!(
      lines["lines"][2]["event_code"],
      "deployment_preflight_failed"
    );
    assert!(lines["lines"][2]["fields"]["diagnostic_path"].is_null());
    record_graph_failure(
      &app,
      &Err(err(
        StatusCode::FORBIDDEN,
        "cross-site request requires Origin",
      )),
      false,
    );
    assert_eq!(
      LogStore::new(&app.paths, "default")
        .unwrap()
        .read("isos", None, 10)
        .unwrap()["lines"]
        .as_array()
        .unwrap()
        .len(),
      3
    );
  }

  #[tokio::test]
  async fn core_setup_api_rejects_unknown_role_and_missing_graph() {
    for (route, expected) in [
      (
        "/api/isos/v1/cores/setup/other/catalog",
        StatusCode::NOT_FOUND,
      ),
      (
        "/api/isos/v1/cores/setup/eos/validate",
        StatusCode::BAD_REQUEST,
      ),
    ] {
      let request = Request::builder()
        .uri(route)
        .header(header::HOST, "127.0.0.1:16000")
        .body(Body::empty())
        .unwrap();
      let response = test_router().oneshot(request).await.unwrap();
      assert_eq!(response.status(), expected);
    }
  }

  #[tokio::test]
  #[ignore = "requires ISOS_TEST_UNIFIED_HOME and ISOS_TEST_UNIFIED_GRAPH_PATH"]
  async fn fresh_unified_http_apply_publishes_and_rejects_stale_revision() {
    let home = PathBuf::from(std::env::var("ISOS_TEST_UNIFIED_HOME").unwrap());
    let graph_path = PathBuf::from(std::env::var("ISOS_TEST_UNIFIED_GRAPH_PATH").unwrap());
    assert!(home.is_absolute() && graph_path.is_absolute());
    let app = Arc::new(App {
      paths: Paths {
        config: home.join("config"),
        data: home.join("data"),
        runtime: home.join("run"),
      },
      deployment: "default".into(),
    });
    let saved_path = app.paths.node_graph_path("default");
    assert!(
      !saved_path.exists(),
      "fresh writer test needs no saved graph"
    );
    let graph: Value = serde_json::from_slice(&fs::read(graph_path).unwrap()).unwrap();
    assert_eq!(graph["version"], "0.2.0");
    let request = Request::builder()
      .method(Method::PUT)
      .uri("/api/isos/v1/graph")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from(
        serde_json::to_vec(&json!({"schema_version":"0.2.0","revision":"","graph":graph})).unwrap(),
      ))
      .unwrap();
    let response = router(app.clone()).oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let published = fs::read(&saved_path).unwrap();
    let revision = graph_contract::revision(&published);
    assert!(
      crate::domain::graph::generated_v2::selected_root(
        &app.paths,
        "default",
        &serde_json::from_slice(&published).unwrap(),
        &revision
      )
      .is_ok()
    );
    let request = Request::builder().method(Method::PUT).uri("/api/isos/v1/graph")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from(serde_json::to_vec(&json!({"schema_version":"0.2.0","revision":"","graph":serde_json::from_slice::<Value>(&published).unwrap()})).unwrap())).unwrap();
    let response = router(app).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(fs::read(saved_path).unwrap(), published);
  }

  #[tokio::test]
  async fn three_core_validate_and_apply_require_registered_wrapper_catalog() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    let graph = crate::domain::graph::starter::draft(&app.paths, "default").unwrap();
    assert!(graph["cores"]["ei"].is_object());
    assert!(graph["cores"]["eos"].is_object());
    assert!(
      graph
        .pointer("/cores/isos/layers/process/groups/component/nodes/ei")
        .is_none()
    );
    let path = app.paths.node_graph_path("default");
    let saved = serde_json::to_vec_pretty(&graph).unwrap();
    config::atomic_write(&path, &saved).unwrap();
    let revision = graph_contract::revision(&saved);
    let validate = Request::builder()
      .method(Method::POST)
      .uri("/api/isos/v1/graph/validate")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from(
        serde_json::to_vec(&json!({"schema_version":"0.1.0","graph":graph})).unwrap(),
      ))
      .unwrap();
    let response = router(app.clone()).oneshot(validate).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
      serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
    assert_eq!(body["data"]["valid"], false);
    assert!(matches!(
      body["data"]["diagnostics"][0]["code"].as_str(),
      Some("unsupported_node_type" | "owner_reader_unavailable")
    ));
    let apply = Request::builder()
      .method(Method::PUT)
      .uri("/api/isos/v1/graph")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from(
        serde_json::to_vec(&json!({"schema_version":"0.1.0","revision":revision,"graph":graph}))
          .unwrap(),
      ))
      .unwrap();
    let response = router(app).oneshot(apply).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(fs::read(path).unwrap(), saved);
  }

  fn test_router() -> Router {
    router(Arc::new(App {
      paths: Paths {
        config: PathBuf::from("/tmp/isos-test/config"),
        data: PathBuf::from("/tmp/isos-test/data"),
        runtime: PathBuf::from("/tmp/isos-test/run"),
      },
      deployment: "default".into(),
    }))
  }

  #[tokio::test]
  async fn obsolete_component_and_build_routes_are_absent() {
    for route in [
      "/api/isos/v2/graph",
      "/api/isos/v2/graph/validate",
      "/api/isos/v1/profiles",
      "/api/isos/v1/config/eos/describe",
      "/api/isos/v1/build-binaries",
      "/api/isos/v1/build-binaries/select",
    ] {
      let request = Request::builder()
        .uri(route)
        .header(header::HOST, "127.0.0.1:16000")
        .body(Body::empty())
        .unwrap();
      let response = test_router().oneshot(request).await.unwrap();
      assert_eq!(response.status(), StatusCode::NOT_FOUND, "{route}");
    }
  }

  #[tokio::test]
  async fn categorized_logs_return_bounded_cursor_envelope() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    let store = LogStore::new(&app.paths, "default").unwrap();
    store.append("isos", "info", "ready").unwrap();
    for (route, expected) in [
      (
        "/api/isos/v1/runtime/logs?source=isos&limit=1",
        StatusCode::OK,
      ),
      (
        "/api/isos/v1/runtime/logs?source=isos&limit=201",
        StatusCode::BAD_REQUEST,
      ),
      (
        "/api/isos/v1/runtime/logs?source=..%2Fsecret",
        StatusCode::BAD_REQUEST,
      ),
    ] {
      let request = Request::builder()
        .uri(route)
        .header(header::HOST, "127.0.0.1:16000")
        .body(Body::empty())
        .unwrap();
      let response = router(app.clone()).oneshot(request).await.unwrap();
      assert_eq!(response.status(), expected);
      if expected == StatusCode::OK {
        let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"]["lines"][0]["message"], "ready");
        assert_eq!(
          body["data"]["lines"][0]["source"],
          "isos.runtime.isos.event@aether-v0.2.1"
        );
        assert!(body["data"]["next_cursor"].is_string());
      }
    }
  }

  #[tokio::test]
  async fn current_logs_preserve_typed_errors_and_source_filter() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    let store = LogStore::new(&app.paths, "default").unwrap();
    let run = "rhttp1";
    store.publish_run(run).unwrap();
    store
      .append_event_context(
        "isos",
        "info",
        "deployment_ready",
        "ready",
        crate::runtime::logs::LogContext {
          run_id: Some(run),
          correlation_id: None,
          node_path: None,
          fields: json!({}),
        },
      )
      .unwrap();
    for (route, status, code) in [
      ("/api/isos/v1/runtime/logs?view=current", StatusCode::OK, ""),
      (
        "/api/isos/v1/runtime/logs?view=current&cursor=123:456",
        StatusCode::BAD_REQUEST,
        "log_cursor_invalid",
      ),
      (
        "/api/isos/v1/runtime/logs?view=current&node_path=bad",
        StatusCode::BAD_REQUEST,
        "log_node_path_invalid",
      ),
      (
        "/api/isos/v1/runtime/logs?view=other",
        StatusCode::BAD_REQUEST,
        "log_view_invalid",
      ),
      ("/api/isos/v1/runtime/logs?source=isos", StatusCode::OK, ""),
    ] {
      let request = Request::builder()
        .uri(route)
        .header(header::HOST, "127.0.0.1:16000")
        .body(Body::empty())
        .unwrap();
      let response = router(app.clone()).oneshot(request).await.unwrap();
      assert_eq!(response.status(), status, "{route}");
      let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
      if code.is_empty() && route.contains("view=current") {
        assert_eq!(body["data"]["lines"][0]["event_code"], "deployment_ready");
        assert!(
          body["data"]["next_cursor"]
            .as_str()
            .unwrap()
            .starts_with("lf1.")
        );
      } else if !code.is_empty() {
        assert_eq!(body["error"]["details"]["diagnostics"][0]["code"], code);
      }
    }
  }

  #[tokio::test]
  async fn logs_filter_level_and_default_to_unified_stream() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    let store = LogStore::new(&app.paths, "default").unwrap();
    let path = app.paths.owner_data("eos").join("log/default/eos.jsonl");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let run_id = store.manager_scope_id().unwrap();
    fs::write(&path, format!("{}\n{}\n",
      json!({"timestamp":"2026-11-09T09:00:00Z","level":"info","source":"eos.owner_service.rs@aether-v0.2.1","message":"ready","run_id":run_id}),
      json!({"timestamp":"2026-11-09T09:00:01Z","level":"error","source":"eos.owner_service.rs@aether-v0.2.1","message":"failed","run_id":run_id})
    )).unwrap();
    for (route, status, expected) in [
      (
        "/api/isos/v1/runtime/logs?source=eos&level=warn",
        StatusCode::OK,
        "failed",
      ),
      (
        "/api/isos/v1/runtime/logs?source=eos&level=verbose",
        StatusCode::BAD_REQUEST,
        "",
      ),
    ] {
      let request = Request::builder()
        .uri(route)
        .header(header::HOST, "127.0.0.1:16000")
        .body(Body::empty())
        .unwrap();
      let response = router(app.clone()).oneshot(request).await.unwrap();
      assert_eq!(response.status(), status);
      if status == StatusCode::OK {
        let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"]["lines"].as_array().unwrap().len(), 1);
        let line = &body["data"]["lines"][0];
        assert_eq!(
          line["message"].as_str().or_else(|| line.as_str()),
          Some(expected)
        );
      }
    }
    let request = Request::builder()
      .uri("/api/isos/v1/runtime/logs?limit=2")
      .header(header::HOST, "127.0.0.1:16000")
      .body(Body::empty())
      .unwrap();
    let response = router(app).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["data"]["lines"].as_array().unwrap().len(), 2);
  }

  #[tokio::test]
  async fn first_node_graph_read_returns_editable_v1_draft() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    let request = Request::builder()
      .uri("/api/isos/v1/graph")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::ORIGIN, "https://www.os-island.com")
      .body(Body::empty())
      .unwrap();
    let response = router(app).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["data"]["revision"], "");
    assert_eq!(body["data"]["graph"]["version"], "0.2.0");
  }

  #[tokio::test]
  async fn new_graph_draft_fences_obsolete_saved_graph_without_changing_it() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    let old = br#"{"version":"0.1.0","cores":{"eos":{}}}"#;
    let path = app.paths.node_graph_path("default");
    config::atomic_write(&path, old).unwrap();
    let request = Request::builder()
      .uri("/api/isos/v1/graph/new")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::ORIGIN, "https://www.os-island.com")
      .body(Body::empty())
      .unwrap();
    let response = router(app).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["data"]["revision"], graph_contract::revision(old));
    assert_eq!(body["data"]["graph"]["version"], "0.2.0");
    assert_eq!(body["data"]["graph"]["robot"]["id"], "default");
    assert_eq!(body["data"]["graph"]["cores"].as_object().unwrap().len(), 3);
    assert_eq!(
      body["data"]["graph"]["cores"]["ei"]["layers"]["io"]["groups"]["input"]["nodes"]["trigger"]["node_type"],
      "ei.io.input.trigger@v0.2"
    );
    assert_eq!(fs::read(path).unwrap(), old);
  }

  #[tokio::test]
  async fn home_reports_missing_package_and_graph() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    for route in ["/api/isos/v1/packages", "/api/isos/v1/cores/setup-status"] {
      let request = Request::builder()
        .uri(route)
        .header(header::HOST, "127.0.0.1:16000")
        .header(header::ORIGIN, "https://www.os-island.com")
        .body(Body::empty())
        .unwrap();
      let response = router(app.clone()).oneshot(request).await.unwrap();
      assert_eq!(response.status(), StatusCode::OK);
      let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
      let body: Value = serde_json::from_slice(&bytes).unwrap();
      if route.ends_with("packages") {
        assert_eq!(body["data"]["installed"], json!([]));
      } else {
        assert_eq!(body["data"]["ready"], false);
        assert!(body["data"]["issues"].as_array().unwrap().len() >= 2);
      }
    }
  }

  #[tokio::test]
  async fn unverifiable_bundle_link_is_rejected_without_store_change() {
    let temp = tempfile::tempdir().unwrap();
    let bundle = temp.path().join("bundle");
    fs::create_dir(&bundle).unwrap();
    let app = Arc::new(App {
      paths: Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    app.paths.prepare().unwrap();
    let request = Request::builder()
      .method(Method::POST)
      .uri("/api/isos/v1/packages/link")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from(
        json!({"id":"test/sample","version":"1.0.0","path":bundle}).to_string(),
      ))
      .unwrap();
    let response = router(app.clone()).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value =
      serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
    assert_eq!(body["error"]["code"], "unsupported_package_contract");
    assert!(packages::list(&app.paths).unwrap().is_empty());
    assert!(
      !packages::package_path(&app.paths, "test/sample", "1.0.0")
        .unwrap()
        .exists()
    );
  }

  #[tokio::test]
  async fn loopback_and_trusted_browser_origins_connect_without_token() {
    let request = Request::builder()
      .method(Method::OPTIONS)
      .uri("/api/isos/v1/system/version")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::ORIGIN, "https://www.os-island.com")
      .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
      .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
      .body(Body::empty())
      .unwrap();
    let response = test_router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
      response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
      "https://www.os-island.com"
    );

    let request = Request::builder()
      .uri("/api/isos/v1/system/version")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::ORIGIN, "https://www.os-island.com")
      .body(Body::empty())
      .unwrap();
    let response = test_router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
      "https://www.os-island.com"
    );

    let request = Request::builder()
      .uri("/api/isos/v1/system/version")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::ORIGIN, "https://www.os-island.com")
      .body(Body::empty())
      .unwrap();
    let response = test_router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
      "https://www.os-island.com"
    );

    for headers in [
      [("origin", "http://127.0.0.1:16000")],
      [("origin", "http://127.0.0.1:16001")],
      [("sec-fetch-site", "same-origin")],
    ] {
      let mut request = Request::builder()
        .uri("/api/isos/v1/system/version")
        .header(header::HOST, "127.0.0.1:16000");
      for (name, value) in headers {
        request = request.header(name, value);
      }
      let response = test_router()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
      assert_eq!(response.status(), StatusCode::OK);
    }

    let request = Request::builder()
      .uri("/api/isos/v1/system/version")
      .header(header::HOST, "127.0.0.1:16000")
      .body(Body::empty())
      .unwrap();
    assert_eq!(
      test_router().oneshot(request).await.unwrap().status(),
      StatusCode::OK
    );
    let request = Request::builder()
      .uri("/api/isos/v1/system/version")
      .header(header::HOST, "127.0.0.1:16000")
      .header("sec-fetch-site", "cross-site")
      .body(Body::empty())
      .unwrap();
    assert_eq!(
      test_router().oneshot(request).await.unwrap().status(),
      StatusCode::FORBIDDEN
    );
  }

  #[tokio::test]
  async fn old_route_is_absent_and_numeric_schema_is_typed_error() {
    let old = Request::builder()
      .uri("/api/isos/v2/system/version")
      .header(header::HOST, "127.0.0.1:16000")
      .body(Body::empty())
      .unwrap();
    assert_eq!(
      test_router().oneshot(old).await.unwrap().status(),
      StatusCode::NOT_FOUND
    );

    let request = Request::builder()
      .method(Method::POST)
      .uri("/api/isos/v1/presentations/select")
      .header(header::HOST, "127.0.0.1:16000")
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from(r#"{"schema_version":2,"node_path":"eos/interface/camera/front","port":"frame","accept":[],"run_id":"run","graph_revision":"rev"}"#))
      .unwrap();
    let response = test_router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), 4096)
      .await
      .unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["schema_version"], "0.1.0");
    assert_eq!(body["error"]["code"], "unsupported_schema_version");
  }

  #[tokio::test]
  async fn rejects_other_origins_and_hosts() {
    for (host, origin) in [
      ("127.0.0.1:16000", "https://evil.example"),
      ("127.0.0.1:16000", "http://127.0.0.1:16002"),
      ("localhost:16000", "https://www.os-island.com"),
    ] {
      let request = Request::builder()
        .uri("/api/isos/v1/system/version")
        .header(header::HOST, host)
        .header(header::ORIGIN, origin)
        .body(Body::empty())
        .unwrap();
      let response = test_router().oneshot(request).await.unwrap();
      assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
  }
  #[tokio::test]
  async fn api_does_not_serve_frontend_pages() {
    for route in ["/", "/en/isos", "/assets/app.js"] {
      let response = test_router()
        .oneshot(
          Request::builder()
            .uri(route)
            .header(header::HOST, "127.0.0.1:16000")
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
      assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
  }
}
