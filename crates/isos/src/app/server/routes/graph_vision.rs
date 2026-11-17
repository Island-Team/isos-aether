//! Graph-bound Vision projection. EOS produces and validates each frame.
use super::*;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::domain::graph::{edge_admission, vision_bindings};

const MAX_FRAME_BYTES: usize = 20 * 1024 * 1024;
const MAX_HEADER_BYTES: usize = 4096;

async fn active(
  app: &App,
) -> Result<(String, String, Vec<vision_bindings::Binding>), (StatusCode, Json<Value>)> {
  let paths = app.paths.clone();
  let deployment = app.deployment.clone();
  let status = tokio::task::spawn_blocking(move || {
    crate::runtime::manager::manager_status_for_http(&paths, &deployment)
  })
  .await
  .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?
  .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if status["state"] != "running" || status["run_id_state"] != "known" {
    return Err(fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "vision_run_unavailable",
      "Vision run is unavailable",
      Value::Null,
    ));
  }
  let run_id = status["run_id"]
    .as_str()
    .ok_or_else(|| err(StatusCode::SERVICE_UNAVAILABLE, "Active run is unavailable"))?
    .to_owned();
  let revision = status["active_revision"]
    .as_str()
    .ok_or_else(|| {
      err(
        StatusCode::SERVICE_UNAVAILABLE,
        "Active revision is unavailable",
      )
    })?
    .to_owned();
  let graph =
    crate::domain::graph::graph_contract::read(&app.paths.node_graph_path(&app.deployment))
      .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if graph.revision != revision {
    return Err(fail(
      StatusCode::CONFLICT,
      "run_fence_stale",
      "Active graph revision is stale",
      Value::Null,
    ));
  }
  let catalog = crate::runtime::process::catalog_for_graph(&app.paths, &graph.value)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  let edges = edge_admission::admit(&graph.value, &catalog).map_err(|_| {
    err(
      StatusCode::SERVICE_UNAVAILABLE,
      "Vision graph route is unavailable",
    )
  })?;
  let bindings = vision_bindings::bindings(&graph.value, &catalog, &edges).map_err(|_| {
    err(
      StatusCode::SERVICE_UNAVAILABLE,
      "Vision graph binding is unavailable",
    )
  })?;
  Ok((run_id, revision, bindings))
}

pub(crate) async fn bindings(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  let (run_id, revision, bindings) = active(&app).await?;
  ok(json!({"run_id":run_id,"revision":revision,"bindings":bindings}))
}

pub(crate) async fn websocket(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
  authorized(&app, &headers)?;
  let (run_id, revision, bindings) = active(&app).await?;
  let graph_path = app.paths.node_graph_path(&app.deployment);
  let paths = app.paths.clone();
  Ok(
    upgrade
      .max_message_size(4096)
      .on_upgrade(move |socket| relay(socket, paths, run_id, revision, bindings, graph_path)),
  )
}

