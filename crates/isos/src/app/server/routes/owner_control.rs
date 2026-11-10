//! Bounded browser projection of the EOS public bridge. EOS remains the command admission owner.
use super::*;
use axum::{body::Bytes, extract::DefaultBodyLimit};
use std::time::Duration;

const MAX_REQUEST_BYTES: usize = 20_480;
const MAX_REPLY_BYTES: usize = 65_536;

pub(super) fn route() -> axum::routing::MethodRouter<Arc<App>> {
  post(eos_control)
    .options(preflight)
    .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
}

fn admitted_op(value: &Value) -> bool {
  matches!(
    value.get("op").and_then(Value::as_str),
    Some(
      "discover"
        | "status"
        | "health"
        | "observe"
        | "time"
        | "result"
        | "primitive"
        | "primitive_typed"
        | "manual"
    )
  ) && value.as_object().is_some_and(|object| {
    !object.contains_key("grant")
      && !object.contains_key("token")
      && !object.contains_key("endpoint")
  })
}

pub(super) async fn active_bridge_port(app: &App) -> Result<u64, (StatusCode, Json<Value>)> {
  let paths = app.paths.clone();
  let deployment = app.deployment.clone();
  let status = tokio::task::spawn_blocking(move || {
    crate::runtime::manager::manager_status_for_http(&paths, &deployment)
  })
  .await
  .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?
  .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if status["state"] != "running"
    || status["run_id_state"] != "known"
    || status["active_revision"].as_str().is_none()
    || !status["processes"].as_array().is_some_and(|items| {
      items
        .iter()
        .any(|item| item["id"] == "bridge" && item["state"] == "running")
    })
  {
    return Err(fail(
      StatusCode::SERVICE_UNAVAILABLE,
      "owner_bridge_unavailable",
      "EOS bridge is unavailable",
      Value::Null,
    ));
  }
  let graph_bytes = fs::read(app.paths.node_graph_path(&app.deployment))
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if hex::encode(Sha256::digest(&graph_bytes))
    != status["active_revision"].as_str().unwrap_or_default()
  {
    return Err(fail(
      StatusCode::CONFLICT,
      "run_fence_stale",
      "Active graph revision is stale",
      Value::Null,
    ));
  }
  let graph: Value = serde_json::from_slice(&graph_bytes)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  let endpoints = core_setup::endpoints(&app.paths, &app.deployment, &graph)
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  let port = endpoints["eos_control"]["port"]
    .as_u64()
    .filter(|port| (1024..=65535).contains(port))
    .ok_or_else(|| {
      err(
        StatusCode::SERVICE_UNAVAILABLE,
        "EOS bridge endpoint is unavailable",
      )
    })?;
  Ok(port)
}

pub(crate) async fn eos_control(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  body: Bytes,
) -> ApiResult {
  authorized(&app, &headers)?;
  if body.len() > MAX_REQUEST_BYTES {
    return Err(fail(
      StatusCode::PAYLOAD_TOO_LARGE,
      "owner_request_oversized",
      "EOS request exceeds size limit",
      Value::Null,
    ));
  }
  let command: Value = serde_json::from_slice(&body)
    .map_err(|_| err(StatusCode::BAD_REQUEST, "EOS request must be a JSON object"))?;
  if !admitted_op(&command) {
    return Err(fail(
      StatusCode::BAD_REQUEST,
      "owner_operation_invalid",
      "EOS public operation is invalid",
      Value::Null,
    ));
  }
  let port = active_bridge_port(&app).await?;
  let client = reqwest::Client::builder()
    .connect_timeout(Duration::from_millis(250))
    .timeout(Duration::from_secs(4))
    .build()
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  let response = client
    .post(format!("http://127.0.0.1:{port}/api/eos/v2/control"))
    .json(&command)
    .send()
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  let owner_status = response.status();
  let reply = response
    .bytes()
    .await
    .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
  if reply.len() > MAX_REPLY_BYTES {
    return Err(fail(
      StatusCode::BAD_GATEWAY,
      "owner_reply_oversized",
      "EOS reply exceeds size limit",
      Value::Null,
    ));
  }
  let value: Value = serde_json::from_slice(&reply).map_err(|_| {
    fail(
      StatusCode::BAD_GATEWAY,
      "owner_reply_invalid",
      "EOS reply is invalid",
      Value::Null,
    )
  })?;
  if !owner_status.is_success() || value["ok"] != true {
    let message = value["error"]
      .as_str()
      .unwrap_or("EOS rejected the request");
    return Err(fail(
      StatusCode::UNPROCESSABLE_ENTITY,
      "owner_command_rejected",
      message.chars().take(256).collect::<String>(),
      Value::Null,
    ));
  }
  ok(value)
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn public_operations_exclude_grants_and_arbitrary_targets() {
    assert!(admitted_op(&json!({"op":"discover"})));
    assert!(admitted_op(&json!({"op":"primitive_typed","header":{}})));
    assert!(!admitted_op(&json!({"op":"shutdown"})));
    assert!(!admitted_op(
      &json!({"op":"manual","endpoint":"127.0.0.1:9"})
    ));
    assert!(!admitted_op(&json!({"op":"time","grant":"secret"})));
  }
}
