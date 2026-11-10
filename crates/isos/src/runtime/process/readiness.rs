//! Human-facing readiness for configured core executables.

use crate::{
  app::config::Paths, runtime::process::core_bins, runtime::owners::core_setup,
  domain::graph::generated_v2, domain::graph::graph_contract, app::isosui, runtime::process,
  runtime::process::reader_features,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn process_present(graph: &Value, id: &str) -> bool {
  graph
    .pointer(&format!(
      "/cores/isos/layers/process/groups/component/nodes/{id}"
    ))
    .is_some_and(|node| node["enabled"] != false)
}

fn owner_layers_present(graph: &Value, core: &str) -> bool {
  if graph["cores"].get(core).is_some() {
    return true;
  }
  graph
    .pointer("/cores/isos/layers")
    .and_then(Value::as_object)
    .is_some_and(|layers| {
      layers
        .keys()
        .any(|name| name.starts_with(&format!("{core}_")))
    })
}

fn owner_worker_running(active: Option<&Value>, core: &str) -> bool {
  active
    .and_then(|value| value.get("processes"))
    .and_then(Value::as_array)
    .is_some_and(|processes| {
      processes.iter().any(|process| {
        process["id"] == core && matches!(process["state"].as_str(), Some("running" | "ready"))
      })
    })
}

fn admission_issue(graph: &Value, selected_catalog: &anyhow::Result<Value>) -> Option<String> {
  match selected_catalog {
    Ok(catalog) => graph_contract::admission_diagnostic(graph, Some(catalog))
      .map(|diagnostic| format!("saved graph is incompatible: {diagnostic}")),
    Err(error) => Some(format!(
      "saved graph is incompatible: catalog unavailable: {error}"
    )),
  }
}

fn generation_action(code: &str) -> &'static str {
  match code {
    "generated_launch_input_mismatch" => {
      "Apply the saved graph again after rebuilding the core binaries"
    }
    "generated_revision_mismatch" => "Reload the saved graph revision",
    _ => "Apply the saved graph again to restore its generated configuration",
  }
}

fn generation_issue(
  paths: &Paths,
  name: &str,
  graph: &Value,
  revision: Option<&str>,
) -> (bool, Option<(String, String, String)>) {
  let Some(revision) = revision else {
    return (false, None);
  };
  match generated_v2::selected(paths, name, graph, revision, None) {
    Ok(None) => {
      let prior_apply = std::fs::read(paths.generated(name).join("selection.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|selection| {
          selection["entries"]
            .as_object()
            .map(|entries| !entries.is_empty())
        })
        .unwrap_or(false);
      if prior_apply {
        (
          false,
          Some((
            "pending_apply".into(),
            "The saved graph pool file changed since Apply".into(),
            "Apply the saved graph pool file before Run".into(),
          )),
        )
      } else {
        (false, None)
      }
    }
    Ok(Some(root)) if root.join("core-setup/launch-inputs.json").is_file() => (true, None),
    Ok(Some(_)) => (
      false,
      Some((
        "generated_missing_input".into(),
        "Selected generation has no core launch inputs".into(),
        "Apply the saved graph again to restore its generated configuration".into(),
      )),
    ),
    Err(issue) => (
      false,
      Some((
        issue.code.into(),
        issue.to_string(),
        generation_action(issue.code).into(),
      )),
    ),
  }
}

