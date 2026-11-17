//! Live, read-only owner status projection for the browser.
use super::*;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures_util::StreamExt;
use std::time::Duration;

const STATUS_INTERVAL: Duration = Duration::from_millis(200);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

pub(crate) async fn websocket(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
  authorized(&app, &headers)?;
  Ok(
    upgrade
      .max_message_size(1024)
      .on_upgrade(move |socket| relay(socket, app)),
  )
}

async fn snapshot(app: &App) -> Value {
  let paths = app.paths.clone();
  let deployment = app.deployment.clone();
  let status = tokio::task::spawn_blocking(move || {
    crate::runtime::manager::manager_status_for_poll(&paths, &deployment)
  });
  let paths = app.paths.clone();
  let health = tokio::task::spawn_blocking(move || {
    crate::runtime::owners::owner_services::health_report(&paths)
  });
  let (status, health) = tokio::join!(status, health);
  match (status, health) {
    (Ok(Ok(process_status)), Ok(owners)) => json!({
      "type":"snapshot",
      "process_status":process_status,
      "system_health":{
        "service":"isos",
        "api_schema_version":"0.1.0",
        "state":"ready",
        "owners":owners["owners"]
      }
    }),
    (status, health) => {
      let mut unavailable = Vec::new();
      if !matches!(status, Ok(Ok(_))) {
        unavailable.push("process_status");
      }
      if health.is_err() {
        unavailable.push("system_health");
      }
      json!({"type":"error","error":{"code":"status_unavailable","unavailable":unavailable}})
    }
  }
}

async fn relay(mut socket: WebSocket, app: Arc<App>) {
  let mut interval = tokio::time::interval(STATUS_INTERVAL);
  interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
  let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
  heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
  // The initial interval tick is immediate. The first heartbeat follows it.
  heartbeat.tick().await;
  let mut last: Option<Value> = None;
  let mut sequence = 0_u64;
  loop {
    tokio::select! {
      biased;
      incoming = socket.next() => match incoming {
        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
        // This is a read-only stream; client data has no control meaning.
        Some(Ok(_)) => {},
      },
      _ = interval.tick() => {
        let next = snapshot(&app).await;
        if last.as_ref() != Some(&next) {
          sequence += 1;
          let mut event = next.clone();
          event["sequence"] = json!(sequence);
          if socket.send(Message::Text(event.to_string().into())).await.is_err() { break; }
          last = Some(next);
        }
      },
      _ = heartbeat.tick() => {
        if socket.send(Message::Ping(Vec::new().into())).await.is_err() { break; }
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures_util::StreamExt;
  use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::header::HOST};

  #[tokio::test]
  async fn idle_stream_reports_failure_once_without_repeated_messages() {
    let temp = tempfile::tempdir().unwrap();
    let paths = crate::app::config::Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let app = Arc::new(App {
      paths,
      deployment: "default".into(),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server =
      tokio::spawn(async move { axum::serve(listener, super::super::router(app)).await });
    let mut request = format!("ws://127.0.0.1:{port}/api/isos/v1/runtime/status/ws")
      .into_client_request()
      .unwrap();
    request.headers_mut().insert(
      HOST,
      format!("127.0.0.1:{}", crate::app::server::local_port().unwrap())
        .parse()
        .unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let first = tokio::time::timeout(Duration::from_secs(2), socket.next())
      .await
      .unwrap()
      .unwrap()
      .unwrap();
    let event: Value = serde_json::from_str(first.to_text().unwrap()).unwrap();
    assert_eq!(event["type"], "error");
    assert_eq!(event["sequence"], 1);
    assert_eq!(event["error"]["code"], "status_unavailable");
    assert!(
      tokio::time::timeout(Duration::from_millis(600), socket.next())
        .await
        .is_err()
    );
    socket.close(None).await.unwrap();
    server.abort();
  }
}