async fn relay(
  mut socket: WebSocket,
  paths: crate::app::config::Paths,
  run_id: String,
  revision: String,
  bindings: Vec<vision_bindings::Binding>,
  graph_path: std::path::PathBuf,
) {
  let Ok(Some(Ok(Message::Text(text)))) =
    tokio::time::timeout(std::time::Duration::from_secs(5), socket.next()).await
  else {
    return;
  };
  let Ok(request) = serde_json::from_str::<Value>(&text) else {
    return;
  };
  let Some(binding) = subscribed_binding(&request, &run_id, &revision, &bindings) else {
    return;
  };
  let request = json!({"schema_version":1,"core":"eos","op":"vision_subscribe","run_id":run_id,
    "revision":revision,"transfer_path":binding.transfer_path,"source_path":binding.source_path});
  let request_paths = paths.clone();
  let Ok(Ok(ticket)) = tokio::task::spawn_blocking(move || {
    crate::runtime::owners::owner_services::setup(&request_paths, "eos", &request)
  })
  .await
  else {
    return;
  };
  if ticket["ok"] != true
    || ticket["run_id"] != run_id
    || ticket["revision"] != revision
    || ticket["transfer_path"] != binding.transfer_path
    || ticket["source_path"] != binding.source_path
  {
    return;
  }
  let (Some(worker_socket), Some(token)) = (
    ticket["worker_socket"].as_str(),
    ticket["vision_token"].as_str(),
  ) else {
    return;
  };
  if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
    return;
  }
  let Ok(mut worker) = tokio::net::UnixStream::connect(worker_socket).await else {
    return;
  };
  let subscribe = json!({"op":"vision_subscribe","run_id":run_id,"revision":revision,
    "node":binding.source_path,"vision_token":token});
  if worker
    .write_all(subscribe.to_string().as_bytes())
    .await
    .is_err()
    || worker.shutdown().await.is_err()
  {
    return;
  }
  let mut last_sequence = None;
  let mut last_delivery: Option<tokio::time::Instant> = None;
  loop {
    if binding.single_frame && last_delivery.is_some() {
      break;
    }
    let frame = tokio::select! {
      incoming = socket.next() => { let _ = incoming; break; },
      frame = read_vision_packet(&mut worker) => frame,
    };
    let Ok((header, jpeg)) = frame else {
      break;
    };
    let graph_path = graph_path.clone();
    let Ok(current_revision) = tokio::task::spawn_blocking(move || {
      std::fs::read(&graph_path)
        .ok()
        .map(|bytes| crate::domain::graph::graph_contract::revision(&bytes))
    })
    .await
    else {
      break;
    };
    if current_revision.as_deref() != Some(&revision) {
      break;
    }
    if header["run_id"] != run_id
      || header["revision"] != revision
      || header["source_path"] != binding.source_path
      || header["encoding"] != "jpeg"
      || header["width_px"].as_u64().is_none_or(|value| value == 0)
      || header["height_px"].as_u64().is_none_or(|value| value == 0)
    {
      break;
    }
    let Some(sequence) = header["output_sequence"].as_u64() else {
      break;
    };
    if last_sequence.is_some_and(|previous| sequence <= previous) {
      break;
    }
    last_sequence = Some(sequence);
    let outbound_header = json!({"op":"frame","run_id":run_id,"revision":revision,
      "monitor_path":binding.monitor_path,"transfer_path":binding.transfer_path,
      "source_path":binding.source_path,"output_sequence":sequence,
      "sample":header["sample"],"width_px":header["width_px"],"height_px":header["height_px"],"encoding":"jpeg"});
    let Ok(header_bytes) = serde_json::to_vec(&outbound_header) else {
      break;
    };
    if header_bytes.len() > MAX_HEADER_BYTES {
      break;
    }
    let mut outbound = Vec::with_capacity(4 + header_bytes.len() + jpeg.len());
    outbound.extend_from_slice(&(header_bytes.len() as u32).to_be_bytes());
    outbound.extend_from_slice(&header_bytes);
    outbound.extend_from_slice(&jpeg);
    if let (Some(interval), Some(last)) = (binding.interval_ms, last_delivery) {
      tokio::select! {
        _ = socket.next() => break,
        _ = tokio::time::sleep_until(last + std::time::Duration::from_millis(interval)) => {}
      }
    }
    if socket.send(Message::Binary(outbound.into())).await.is_err() {
      break;
    }
    last_delivery = Some(tokio::time::Instant::now());
  }
  let _ = socket.send(Message::Close(None)).await;
}

fn subscribed_binding<'a>(
  request: &Value,
  run_id: &str,
  revision: &str,
  bindings: &'a [vision_bindings::Binding],
) -> Option<&'a vision_bindings::Binding> {
  if request["run_id"] != run_id || request["revision"] != revision {
    return None;
  }
  bindings
    .iter()
    .find(|binding| request["monitor_path"] == binding.monitor_path)
}

async fn read_vision_packet(
  worker: &mut tokio::net::UnixStream,
) -> std::io::Result<(Value, Vec<u8>)> {
  let header_len = worker.read_u32().await? as usize;
  if header_len == 0 || header_len > MAX_HEADER_BYTES {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidData,
      "vision header too large",
    ));
  }
  let mut header = vec![0; header_len];
  worker.read_exact(&mut header).await?;
  let header = serde_json::from_slice(&header)
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
  let jpeg_len = worker.read_u32().await? as usize;
  if !(4..=MAX_FRAME_BYTES).contains(&jpeg_len) {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidData,
      "vision frame too large",
    ));
  }
  let mut jpeg = vec![0; jpeg_len];
  worker.read_exact(&mut jpeg).await?;
  if !jpeg.starts_with(&[0xff, 0xd8]) || !jpeg.ends_with(&[0xff, 0xd9]) {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidData,
      "invalid JPEG",
    ));
  }
  Ok((header, jpeg))
}

