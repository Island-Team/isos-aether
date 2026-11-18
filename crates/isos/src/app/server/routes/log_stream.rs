//! Current-scope log pages delivered over a read-only WebSocket.
use super::*;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures_util::StreamExt;
use std::{collections::BTreeMap, time::Duration};

pub(crate) async fn websocket(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  axum::extract::Query(query): axum::extract::Query<BTreeMap<String, String>>,
  upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
  authorized(&app, &headers)?;
  Ok(
    upgrade
      .max_message_size(1024)
      .on_upgrade(move |socket| relay(socket, app, headers, query)),
  )
}

async fn relay(
  mut socket: WebSocket,
  app: Arc<App>,
  headers: HeaderMap,
  mut query: BTreeMap<String, String>,
) {
  query.insert("view".into(), "current".into());
  let mut interval = tokio::time::interval(Duration::from_millis(200));
  interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
  let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
  heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
  heartbeat.tick().await;
  let mut last_scope: Option<Value> = None;
  let mut last_anchor: Option<Value> = None;
  let mut last_error: Option<Value> = None;
  loop {
    tokio::select! {
      biased;
      incoming = socket.next() => match incoming {
        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
        Some(Ok(_)) => {},
      },
      _ = interval.tick() => {
        let response = super::runtime::logs(State(app.clone()), headers.clone(), axum::extract::Query(query.clone())).await;
        match response {
          Ok(Json(envelope)) => {
            let page = &envelope["data"];
            if let Some(cursor) = page["next_cursor"].as_str() { query.insert("cursor".into(), cursor.into()); }
            let scope_changed = last_scope.as_ref() != Some(&page["scope"]);
            let anchor_changed = last_anchor.as_ref() != Some(&page["anchor"]);
            let meaningful = page["lines"].as_array().is_some_and(|rows| !rows.is_empty())
              || page["diagnostics"].as_array().is_some_and(|rows| !rows.is_empty());
            // Send a new scope/anchor even when it has no rows.
            if meaningful || scope_changed || anchor_changed {
              if socket.send(Message::Text(json!({"type":"page","page":page}).to_string().into())).await.is_err() { break; }
              last_scope = Some(page["scope"].clone());
              last_anchor = Some(page["anchor"].clone());
              last_error = None;
            }
          }
          Err((status, Json(error))) => {
            if last_error.as_ref() != Some(&error) {
              if socket.send(Message::Text(json!({"type":"error","status":status.as_u16(),"error":error["error"]}).to_string().into())).await.is_err() { break; }
              last_error = Some(error);
            }
            // A bad filter cannot recover on this connection.
            if status == StatusCode::BAD_REQUEST { break; }
          }
        }
      }
      _ = heartbeat.tick() => { if socket.send(Message::Ping(Vec::new().into())).await.is_err() { break; } }
    }
  }
}
