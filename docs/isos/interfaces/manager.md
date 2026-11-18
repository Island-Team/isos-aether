<a id="manager"></a>
<a id="manager--owner-implementation-notes"></a>
# Owner implementation notes

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Interfaces for nodes, packages, cores, and IsosUI](README.md)
  - [Shared contract catalog](contract-catalog.md)
  - [Core owner contracts](core.md)
- Subfolders:
  - [api/](api/README.md)
  - [node/](node/README.md)
  - [package/](package/README.md)
  - [server/](server/README.md)

</details>
<!-- doc-navigation:end -->

This page is for implementers of the IsOS local manager and its configured services. These are internal control surfaces; package and core publishers should use the [external core contracts](core.md) for handoff and stop.

- [Manager](manager.md#manager_protocol) defines the local operation IPC and revision-checked mutation envelope.
- [Services](../core/service.md) defines the manager TOML entries that select local owners and launch children.
- [Log cursors](manager.md#manager_log_cursors) defines run-scoped log pagination and ordering for diagnostics.

[中文](../../../docs_cn/isos/interfaces/manager_cn.md)

<a id="manager_log_cursors"></a>
<a id="manager_log_cursors--current-run-log-cursors"></a>
## Current-run log cursors

[中文](../../../docs_cn/isos/interfaces/manager_cn.md#manager_log_cursors) · [Local core logs](../core/logger.md)

The log API reads IsOS's manager JSONL and its bounded EOS and EI owner log pools, then merges accepted records by timestamp. EOS and EI provide a filtered owner page over their private sockets; IsOS stores selected records before serving UI readers. A continuation cursor is opaque to clients and signed by IsOS. It binds the source filter, run identity, file identities, and offsets; a forged token is rejected. A changed run or replaced file produces a gap diagnostic and starts at a safe boundary. Logs are observations, not proof of worker health; use deployment status for that.

| Field | Type / unit | Rule |
| --- | --- | --- |
| `cursor` | opaque string / continuation | Optional on the first page; bound to the filters and current run. |
| `limit` | integer / records | 1–200 per page. |
| `source` | string / core | `all`, `isos`, `eos`, or `ei`. |
| `level` | string / severity | `trace`, `debug`, `info`, `warn`, or `error`. |
| `node_path` | string / graph path | Optional exact match. |
| `correlation_id` | string / operation | Optional exact match. |

IsOS reads `data/isos/log/deployments/<deployment>/isos.jsonl` and `owner-pool/eos.jsonl` and `owner-pool/ei.jsonl` below the same deployment directory. The owner pools persist independently of a browser session and let a new UI connection read earlier startup, Run, warning, and error records. IsOS uses bounded complete lines and rejects symlinks, malformed JSON, oversized records, and records whose source core does not match the pool. EOS and EI retain their local JSONL files as replay sources for owner reattachment. Launcher and install records use the same JSONL envelope in their own IsOS files; see the [logging guide](../core/logger.md).

A development manager may replace its own prior session diagnostics after a confirmed Stop. It retains both owner pools, the `owner-pool/{eos,ei}.cursor` source offsets, and owner-local JSONL so a reconnect continues without losing EOS or EI history. The UI still treats an IsOS cursor scoped to a replaced manager file as rotated and requests a new page.

In an idle `manager_session` log view, IsOS includes retained EOS and EI pool records from earlier manager sessions, even when their `run_id` differs from the current manager session ID. IsOS's own records stay bound to the current manager session. Once a deployment run is selected, its `deployment_run` view includes only records for that exact run ID from all three sources. A source or severity filter applies inside either scope; neither an old owner record nor a visible log line grants run control.

<a id="manager_protocol"></a>
<a id="manager_protocol--manager-ipc-v1"></a>
## Manager IPC v1

[中文](../../../docs_cn/isos/interfaces/manager_cn.md#manager_protocol) · [Boundary index](manager.md) · [Runtime HTTP](api/http.md#http_runtime)

<a id="manager_protocol--overview"></a>
### Overview

The IsOS CLI and loopback HTTP server call the IsOS-owned manager over an owner-only Unix socket. This wire is `isos.manager@v1`; it is separate from HTTP schema, graph version, and owner setup. The manager serializes deployment mutations.

Start and restart validate the saved `0.2.1` graph and its exact byte revision before preparing a launch. The resulting deployment plan always uses managed admission, including control grants and owner readiness checks.

<a id="manager_protocol--authentication-and-transport"></a>
### Authentication and transport

The socket checks same-UID peer identity. A request is one bounded newline-delimited JSON object with a deadline; an oversized, malformed, timed-out, or unknown operation fails before dispatch. The socket is private and must never be projected into a browser endpoint.

<a id="manager_protocol--interfaces"></a>
### Interfaces

The request has `schema_version:"0.1.0"`, snake-case `op`, and the payload belonging to that operation. Extra fields in a typed payload are rejected. Example stop request:

```json
{
  "schema_version": "0.1.0",
  "op": "deployment_stop",
  "payload": {
    "deployment": "default",
    "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  }
}
```

The response has `schema_version:"0.1.0"`, `ok`, operation-specific `data`, and optional `error` with stable `code`, safe `message`, and `correlation_id`. Example failure:

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "revision_conflict",
    "message": "request conflicts with current state",
    "correlation_id": "req-42"
  }
}
```

| Field | Type / unit | Purpose and validation |
| --- | --- | --- |
| `schema_version` | string / version | Required, exactly `0.1.0`. |
| `op` | closed snake-case string / none | Selects one operation and payload type; unknown values reject. |
| `payload.deployment` | slug string / none | Selects a manager-owned deployment; required for deployment and append operations. |
| `payload.revision` | optional string / SHA-256 | Exact saved-byte fence for deployment mutations; absence accepts the current saved revision. |
| `payload.service_id` | string / none | Required only for service target/log operations. |
| `payload.cursor` | optional opaque string / none | Service log continuation; clients do not decode it. |
| `payload.limit` | optional unsigned integer / records | Bounds a service log read. |


<a id="manager_protocol--operations"></a>
### Operations

No-payload operations: `manager_identity`, `manager_shutdown`, `services_list`, `services_validate`, and `services_status`. Deployment operations: `deployment_start`, `deployment_restart`, `deployment_stop` use `deployment` and optional `revision`; `deployment_status` and `deployment_active_packages` use `deployment` only. Service mutations: `services_start`, `services_stop`, `services_restart` use `service_id`; `services_logs` adds optional `cursor` and `limit`. A cross-operation field such as `service_id` in `deployment_stop` is invalid. A stop is confirmed only by owner proof; see [dependent stop](core.md#core_dependent_stop).
