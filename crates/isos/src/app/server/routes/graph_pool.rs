//! Named graph drafts. Pool entries never change the active deployment.
use super::*;
use std::path::Path;

const MAX_GRAPH_BYTES: usize = 1024 * 1024;
const MAX_GRAPHS: usize = 128;

fn editable_shape(graph: &Value) -> bool {
  graph["version"] == "0.2.0"
    && graph["robot"]["id"].is_string()
    && graph["robot"]["name"].is_string()
    && graph["cores"]["isos"]["layers"]
      .as_object()
      .is_some_and(|layers| {
        layers.values().all(|layer| {
          layer["groups"].as_object().is_some_and(|groups| {
            groups.values().all(|group| {
              group["nodes"].as_object().is_some_and(|nodes| {
                nodes
                  .values()
                  .all(|node| node["id"].is_string() && node["node_type"].is_string())
              })
            })
          })
        })
      })
}

#[derive(Deserialize)]
pub(crate) struct PoolEdit {
  schema_version: Value,
  revision: String,
  graph: Value,
}

fn pool_path(app: &App, name: &str) -> ApiResultPath {
  process::validate_deployment_name(name)
    .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid graph pool name"))?;
  Ok(
    app
      .paths
      .config
      .join("graph-pool")
      .join(format!("{name}.nodes.json")),
  )
}

type ApiResultPath = std::result::Result<PathBuf, (StatusCode, Json<Value>)>;

fn checked_pool_dir(path: &Path) -> ApiResultPath {
  match fs::symlink_metadata(path) {
    Ok(metadata) if metadata.file_type().is_dir() => Ok(path.to_path_buf()),
    Ok(_) => Err(err(
      StatusCode::UNPROCESSABLE_ENTITY,
      "graph pool directory is unsafe",
    )),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path.to_path_buf()),
    Err(error) => Err(internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error)),
  }
}

fn read_entry(path: &Path) -> std::result::Result<Option<Vec<u8>>, (StatusCode, Json<Value>)> {
  match fs::symlink_metadata(path) {
    Ok(metadata) if !metadata.file_type().is_file() => Err(err(
      StatusCode::UNPROCESSABLE_ENTITY,
      "graph pool entry is unsafe",
    )),
    Ok(_) => fs::read(path)
      .map(Some)
      .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error)),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(error) => Err(internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error)),
  }
}

pub(crate) async fn list(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
  authorized(&app, &headers)?;
  graph::graph_work(move || {
    let dir = checked_pool_dir(&app.paths.config.join("graph-pool"))?;
    let mut graphs = Vec::new();
    if dir.exists() {
      for entry in fs::read_dir(dir).map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))? {
        let entry = entry.map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
        let filename = entry.file_name().to_string_lossy().into_owned();
        let Some(name) = filename.strip_suffix(".nodes.json") else { continue };
        if process::validate_deployment_name(name).is_err() { continue; }
        let bytes = read_entry(&entry.path())?.ok_or_else(|| err(StatusCode::NOT_FOUND, "graph pool entry disappeared"))?;
        if bytes.len() > MAX_GRAPH_BYTES { return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "graph pool entry is too large")); }
        let graph: Value = serde_json::from_slice(&bytes)
          .map_err(|_| err(StatusCode::UNPROCESSABLE_ENTITY, "graph pool entry is invalid"))?;
        graphs.push(json!({"name":name,"revision":graph_contract::revision(&bytes),"version":graph["version"]}));
        if graphs.len() > MAX_GRAPHS { return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "graph pool is full")); }
      }
    }
    graphs.sort_by(|a,b| a["name"].as_str().cmp(&b["name"].as_str()));
    ok(json!({"graphs":graphs}))
  }).await
}

pub(crate) async fn get(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  UrlPath(name): UrlPath<String>,
) -> ApiResult {
  authorized(&app, &headers)?;
  graph::graph_work(move || {
    let path = pool_path(&app, &name)?;
    checked_pool_dir(path.parent().expect("pool path has parent"))?;
    let bytes =
      read_entry(&path)?.ok_or_else(|| err(StatusCode::NOT_FOUND, "graph pool entry not found"))?;
    if bytes.len() > MAX_GRAPH_BYTES {
      return Err(err(
        StatusCode::UNPROCESSABLE_ENTITY,
        "graph pool entry is too large",
      ));
    }
    let graph: Value = serde_json::from_slice(&bytes).map_err(|_| {
      err(
        StatusCode::UNPROCESSABLE_ENTITY,
        "graph pool entry is invalid",
      )
    })?;
    ok(json!({"name":name,"revision":graph_contract::revision(&bytes),"graph":graph}))
  })
  .await
}

