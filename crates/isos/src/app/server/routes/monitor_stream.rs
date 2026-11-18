//! Read-only mock arm monitor projection for the browser.
use super::*;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures_util::StreamExt;
use std::time::Duration;

pub(crate) async fn websocket(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  axum::extract::Query(query): axum::extract::Query<std::collections::BTreeMap<String, String>>,
  upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
  authorized(&app, &headers)?;
  let hz = query
    .get("refresh_hz")
    .map_or(Ok(5_u64), |value| value.parse::<u64>());
  let hz = hz.map_err(|_| err(StatusCode::BAD_REQUEST, "refresh_hz must be 1–5"))?;
  if !(1..=5).contains(&hz) {
    return Err(err(StatusCode::BAD_REQUEST, "refresh_hz must be 1–5"));
  }
  Ok(
    upgrade
      .max_message_size(1024)
      .on_upgrade(move |socket| relay(socket, app, headers, hz)),
  )
}

async fn relay(mut socket: WebSocket, app: Arc<App>, headers: HeaderMap, hz: u64) {
  let mut interval = tokio::time::interval(Duration::from_millis(1000 / hz));
  interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
  let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
  heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
  heartbeat.tick().await;
  let mut last: Option<Value> = None;
  loop {
    tokio::select! {
      biased;
      incoming = socket.next() => match incoming {
        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
        Some(Ok(_)) => {},
      },
      _ = interval.tick() => {
        let event = match super::nodes::node_monitor(State(app.clone()), headers.clone()).await {
          Ok(Json(envelope)) => json!({"type":"snapshot","snapshot":envelope["data"]}),
          Err((status, Json(envelope))) => json!({"type":"error","status":status.as_u16(),"error":envelope["error"]}),
        };
        if last.as_ref() != Some(&event) {
          if socket.send(Message::Text(event.to_string().into())).await.is_err() { break; }
          last = Some(event);
        }
      }
      _ = heartbeat.tick() => { if socket.send(Message::Ping(Vec::new().into())).await.is_err() { break; } }
    }
  }
}
