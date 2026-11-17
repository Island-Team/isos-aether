//! Domain routes exposed by the loopback API.
pub(super) mod catalog;
pub(super) mod graph;
pub(super) mod graph_vision;
pub(super) mod graph_pool;
pub(super) mod log_stream;
pub(super) mod monitor_stream;
pub(super) mod nodes;
pub(super) mod owner_control;
pub(super) mod package_handlers;
pub(super) mod presentations;
pub(super) mod runtime;
pub(super) mod status_stream;
pub(super) mod system;
use system::{health, ui_config, version};
use nodes::{external_node, node_event, node_event_result, node_monitor};
use super::*;

pub(super) fn router(app: Arc<App>) -> Router {
  let api = Router::new()
    .route("/api/isos/v1/system/health", get(health).options(preflight))
    .route(
      "/api/isos/v1/system/ui-config",
      get(ui_config).options(preflight),
    )
    .route(
      "/api/isos/v1/system/version",
      get(version).options(preflight),
    )
    .route(
      "/api/isos/v1/catalog/node-types",
      get(node_types).options(preflight),
    )
    .route(
      "/api/isos/v1/catalog/node-types/selected",
      get(selected_node_types).options(preflight),
    )
    .route(
      "/api/isos/v1/catalog/node-options",
      get(node_options_current)
        .post(node_options_for_graph)
        .options(preflight),
    )
    .route(
      "/api/isos/v1/catalog/node-sets",
      get(package_node_sets).options(preflight),
    )
    .route(
      "/api/isos/v1/catalog/node-sets/preview",
      post(preview_node_set).options(preflight),
    )
    .route(
      "/api/isos/v1/runtime/process-status",
      get(process_status).options(preflight),
    )
    .route(
      "/api/isos/v1/runtime/status/ws",
      get(status_stream::websocket),
    )
    .route("/api/isos/v1/owners/eos/control", owner_control::route())
    .route(
      "/api/isos/v1/vision/bindings",
      get(graph_vision::bindings).options(preflight),
    )
    .route("/api/isos/v1/vision/ws", get(graph_vision::websocket))
    .route(
      "/api/isos/v1/nodes/events",
      post(node_event).options(preflight),
    )
    .route(
      "/api/isos/v1/nodes/events/result",
      post(node_event_result).options(preflight),
    )
    .route(
      "/api/isos/v1/nodes/monitor",
      get(node_monitor).options(preflight),
    )
    .route(
      "/api/isos/v1/nodes/monitor/ws",
      get(monitor_stream::websocket),
    )
    .route(
      "/api/isos/v1/nodes/external",
      post(external_node).options(preflight),
    )
    .route(
      "/api/isos/v1/presentations/select",
      post(select_presentation).options(preflight),
    )
    .route(
      "/api/isos/v1/packages",
      get(package_list).options(preflight),
    )
    .route(
      "/api/isos/v1/packages/config",
      get(package_handlers::package_config_get)
        .put(package_handlers::package_config_put)
        .options(preflight),
    )
    .route(
      "/api/isos/v1/packages/config/validate",
      post(package_handlers::package_config_validate).options(preflight),
    )
    .route(
      "/api/isos/v1/packages/install",
      post(package_install).options(preflight),
    )
    .route(
      "/api/isos/v1/packages/preview-download",
      post(package_preview_download).options(preflight),
    )
    .route(
      "/api/isos/v1/packages/link",
      post(package_link).options(preflight),
    )
    .route(
      "/api/isos/v1/packages/preview-link",
      post(preview_link).options(preflight),
    )
    .route(
      "/api/isos/v1/cores/readiness",
      get(core_readiness).options(preflight),
    )
    .route(
      "/api/isos/v1/cores/setup-status",
      get(setup_status).options(preflight),
    )
    .route(
      "/api/isos/v1/runtime/control",
      post(process_control).options(preflight),
    )
    .route("/api/isos/v1/runtime/logs", get(logs).options(preflight))
    .route("/api/isos/v1/runtime/logs/ws", get(log_stream::websocket))
    .route(
      "/api/isos/v1/graph",
      get(node_graph).put(apply_node_graph).options(preflight),
    )
    .route(
      "/api/isos/v1/graph/new",
      get(new_node_graph).options(preflight),
    )
    .route(
      "/api/isos/v1/graph/convert",
      get(graph::convert_node_graph).options(preflight),
    )
    .route(
      "/api/isos/v1/graph-pool",
      get(graph_pool::list).options(preflight),
    )
    .route(
      "/api/isos/v1/graph-pool/{name}",
      get(graph_pool::get).put(graph_pool::put).options(preflight),
    )
    .route(
      "/api/isos/v1/graph/validate",
      post(validate_node_graph).options(preflight),
    )
    .route(
      "/api/isos/v1/graph/connection",
      post(graph::validate_connection).options(preflight),
    )
    .route(
      "/api/isos/v1/cores/setup/{role}/{op}",
      get(core_setup_route)
        .post(core_setup_route)
        .options(preflight),
    )
    .route_layer(middleware::from_fn(cors));
  Router::new().merge(api).with_state(app)
}