pub(crate) async fn put(
  State(app): State<Arc<App>>,
  headers: HeaderMap,
  UrlPath(name): UrlPath<String>,
  Json(edit): Json<PoolEdit>,
) -> ApiResult {
  authorized(&app, &headers)?;
  if edit.schema_version != "0.1.0" {
    return Err(err(
      StatusCode::BAD_REQUEST,
      "unsupported API schema version",
    ));
  }
  graph::graph_work(move || {
    let path = pool_path(&app, &name)?;
    checked_pool_dir(path.parent().expect("pool path has parent"))?;
    if !editable_shape(&edit.graph) {
      return Err(err(
        StatusCode::UNPROCESSABLE_ENTITY,
        "graph pool requires an editable 0.2.0 graph",
      ));
    }
    let bytes = serde_json::to_vec_pretty(&edit.graph)
      .map_err(|error| internal_err(StatusCode::UNPROCESSABLE_ENTITY, &error))?;
    if bytes.len() > MAX_GRAPH_BYTES {
      return Err(err(
        StatusCode::UNPROCESSABLE_ENTITY,
        "graph pool entry is too large",
      ));
    }
    let _lock = crate::runtime::lock::Lock::take(&app.paths)
      .map_err(|error| internal_err(StatusCode::SERVICE_UNAVAILABLE, &error))?;
    let prior = read_entry(&path)?;
    let current = prior
      .as_deref()
      .map(graph_contract::revision)
      .unwrap_or_default();
    if current != edit.revision {
      return Err(err(StatusCode::CONFLICT, "graph pool revision changed"));
    }
    if prior.is_none() {
      let count = fs::read_dir(path.parent().expect("pool path has parent"))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".nodes.json"))
        .count();
      if count >= MAX_GRAPHS {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "graph pool is full"));
      }
    }
    crate::app::config::atomic_write(&path, &bytes)
      .map_err(|error| internal_err(StatusCode::INTERNAL_SERVER_ERROR, &error))?;
    ok(json!({"name":name,"revision":graph_contract::revision(&bytes),"graph":edit.graph}))
  })
  .await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn named_drafts_are_revision_fenced_and_do_not_change_active_graph() {
    let temp = tempfile::tempdir().unwrap();
    let paths = crate::app::config::Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let active = paths.node_graph_path("default");
    let active_bytes = br#"{"version":"0.2.0","robot":{"id":"active","name":"Active"},"cores":{"isos":{"layers":{}}}}"#;
    crate::app::config::atomic_write(&active, active_bytes).unwrap();
    let app = Arc::new(App {
      paths,
      deployment: "default".into(),
    });
    let graph = json!({"version":"0.2.0","robot":{"id":"robot","name":"Robot"},"cores":{"isos":{"layers":{}}}});
    let saved = put(
      State(app.clone()),
      HeaderMap::new(),
      UrlPath("draft-a".into()),
      Json(PoolEdit {
        schema_version: json!("0.1.0"),
        revision: String::new(),
        graph: graph.clone(),
      }),
    )
    .await
    .unwrap()
    .0;
    let revision = saved["data"]["revision"].as_str().unwrap();
    assert_eq!(saved["data"]["graph"], graph);
    let listed = list(State(app.clone()), HeaderMap::new()).await.unwrap().0;
    assert!(
      listed["data"]["graphs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["name"] == "draft-a")
    );
    let loaded = get(
      State(app.clone()),
      HeaderMap::new(),
      UrlPath("draft-a".into()),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(loaded["data"]["revision"], revision);
    assert_eq!(fs::read(active).unwrap(), active_bytes);
    let conflict = put(
      State(app),
      HeaderMap::new(),
      UrlPath("draft-a".into()),
      Json(PoolEdit {
        schema_version: json!("0.1.0"),
        revision: String::new(),
        graph,
      }),
    )
    .await
    .unwrap_err();
    assert_eq!(conflict.0, StatusCode::CONFLICT);
  }

  #[tokio::test]
  async fn invalid_shape_and_symlink_entry_are_rejected() {
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
    let rejected = put(
      State(app.clone()),
      HeaderMap::new(),
      UrlPath("bad".into()),
      Json(PoolEdit {
        schema_version: json!("0.1.0"),
        revision: String::new(),
        graph: json!({"version":"0.2.0"}),
      }),
    )
    .await
    .unwrap_err();
    assert_eq!(rejected.0, StatusCode::UNPROCESSABLE_ENTITY);
    let dir = app.paths.config.join("graph-pool");
    fs::create_dir_all(&dir).unwrap();
    std::os::unix::fs::symlink(temp.path().join("elsewhere"), dir.join("bad.nodes.json")).unwrap();
    let rejected = get(State(app), HeaderMap::new(), UrlPath("bad".into()))
      .await
      .unwrap_err();
    assert_eq!(rejected.0, StatusCode::UNPROCESSABLE_ENTITY);
  }
}