pub fn report(paths: &Paths, name: &str, active: Option<&Value>) -> Value {
  let graph_path = paths.node_graph_path(name);
  let saved = graph_contract::read(&graph_path).ok();
  let graph_revision = saved.as_ref().map(|saved| saved.revision.clone());
  let graph: Value = saved.map_or(Value::Null, |saved| saved.value);
  let selected_catalog = graph
    .is_object()
    .then(|| process::catalog_for_graph(paths, &graph));
  let graph_admission_issue = if graph.is_object() {
    admission_issue(
      &graph,
      selected_catalog
        .as_ref()
        .expect("graph has selected catalog"),
    )
  } else {
    None
  };
  let health_graph = graph["cores"].get("ei").is_some() && graph["cores"].get("eos").is_some();
  let projected = if graph.is_object() && !health_graph {
    process::project_with_packages(paths, &graph.to_string()).ok()
  } else {
    None
  };
  let mut reader_issues = BTreeMap::new();
  if graph.is_object() && graph_admission_issue.is_none() && !health_graph {
    if let (Some(specs), Some(Ok(catalog))) = (&projected, selected_catalog.as_ref()) {
      for core in ["eos", "ei"] {
        if let Err(issue) = reader_features::check_core(paths, &graph, specs, catalog, core) {
          reader_issues.insert(core, issue);
        }
      }
    } else if let Err(issue) = reader_features::check_setup_basics(paths, &graph) {
      let core = if issue.context["reader"]["core"] == "ei" {
        "ei"
      } else {
        "eos"
      };
      reader_issues.insert(core, issue);
    }
  }
  let endpoints = graph
    .is_object()
    .then(|| core_setup::endpoints(paths, name, &graph))
    .transpose();
  let cores: Vec<Value> = ["eos", "ei"]
        .into_iter()
        .map(|core| {
            let explicit = match core {
                "eos" => std::env::var_os("ISOS_AETHER_BIN_DIR").is_some(),
                _ => std::env::var_os("ISOS_EI_BIN_DIR").is_some(),
            };
            let path = if core == "eos" {
                core_bins::resolve("aether-profile")
            } else {
                core_setup::adapter_path(paths, core)
            };
            let process_enabled = health_graph || process_present(&graph, core);
            let enabled = process_enabled || owner_layers_present(&graph, core);
            let (generation_configured, generation_issue) = if enabled {
                generation_issue(paths, name, &graph, graph_revision.as_deref())
            } else {
                (false, None)
            };
            let graph_configured = enabled && generation_configured;
            let running = owner_worker_running(active, core);
            let running_changed = running
                && projected.as_ref().is_some_and(|specs| {
                    specs
                        .iter()
                        .find(|spec| spec.id == core)
                        .is_some_and(|spec| {
                            active
                                .and_then(|value| value.get("processes"))
                                .and_then(Value::as_array)
                                .and_then(|processes| {
                                    processes.iter().find(|process| process["id"] == core)
                                })
                                .is_some_and(|process| {
                                    process["fingerprint"] != json!(spec.fingerprint)
                                })
                        })
                });
            let (path, version, compatible, reason) = match path {
                Ok(path) => match if core == "ei" && graph.is_object() {
                    core_setup::identity_for_graph(paths, core, &graph)
                } else {
                    core_setup::identity(paths, core)
                } {
                    Ok(identity) => {
                        let supports_graph = identity["supported_graph_versions"]
                            .as_array()
                            .is_some_and(|versions| versions.iter().any(|v| v == "0.2.0"));
                        (
                            Some(path.to_string_lossy().into_owned()),
                            identity["product_version"].as_str().map(str::to_owned),
                            supports_graph,
                            (!supports_graph).then(|| format!(
                                "{core} setup does not advertise graph 0.2.0"
                            )),
                        )
                    },
                    Err(error) => (
                        Some(path.to_string_lossy().into_owned()),
                        None,
                        false,
                        Some(error.to_string()),
                    ),
                },
                Err(error) => (None, None, false, Some(error.to_string())),
            };
            let inputs_issue = if graph_configured {
                core_setup::verify_launch_inputs(paths, name)
                    .err()
                    .map(|e| e.to_string())
            } else {
                None
            };
            let endpoint_issue = endpoints.as_ref().err().map(ToString::to_string);
            let selected_issue = reader_issues.get(core);
            let ready = compatible
                && graph_configured
                && process_enabled
                && graph_admission_issue.is_none()
                && generation_issue.is_none()
                && selected_issue.is_none()
                && inputs_issue.is_none()
                && endpoint_issue.is_none()
                && !running_changed;
            let reason = if enabled {
                graph_admission_issue
                    .clone()
                    .or_else(|| (!process_enabled).then(|| format!("{core} layers require a {core} process node before Run")))
                    .or_else(|| selected_issue.map(ToString::to_string))
                    .or_else(|| generation_issue.as_ref().map(|issue| issue.1.clone()))
                    .or(reason)
                    .or(inputs_issue)
                    .or(endpoint_issue)
                    .or_else(|| {
                        running_changed.then(|| {
                            "running from an earlier configured build; restart to use the saved build"
                                .into()
                        })
                    })
            } else {
                None
            };
            let configured_endpoint = endpoints.as_ref().ok().and_then(|endpoints| {
                endpoints.as_ref().and_then(|value| match core {
                    "eos" => value.get("eos_control"),
                    "ei" => value.get("ei_northbound"),
                    _ => None,
                })
            });
            let action = if !enabled {
                "Core is not enabled in the saved graph".into()
            } else if graph_admission_issue.is_some() {
                "Use a valid 0.2.0 graph".into()
            } else if !compatible {
                format!("Build {core} in this workspace and configure its binary directory")
            } else if !process_enabled {
                format!("Add the {core} process node and Apply the graph")
            } else if let Some((_, _, action)) = &generation_issue {
                action.clone()
            } else if !graph_configured {
                "Save the graph to generate core launch inputs".into()
            } else if running_changed {
                "Restart the deployment to use the saved build".into()
            } else if reason.is_some() {
                "Apply the saved graph again, then restart".into()
            } else if running {
                "Running from the configured build".into()
            } else {
                "Run the saved graph".into()
            };
            json!({"core":core,"path":path,"version":version,"compatible":compatible,
                "configured":explicit || path.is_some(),"enabled":enabled,"graph_configured":graph_configured,
                "running":running,"ready":ready,"reason":reason,"action":action,
                "configured_endpoint":configured_endpoint,
                "diagnostics":selected_issue.into_iter().collect::<Vec<_>>(),
                "configuration_diagnostic":generation_issue.as_ref().map(|issue| json!({"code":issue.0,"message":issue.1})),
                "compatible_upgrade":null})
        })
        .collect();
  let ui = match isosui::bundle_for_serve(paths) {
    Ok(path) => json!({"path":path,"ready":true}),
    Err(error) => json!({"path":null,"ready":false,"reason":error.to_string(),
            "action":"Build IsosUI under ui/ and configure ISOSUI_DIST_DIR"}),
  };
  json!({"cores":cores,"graph":{"path":graph_path,"configured":graph.is_object(),"version":graph["version"]},
        "isos":{"version":env!("CARGO_PKG_VERSION"),"path":std::env::current_exe().ok()},
        "isosui":ui})
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ready_owner_worker_is_running_for_readiness() {
    let active = json!({"processes":[
      {"id":"eos","state":"ready","worker_pid":123},
      {"id":"ei","state":"running","worker_pid":124}
    ]});
    assert!(owner_worker_running(Some(&active), "eos"));
    assert!(owner_worker_running(Some(&active), "ei"));
    assert!(!owner_worker_running(Some(&active), "isos"));
    assert!(!owner_worker_running(None, "eos"));
  }

  #[test]
  fn failed_owner_worker_is_not_running_for_readiness() {
    let active = json!({"processes":[{"id":"eos","state":"failed"}]});
    assert!(!owner_worker_running(Some(&active), "eos"));
  }

  #[test]
  fn three_core_wrapper_catalog_is_used_for_readiness_admission() {
    let graph = json!({"version":"0.2.0","cores":{
      "isos":{"layers":{}},
      "ei":{"layers":{"cognition":{"groups":{"context":{"nodes":{
        "builder":{"id":"builder","node_type":"ei.cognition.context.builder@v0.2"}
      }}}}}},
      "eos":{"layers":{}}
    }});
    let mut catalog = json!({"types":[]});
    assert!(
      admission_issue(&graph, &Ok(catalog.clone()))
        .unwrap()
        .contains("builder/node_type")
    );
    catalog["types"].as_array_mut().unwrap().push(json!({
      "id":"ei.cognition.context.builder@v0.2",
      "core":"ei","layer":"cognition","group":"context"
    }));
    assert!(admission_issue(&graph, &Ok(catalog)).is_none());
  }

  #[test]
  fn disabled_process_is_not_an_enabled_core() {
    let graph = json!({"cores":{"isos":{"layers":{"process":{"groups":{"component":{"nodes":{
        "ei":{"enabled":false}
    }}}}}}}});
    assert!(!process_present(&graph, "ei"));
  }

  #[test]
  fn owner_layers_remain_visible_without_a_process_node() {
    let graph =
      json!({"version":"0.2.0","cores":{"isos":{"layers":{"ei_cognition":{"groups":{}}}}}});
    assert!(owner_layers_present(&graph, "ei"));
    assert!(!process_present(&graph, "ei"));
    assert!(!owner_layers_present(&graph, "eos"));
  }

  #[test]
  fn unsupported_graph_version_takes_priority_over_changed_binary_guidance() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    crate::app::config::atomic_write(
            &paths.node_graph_path("default"),
            br#"{"version":"2.0.0","robot":{},"cores":{"isos":{"layers":{"process":{"groups":{"component":{"nodes":{"eos":{"id":"eos","node_type":"isos.process.component@v0.2","settings":{"role":"eos","args":[]}}}}}}}}}}"#,
        )
        .unwrap();
    let status = report(&paths, "default", None);
    let eos = &status["cores"][0];
    assert!(eos["reason"].as_str().unwrap().contains("incompatible"));
    assert!(
      eos["action"]
        .as_str()
        .unwrap()
        .contains("valid 0.2.0 graph")
    );
    assert!(status["cores"][1]["reason"].is_null());
  }

  #[test]
  fn new_graph_is_not_reported_as_an_unknown_version() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let graph = crate::domain::graph::starter::draft(&paths, "default").unwrap();
    crate::app::config::atomic_write(
      &paths.node_graph_path("default"),
      &serde_json::to_vec_pretty(&graph).unwrap(),
    )
    .unwrap();
    let status = report(&paths, "default", None);
    assert_eq!(status["graph"]["configured"], true);
    assert_eq!(status["graph"]["version"], "0.2.0");
  }

  #[test]
  fn unapplied_graph_has_no_generation_error() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let graph = crate::domain::graph::starter::draft(&paths, "default").unwrap();
    let bytes = serde_json::to_vec_pretty(&graph).unwrap();
    crate::app::config::atomic_write(&paths.node_graph_path("default"), &bytes).unwrap();
    let saved = graph_contract::read(&paths.node_graph_path("default")).unwrap();
    assert_eq!(
      generation_issue(&paths, "default", &saved.value, Some(&saved.revision)),
      (false, None)
    );
  }

  #[test]
  fn edited_pool_file_after_prior_apply_reports_pending_apply() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let graph = crate::domain::graph::starter::draft(&paths, "default").unwrap();
    let bytes = serde_json::to_vec_pretty(&graph).unwrap();
    crate::app::config::atomic_write(&paths.node_graph_path("default"), &bytes).unwrap();
    let prior = "a".repeat(64);
    let generated = paths.generated("default");
    std::fs::create_dir_all(&generated).unwrap();
    let mut entries = serde_json::Map::new();
    entries.insert(prior, json!({"kind":"g03"}));
    std::fs::write(
      generated.join("selection.json"),
      serde_json::to_vec(&json!({"schema_version":2,"entries":entries})).unwrap(),
    )
    .unwrap();
    let saved = graph_contract::read(&paths.node_graph_path("default")).unwrap();
    let (configured, issue) =
      generation_issue(&paths, "default", &saved.value, Some(&saved.revision));
    assert!(!configured);
    assert_eq!(issue.unwrap().0, "pending_apply");
  }

  #[test]
  fn broken_selected_generation_is_reported() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let graph = crate::domain::graph::starter::draft(&paths, "default").unwrap();
    let bytes = serde_json::to_vec_pretty(&graph).unwrap();
    crate::app::config::atomic_write(&paths.node_graph_path("default"), &bytes).unwrap();
    let saved = graph_contract::read(&paths.node_graph_path("default")).unwrap();
    let generated = paths.generated("default");
    std::fs::create_dir_all(&generated).unwrap();
    std::fs::write(generated.join("selection.json"), b"not json").unwrap();
    let (configured, issue) =
      generation_issue(&paths, "default", &saved.value, Some(&saved.revision));
    assert!(!configured);
    let issue = issue.expect("invalid selection must be reported");
    assert_eq!(issue.0, "generated_manifest_mismatch");
    assert!(issue.2.contains("Apply the saved graph"));
  }
}
