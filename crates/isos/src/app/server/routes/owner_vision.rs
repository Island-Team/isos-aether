//! Vision discovery and stream projection through IsOS; EOS owns stream provenance.
use super::*;
use axum::extract::ws::{Message as BrowserMessage, WebSocket, WebSocketUpgrade};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message as OwnerMessage;

const MAX_FRAME: usize = 8 * 1024 * 1024 + 4096 + 4;

pub(crate) async fn gates(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  let port = super::owner_control::active_bridge_port(&app).await?;
  let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(4))
    .build()
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  let reply = client
    .get(format!("http://127.0.0.1:{port}/api/eos/v2/vision/gates"))
    .send()
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if !reply.status().is_success() {
    return Err(fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "owner_vision_unavailable",
      "EOS Vision discovery unavailable",
      Value::Null,
    ));
  }
  let bytes = reply
    .bytes()
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if bytes.len() > 65_536 {
    return Err(fail(
      StatusCode::BAD_GATEWAY,
      "owner_reply_oversized",
      "EOS Vision catalog exceeds size limit",
      Value::Null,
    ));
  }
  let value: Value = serde_json::from_slice(&bytes)
    .map_err(|_| err(StatusCode::BAD_GATEWAY, "EOS Vision catalog is invalid"))?;
  if !value["gates"].is_array() || !value["revision"].is_string() {
    return Err(err(
      StatusCode::BAD_GATEWAY,
      "EOS Vision catalog is invalid",
    ));
  }
  ok(value)
}

pub(crate) async fn websocket(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
  authorized(&app, &headers)?;
  let port = super::owner_control::active_bridge_port(&app).await?;
  Ok(
    upgrade
      .max_message_size(MAX_FRAME)
      .on_upgrade(move |browser| relay(browser, port)),
  )
}

async fn relay(browser: WebSocket, port: u64) {
  let endpoint = format!("ws://127.0.0.1:{port}/api/eos/v2/vision/ws");
  let Ok((owner, _)) = tokio_tungstenite::connect_async(endpoint).await else {
    return;
  };
  let (mut browser_tx, mut browser_rx) = browser.split();
  let (mut owner_tx, mut owner_rx) = owner.split();
  loop {
    tokio::select! {
      incoming = browser_rx.next() => match incoming {
        Some(Ok(BrowserMessage::Text(text))) if text.len() <= 4096 => {
          if owner_tx.send(OwnerMessage::Text(text.to_string().into())).await.is_err() { break; }
        }
        Some(Ok(BrowserMessage::Close(_))) | None | Some(Err(_)) => break,
        _ => break,
      },
      incoming = owner_rx.next() => match incoming {
        Some(Ok(OwnerMessage::Text(text))) if text.len() <= 4096 => {
          if browser_tx.send(BrowserMessage::Text(text.to_string().into())).await.is_err() { break; }
        }
        Some(Ok(OwnerMessage::Binary(bytes))) if bytes.len() <= MAX_FRAME => {
          if browser_tx.send(BrowserMessage::Binary(bytes)).await.is_err() { break; }
        }
        Some(Ok(OwnerMessage::Close(_))) | None | Some(Err(_)) => break,
        Some(Ok(OwnerMessage::Ping(_))) | Some(Ok(OwnerMessage::Pong(_))) => {},
        _ => break,
      },
    }
  }
  let _ = browser_tx.send(BrowserMessage::Close(None)).await;
  let _ = owner_tx.close().await;
}