#[cfg(test)]
fn accepted_frame(
  value: &Value,
  run_id: &str,
  revision: &str,
  binding: &vision_bindings::Binding,
) -> Option<Value> {
  let sample = &value["sample"];
  if value["ok"] != true
    || value["run_id"] != run_id
    || value["revision"] != revision
    || value["transfer_path"] != binding.transfer_path
    || value["source_path"] != binding.source_path
    || sample["node"] != binding.transfer_path
    || sample["output"]["port"] != "frame"
    || !sample["output"]["payload"].is_object()
  {
    return None;
  }
  Some(json!({"op":"frame","run_id":run_id,"revision":revision,
    "monitor_path":binding.monitor_path,"transfer_path":binding.transfer_path,
    "source_path":binding.source_path,"output_sequence":sample["sequence"],
    "frame":sample["output"]["payload"]}))
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::{
    body::Body,
    http::{Request, header},
  };
  use tower::ServiceExt;

  fn binding() -> vision_bindings::Binding {
    vision_bindings::Binding {
      monitor_path: "isos/ui/vision/front".into(),
      transfer_path: "eos/interface/egress/front".into(),
      source_path: "eos/hardware/camera/front".into(),
      label: "Front".into(),
      interval_ms: Some(200),
      single_frame: false,
    }
  }

  #[test]
  fn subscription_requires_selected_monitor_and_active_fence() {
    let bindings = [binding()];
    let valid = json!({"monitor_path":"isos/ui/vision/front","run_id":"run-1","revision":"rev-1"});
    assert!(subscribed_binding(&valid, "run-1", "rev-1", &bindings).is_some());
    let mut wrong = valid.clone();
    wrong["monitor_path"] = json!("isos/ui/vision/other");
    assert!(subscribed_binding(&wrong, "run-1", "rev-1", &bindings).is_none());
    assert!(subscribed_binding(&valid, "run-2", "rev-1", &bindings).is_none());
    assert!(subscribed_binding(&valid, "run-1", "rev-2", &bindings).is_none());
  }

  #[test]
  fn owner_frame_requires_exact_binding_and_fence() {
    let bound = binding();
    let mut owner = json!({"ok":true,"run_id":"run-1","revision":"rev-1",
      "transfer_path":bound.transfer_path,"source_path":bound.source_path,"sample":{"node":bound.transfer_path,
      "sequence":1,"output":{"port":"frame","payload":{"data":"abc"}}}});
    let accepted = accepted_frame(&owner, "run-1", "rev-1", &bound).unwrap();
    assert_eq!(accepted["monitor_path"], "isos/ui/vision/front");
    assert_eq!(accepted["output_sequence"], 1);
    owner["sample"]["node"] = json!("eos/interface/egress/other");
    assert!(accepted_frame(&owner, "run-1", "rev-1", &bound).is_none());
    owner["sample"]["node"] = json!(bound.transfer_path);
    assert!(accepted_frame(&owner, "run-2", "rev-1", &bound).is_none());
    owner["source_path"] = json!("eos/hardware/camera/other");
    assert!(accepted_frame(&owner, "run-1", "rev-1", &bound).is_none());
    owner["source_path"] = json!(bound.source_path);
    owner["sample"]["output"]["port"] = json!("other");
    assert!(accepted_frame(&owner, "run-1", "rev-1", &bound).is_none());
    owner["sample"]["output"]["port"] = json!("frame");
    owner["sample"]["output"]["payload"] = Value::Null;
    assert!(accepted_frame(&owner, "run-1", "rev-1", &bound).is_none());
  }

  #[tokio::test]
  async fn binary_packet_reader_rejects_malformed_length_and_jpeg() {
    let (mut sender, mut receiver) = tokio::net::UnixStream::pair().unwrap();
    sender.write_all(&4097u32.to_be_bytes()).await.unwrap();
    assert_eq!(
      read_vision_packet(&mut receiver).await.unwrap_err().kind(),
      std::io::ErrorKind::InvalidData
    );
    let (mut sender, mut receiver) = tokio::net::UnixStream::pair().unwrap();
    sender.write_all(&2u32.to_be_bytes()).await.unwrap();
    sender.write_all(b"{}").await.unwrap();
    sender.write_all(&4u32.to_be_bytes()).await.unwrap();
    sender.write_all(b"nope").await.unwrap();
    assert_eq!(
      read_vision_packet(&mut receiver).await.unwrap_err().kind(),
      std::io::ErrorKind::InvalidData
    );
  }

  #[tokio::test]
  async fn stopped_run_rejects_binding_route() {
    let temp = tempfile::tempdir().unwrap();
    let app = Arc::new(App {
      paths: crate::app::config::Paths {
        config: temp.path().join("config"),
        data: temp.path().join("data"),
        runtime: temp.path().join("run"),
      },
      deployment: "default".into(),
    });
    app.paths.prepare().unwrap();
    let router = super::super::router(app);
    let request = Request::builder()
      .uri("/api/isos/v1/vision/bindings")
      .header(header::HOST, "127.0.0.1:16000")
      .body(Body::empty())
      .unwrap();
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  }
}
