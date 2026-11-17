# Client interactions

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/api</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Client API and frontend interfaces](README.md)
  - [Local HTTP routes](http.md)

</details>
<!-- doc-navigation:end -->

## Table of contents

- [Button group interface](#button--button-group-interface)
- [Browser primitive control and result contract](#control--browser-primitive-control-and-result-contract)
- [Graph loading and status projection](#graph_loading--graph-loading-and-status-projection)
- [Current-run log navigation — P07 UI contract](#log_cursors--current-run-log-navigation--p07-ui-contract)
- [Monitor group interface](#monitor--monitor-group-interface)
- [Presentation descriptor boundary](#presentation--presentation-descriptor-boundary)
- [Generic output data selection](#selection--generic-output-data-selection)
- [Vision group interface](#vision--vision-group-interface)
- [Workspace subscription lifecycle](#workspace_panels--u03-workspace-panel-contract)

The shared browser transport displays the first public `error.details.diagnostics[]` path and message on a failed request, along with the API `correlation_id` as a request reference. It retains the typed error code and full structured details for component-specific handling. A `409 revision_conflict` gives refresh guidance; other conflicts retain the server's specific reason. The client does not display arbitrary response bodies.

<a id="button"></a>
<a id="button--button-group-interface"></a>
## Button group interface

[中文](../../../../docs_cn/isos/interfaces/api/client_cn.md#button) · [Node types](../node/README.md)

IsosUI sends one general activation from a registered button node to IsOS with `POST /api/isos/v1/nodes/events`. IsOS verifies local Host and Origin, the active run and saved revision, and the exact declared path through EI and EOS before forwarding to the private EI owner. The browser cannot choose an arm, joint target, owner address, or actuator grant.

For a contributed `isos.ui.external.v1` node, the workbench sends `POST /api/isos/v1/nodes/external` with `{schema_version:1,node_path,run_id,active_revision,op,port,payload}`. `op` is `input` or `read`; `read` requires a null payload. IsOS checks the active run, exact saved graph revision, selected package node instance, registered port, and JSON Schema data type before reaching the package child's private Unix socket. A read response is checked against the declared output data type. The response contains `{node_path,port,run_id,active_revision,payload}`. This interface has no graph edge delivery or actuator route.

```json
{
  "schema_version": "0.2.0",
  "source_node": "isos/ui/button/trigger",
  "output": "activation",
  "data_type": "button_activation@v0.2",
  "run_id": "run-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "payload": { "event_id": "click-1", "kind": "activate" }
}
```

All fields are required; unknown fields are rejected. `source_node` has four ASCII path segments of at most 48 bytes each, using letters, digits, `_`, or `-`. `event_id` has 1–64 ASCII letters, digits, `_`, `-`, or `.`; `run_id` uses the same grammar and at most 128 bytes. The revision is 64 lowercase hexadecimal characters. The body is at most 2048 bytes. A distinct click uses a fresh event ID; an uncertain click retains its original ID.

The graph must have one admitted path from the button through EI input, cognition, and safety, then EOS ingress, skill, dispatcher, and mock arm. IsOS sends `{core:"ei",op:"activate",run_id,revision,source_node,event_id,kind}` on the EI owner socket. A successful submission returns the event ID, command ID, and `accepted` status after EOS reserves the command. A duplicate ID returns its recorded status without another movement. The browser does not wait for arm movement in the submission request.

The public submission success wraps the EI owner result in the IsOS HTTP envelope:

```json
{
  "schema_version": "0.1.0",
  "ok": true,
  "data": {
    "ok": true,
    "run_id": "run-1",
    "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "event_id": "click-1",
    "command_id": "command-1",
    "status": "accepted"
  },
  "error": null
}
```

`POST /api/isos/v1/nodes/events/result` reads the correlated result without resubmitting movement. Its exact request is `{schema_version:"0.2.0",source_node,run_id,active_revision,event_id}` with the same identities as submission. IsOS checks the active run, saved graph, and declared path before asking EI for the result; it rechecks the run before replying. The response includes `event_id`, `command_id`, and `status`: `accepted`, `running`, `succeeded`, `failed`, `cancelled`, or `outcome_unknown`. Only `succeeded` includes six finite `joint_positions_rad` values. IsosUI reads this route until a terminal status or a bounded wait ends; a later read uses the same event ID.

```json
{
  "schema_version": "0.2.0",
  "source_node": "isos/ui/button/trigger",
  "run_id": "run-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1"
}
```

Invalid submission fields return `400 node_event_invalid`, an oversized body `413 node_event_oversized`, a stale run or graph `409 run_fence_stale` or `graph_fence_stale`, and an invalid route `422 node_route_invalid` or `node_edges_invalid`. Invalid result reads return `400 node_result_invalid`, `413 node_result_oversized`, `422 node_route_invalid`, or `503 owner_result_unavailable` or `owner_result_invalid` as appropriate. Owner transport failure or uncertain submission returns `503 owner_outcome_unknown`; query the result using the same event ID before considering another submission. Stop closes admission before worker exit.

<a id="control"></a>
<a id="control--browser-primitive-control-and-result-contract"></a>
## Browser primitive control and result contract

[中文](../../../../docs_cn/isos/interfaces/api/client_cn.md#control) · [Design](../../core/ui/webui.md) · [First use](../../core/ui/webui.md) · [EOS owner contract](../server/eos.md#control)


<a id="control--typed-boundary"></a>
### Typed boundary

The browser client sends only `POST /api/isos/v1/owners/eos/control` through the validated IsOS loopback origin configured for local or hosted builds. IsOS forwards the bounded public request to the selected EOS bridge. It uses `credentials: omit`, JSON, no Authorization header, and the existing four-second client timeout. The browser sends no `authority_grant`, private `primitive_result` or raw Dispatcher operation. IsOS applies Host and approved-Origin checks; the EOS bridge retains its own admission and supplies its own run-scoped operator grant. The client provides an operator-clicked `primitive_typed` sender and a correlated result reader for supported active bindings. A browser `op: "result"` is read-only; the bridge rewrites it to its private operation.

The following fields describe the `base.velocity` request; the active binding supplies the corresponding registered arm identity and typed payload for an arm command.

| Send field | JSON type / unit | Purpose, default and validation |
| --- | --- | --- |
| `op` | literal `primitive_typed` / — | Required exact operation; no default. |
| `header.data_type_id`, `data_type_version`, `codec` | strings / registered identities | Required `base_movement`, `1.0`, `json_v1` for this base slice; no defaults. |
| `header.primitive_name`, `primitive_version` | strings / primitive identity | Required exact values from the active binding; this slice supports `base.velocity@1.0.0`. |
| `header.source_path`, `target_path` | strings / canonical graph node paths | Required exact active binding and its one graph-edge base target; never read from an editable target input. |
| `header.active_revision` | string / saved graph SHA-256 | Required current IsOS saved and active revision, 64 lowercase hexadecimal digits; no default. |
| `header.command_id` | string / command identity | Fresh per invocation, 1–128 UTF-8 bytes and EOS Dispatcher grammar; generate before send and retain after timeout. |
| `header.sequence` | unsigned integer / command order | Positive safe integer generated for this selected binding in the current run; no default. |
| `header.deadline_monotonic_ns` | unsigned integer / nanoseconds | EOS `time` reply plus bounded deadline duration, never browser wall time; reject a non-safe JSON integer or overflow before send. |
| `header.payload_length_bytes` | unsigned integer / UTF-8 bytes | Exact length of compact JSON serialization of the parsed payload, 1–16,384; do not use JavaScript string length for multibyte data. |
| `payload` | typed object / SI units | Exactly `body_velocity` with finite `x_m_s`, `y_m_s`, `yaw_rad_s` within active hardware limits and positive integer `valid_for_ms` within the lease limit. |

The public send contains exactly `op`, `header` and `payload`; the bridge adds the private grant after public checks. On a matching EOS `{"ok":true,"result":...}` response, use the same six-status decoder as the result reader. A missing/mismatched ID, unknown status or malformed response is a protocol error; a transport failure is submission outcome unavailable. The browser never retries a possibly accepted movement. Movement remains bound to a mock or a target qualified by the EOS stop boundary; the per-binding controls cover supported base and arm bindings.

The browser reports `payload_length_bytes` from `TextEncoder(JSON.stringify(payload))` in the public request. The EOS bridge validates the declared length and parsed payload bounds, then canonicalizes the length for its private `serde_json` runtime envelope. This preserves valid small SI values whose JavaScript and Rust compact number spellings differ.

| Client field | JSON type / unit | Purpose, default and validation |
| --- | --- | --- |
| `op` | literal `result` / — | Required exact operation; no default. |
| `command_id` | string / command identity | Required exact ID from the submitted `primitive_typed` header and correlated initial EOS reply, 1–128 UTF-8 bytes; no default. |
| `active_revision` | string / saved graph SHA-256 | Required 64 lowercase hexadecimal digits copied from the current IsOS active revision; it must equal the saved graph revision at each read; no default. |
| `result.command_id` | string / command identity | Must equal the locally tracked command ID; a mismatch is a protocol error, not an outcome. |
| `result.status` | enum / — | Required exact `Completed`, `Failed`, `Cancelled`, `Rejected`, `InProgress` or `OutcomeUnknown`; unknown status is a protocol error. |
| `result.reason` | string or null / UTF-8 text | EOS-reported explanation, at most 512 UTF-8 bytes; render as text, never HTML or a control decision. |

```json
{
  "op": "result",
  "command_id": "move-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

The result reader validates the request before fetch and accepts only `{"ok":true,"result":{"command_id":...,"status":...,"reason":...}}` with the matching ID and bounded fields. EOS bounds the whole success response to 4 KiB. A malformed success, unknown status, ID mismatch or oversized response is shown as a protocol error and never as `Completed`. Fixed owner errors remain errors: malformed request (`400 invalid result request`), invalid origin or Host (`403`), unavailable operator grant (`503 operator control unavailable`), private grant rejection (`operator control grant required`), stale revision (`stale active revision`), opaque other-run/role/ingress/kind/absent ID (`unknown command ID`), bounded result unavailable (`result unavailable`) and IPC outage (`502 runtime unavailable`). The UI does not infer which hidden cause produced `unknown command ID`.

<a id="control--run-and-revision-fence"></a>
### Run and revision fence

The command record key is `(run_id, active_revision, binding_path, command_id)`, where `run_id` is IsOS's exact current deployment-run ID, `active_revision` is the 64-hex digest, `binding_path` is the active graph node path and `command_id` is generated once per invocation. Only the current selected record can update its visible status. On tab/page unmount, binding selection change, run/revision change or Stop, cancel its pending browser read and ignore late replies by generation token. A late reply from another generation cannot overwrite a newer command. Do not persist command IDs or results as authority in browser storage. A current-run log link may prefill `correlation_id` only when an EOS owner event for the captured run and command provides a valid IsOS correlation ID of 1–64 ASCII letters, digits, `_` or `-`; otherwise show the command ID separately and open unfiltered current-run logs. Never assume `command_id` equals `correlation_id`.

<a id="control--visible-states-and-stop"></a>
### Visible states and Stop

`sending` is browser-local and must not pulse as success. An EOS reply with matching ID changes it to the exact owner status. `InProgress` is nonterminal; `Completed`, `Failed`, `Cancelled`, `Rejected` and `OutcomeUnknown` are terminal display states. `Failed` receives an explicit failure label and reason. `OutcomeUnknown` receives an explicit uncertainty label; it never triggers a retry of the movement command. If send times out or the bridge disconnects before a correlated reply, display **submission outcome unavailable** and retain the generated ID for investigation; retry only a guarded read after status/revision/run recovery, never the command. An opaque ID or outage during follow-up displays **result unavailable**, retaining the last known `InProgress` state without converting it to a terminal outcome.

The result reader permits at most one in-flight request per command, begins after a one-second interval, backs off repeated follow-up errors to at most 10 seconds, and stops automatic polling after 60 seconds or 12 attempts, whichever comes first. It pauses when the document is hidden and resumes only after rechecking IsOS run/revision; manual Refresh uses the same checks. The UI shows a paused `InProgress` record and a Refresh action rather than implying completion. Page navigation and Stop abort the read. These bounds apply only to read-only result calls, never to another movement submission.

The deployment **Stop** action stays visible and enabled through IsOS `/api/isos/v1/process-control` even when EOS is offline, the graph is read-only, the active revision differs, a result is unknown or the result client has paused. A local stop click is a pending intent until IsOS reports its governed terminal or unknown outcome; the UI does not equate a `Cancelled` command result with physical stop. Stop remains reachable from the workbench and its floating panels.

The Stop reply has `stopped` (reaped direct-child IDs) and a separate `terminal` object or null; it is not a `process-status` snapshot. The UI fences any outstanding EOS result read as soon as Stop is clicked and blocks another Invoke for that captured run. It shows pending until the IsOS reply arrives. It displays **Stop confirmed** only for a well-formed `terminal.outcome:"confirmed"` whose `terminal.run_id` matches the known run captured at the click, with null reason and confirmed child outcomes. A well-formed `unknown` shows its fixed `reason_code` as an alert. Null, absent, malformed, or mismatched terminal evidence is **Stop unconfirmed**; null with an authoritative `run_id_state:none` means there was no managed run to stop. A transport failure is **Stop response unavailable** and never triggers an automatic retry. After a reply, a fresh `GET /process-status` updates live run and revision separately; its failure cannot erase the Stop outcome. Home, Graph, and EOS Stop controls use the same interpretation.

Duplicate process IDs cannot establish a well-formed terminal. A null terminal with nonempty `stopped` is unconfirmed even on a never-run home. An explicit IsOS API error is shown as **Stop rejected by IsOS** with its owner message, separately from transport unavailability.

Every ID in `stopped` must also appear in `terminal.processes`; a confirmed terminal with a missing child record is unconfirmed.

<a id="control--per-binding-node-control"></a>
### Per-binding node control

IsOS owns graph bytes, the saved and active SHA-256 revision, run ID and deployment Stop. EOS owns the active binding, target admission, live adapter status, journal and terminal result. IsosUI reads these owners and sends an operator-clicked command only through IsOS `POST /api/isos/v1/owners/eos/control`, which forwards to EOS admission. A graph wire declares the binding; it does not execute a command.

<a id="control--active-binding-and-request"></a>
#### Active binding and request

Before a send, IsosUI requires IsOS `run_id_state:known`, nonempty run ID, `state:running`, a 64-hex saved revision equal to the active revision and displayed graph revision, no draft edits or compatibility read-only state. It re-reads the saved graph and process status, verifies the same exact binding and target, and finishes the final IsOS run/revision fence before reading EOS process-local monotonic time. Immediately after that time read, it rechecks the latest local run, revision, binding, Stop, edit and abort state, then sends at most one `primitive_typed` POST with a fresh command ID. The four-second deadline starts from the last blocking owner read; an unavailable or expired send is never replayed with that ID. The header carries `data_type_id` (`base_movement` or `manipulator_movement`), `data_type_version:1.0`, exact primitive name/version, canonical binding/target paths, active revision, fresh 1–128-byte command ID, positive sequence, four-second monotonic deadline, UTF-8 JSON payload byte length (1–16384), and `codec:json_v1`. The payload is the validated typed movement object. The browser never supplies a grant or a private IPC operation.

<a id="graph_loading"></a>
<a id="graph_loading--graph-loading-and-status-projection"></a>
## Graph loading and status projection

The browser remembers the last opened pool file name. On opening, it selects that file if present, otherwise `demo` or the first available file. If the remembered file is missing, the editor announces the fallback; if no file remains or the open file disappears, Save asks for a new name. Save uses the opened file revision fence and states that saving replaces the same named file. Apply selects the exact saved pool revision and regenerates owner configuration; Save and Run performs Save, Apply, then Run.

IsOS owns saved graph bytes, revisions, catalogs, package receipts, readiness and process status. The browser holds only a draft and owner reported projections. The selected graph/catalog snapshot must have one matching `graph_revision` before the editor renders it or permits edits. An unsupported or mismatched snapshot remains read only.

Graph opening and an explicit Reload request wait for the selected graph/catalog snapshot. The canvas may render as soon as that snapshot passes the revision and edit-generation checks. Package inventory, process status and core readiness load independently afterward; failure or delay in one of these projections does not turn a valid graph into an unavailable graph. Late optional responses cannot replace a newer graph request, edit generation or status poll. An explicit reload retains the prior canvas while its snapshot is pending and discards a late snapshot after a local edit or newer catalog request.

When Graph regains browser focus or becomes visible, it refreshes the global and selected type catalogs and installed package inventory so a CLI package install appears without replacing draft graph bytes. The node-library search matches type identity, placement, description, and exact package pin across all cores and layers; choosing a result selects that exact type. Quick-add searches across packages and displays each exact pin. Legacy package process types are absent from both authoring menus; saved graphs remain readable.

The editor treats omitted `capabilities`, `settings`, `inputs` and `outputs` arrays in an owner node type as empty presentation metadata. Selecting or dragging that node must still open its inspector without changing the graph. IsOS remains authoritative for type registration and validation; the editor never infers an omitted capability or port.

Home starts graph, package, setup, readiness, process status and quick system health requests independently on opening, explicit Refresh and after a mutation. Process status and EOS/EI owner service health render as soon as their responses arrive, even while graph, setup or readiness is pending; failure of one projection does not discard the others. System health identifies reachable owner services and their reported run/revision, and a mismatch with the selected IsOS run is shown explicitly. It does not claim deployment readiness or Stop proof. One shared WebSocket supplies subsequent status changes to visible IsOS views. While that stream is unavailable, Home polls process status and system health independently every 1.5 seconds while visible, with at most one request of each kind in flight; this fallback does not recheck the graph. The root UI refreshes its graph on a streamed saved-revision change, or every 30 seconds when the stream is unavailable. A late poll or refresh response cannot overwrite a newer refresh or deployment operation. Home Run and Restart send the saved revision already shown on the page; IsOS checks that revision and graph admission before owner launch. Apply checks the selected graph revision and catalog authority. These UI rules do not change IsOS validation, Apply or command admission.

The browser shares one [runtime status WebSocket](http.md#http_runtime--get-apiisosv1runtimestatusws) across its visible views. It closes the socket while the tab is hidden, reconnects on visibility with bounded backoff, and uses HTTP status reads while the stream is unavailable. A status snapshot does not replace command-time validation or Stop proof.

Graph input bindings may carry a port-level `desired_rate_hz` range with optional positive `min` and `max` values. The node inspector edits that range for the input port and shows each source output's registered, optional official `frequency_hz`. A processing node may declare a different expected rate on its own output. An omitted bound adds no requested limit; the editor does not set an edge frequency. The edge inspector displays both port declarations. IsOS validates the desired range when the graph is checked or saved. The hardware inspector edits `adapter.settings.frequency_hz` as a positive JSON number capped at 1000 Hz, preserving the numeric type when a camera is saved.

<a id="log_cursors"></a>
<a id="log_cursors--current-run-log-navigation--p07-ui-contract"></a>
## Current-run log navigation — P07 UI contract

[中文](../../../../docs_cn/isos/interfaces/api/client_cn.md#log_cursors) · [Workbench design](../../core/ui/webui.md) · [First use](../../core/ui/webui.md)


<a id="log_cursors--selection-and-ordering"></a>
### Selection and ordering




| UI input or owner field | JSON type / unit | UI validation and use |
| --- | --- | --- |
| `view` | literal `current` / — | Required for P07; any other value is an IsOS error. |
| `source`, `level` | source ID or `all`; severity enum / — | Default `all` and `info`; filter detail only. Do not use source/level to select an anchor. |
| `node_path` | canonical saved-graph node JSON Pointer / — | Optional exact filter, at most 512 UTF-8 bytes; reject malformed or member pointers before request. A rejected filter returns HTTP 400 `log_node_path_invalid` at `/query/node_path`. |
| `correlation_id` | ASCII ID / — | Optional exact filter, 1–64 letters, digits, `_` or `-`; never free-text search. A rejected filter returns HTTP 400 `log_correlation_id_invalid` at `/query/correlation_id`. |
| `limit` | integer / events | Default 100, range 1–200; clamp UI input before request. |
| `cursor`, `next_cursor` | opaque ASCII string / position | Incoming cursor is at most 512 bytes; pass the returned value unchanged with the same query. Never parse or store across runs; changed filters reset it. Reusing a filter-mismatched cursor returns HTTP 400 `log_cursor_filter_mismatch` at `/query/cursor`; a malformed or tampered cursor returns `log_cursor_invalid` at the same path. The fixed diagnostic omits the token and filter values. |
| `scope` | object `{kind,run_id,active_revision}` / — | Required; `kind` is manager or deployment, ID is 1–64 ASCII bytes, active revision is a digest or null. Identity selects log history, not live health. |
| `process-status.run_id`, `run_id_state` | string or null; enum / run identity | `run_id_state` is `none`, `known` or `unavailable`; only `known` has the last admitted run ID, 1–64 ASCII bytes. `none` permits manager scope; `unavailable` blocks a current-run claim. |
| `anchor` | object `{kind,state,event}` / — | Required; pending/unavailable has null event, found has a complete sanitized P06 event. Pinned above all filters. |
| `lines`, `rotated` | event array; Boolean / — | At most `limit` same-scope events strictly after anchor; `rotated` alone does not mean loss. |
| `diagnostics` | array of `{code,count}` / lost events | Stable code from the four listed above; nonnegative count or null if unknown. Show a persistent accessible notice for loss or run change. |

<a id="log_cursors--rows-errors-and-bounds"></a>
### Rows, errors and bounds




A failed read retains the cursor and rows and retries after 2, 5, 10, then at most 30 seconds; returning to a visible tab rechecks immediately. The client distinguishes empty results, no matching event, collector unavailability, and retention gaps. Presentation and accessibility behavior is in the [WebUI guide](../../core/ui/webui.md).

<a id="monitor"></a>
<a id="monitor--monitor-group-interface"></a>
## Monitor group interface

[中文](../../../../docs_cn/isos/interfaces/api/client_cn.md#monitor) · [Node types](../node/README.md) · [Button event](client.md#button)

The starter graph returns the mock arm's observed six-joint state through EOS hardware output, dispatcher monitor, skill status, and interface egress nodes; EI IO input, cognition state, and governance monitor nodes; and `isos.ui.monitor.arm@v0.2`. The hardware output is a source with no graph input. EOS reads the same mock arm sampler locally, so the graph has a separate return chain and no second actuator.

Every return edge carries `arm_monitor_state@v0.2` on its `observation` port. The payload has the active `run_id` and `revision`, graph-declared observation `source_node` (`eos/hardware/mock/arm_observer` in the two-chain demo; `eos/hardware/mock/arm` in legacy graphs), increasing `sequence`, `sampled_at_monotonic_ns`, six ordered `joint_names`, `state` (`ready`, `degraded`, `stale`, or `stopped`), and nullable `reason_code`. `joint_positions_rad` contains six finite radians only in `ready` state; otherwise it is null. EOS checks each 100 Hz local sample before command admission, EI receives at most 20 Hz, and IsosUI refreshes at most 5 Hz. These rates and freshness limits are Advanced graph settings.

`GET /api/isos/v1/nodes/monitor` uses the normal IsOS API envelope. Its data is `{state, data_type, run_id, active_revision, observation}`. A stopped deployment returns `state:"stopped"` and null identities and observation. For a running or degraded deployment with a known run, IsOS checks the saved graph revision and complete declared return path, then reads one validated EI snapshot at most once per 200 ms for all local UI readers. The cache key includes deployment, run, and revision; a changed run cannot receive an old snapshot. IsOS rechecks the run before replying. An incomplete path, stale graph, unavailable owner when a fresh read is required, or invalid reply fails closed. IsosUI clears an old run's values on Stop or run change. `GET /api/isos/v1/nodes/monitor/ws` provides the same snapshot over a read-only WebSocket at `refresh_hz=1`–`5` (default 5), emitting changed snapshots or bounded error events and pinging every 15 seconds. The browser uses this route for live monitoring. It has the same loopback Host and approved Origin admission as the HTTP route.

When the active run uses an older graph revision, the endpoint returns `graph_fence_stale` (409). IsosUI identifies this as an old active run rather than an editor conflict and keeps the monitor empty until the run is stopped or recovered and the saved graph is run.

<a id="presentation"></a>
<a id="presentation--presentation-descriptor-boundary"></a>
## Presentation descriptor boundary

This page explains the typed descriptor used by the pure selector. It is for owner and client implementers. A descriptor identifies a registered output and one representation; it carries no payload bytes or endpoint authorization. The live HTTP route currently has no verified descriptor registration and returns 503 after input and run checks.

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `node_path`, `port` | strings / graph path and port ID | Identify one output; both must be valid, bounded tokens. | Required. |
| `data_type` | `DataType` / semantic type | Carries codec, media type, schema, and payload bound for selection. | Required. |
| `representation_id` | string / identity | Distinguishes producer representations for one output. | Required. |
| `method` | enum / renderer | One of `json`, `text`, `chart`, `image`, `canvas`, `overlay`, `audio`. | Required. |
| `delivery` | enum / transport | `http` or `web_socket`; does not grant access. | Required. |
| `endpoint_id` | string / opaque key | Identifies a producer route; it is not a URL or private grant. | Required. |
| `run_id`, `graph_revision` | strings / run ID and SHA-256 | Fence the descriptor to one active run and graph revision. | Required. |

The pure [selector](client.md#selection) checks field shape, codec/media/method compatibility, payload bound, and requested run/revision. Registered port/type binding is an upstream owner-admission responsibility; the current HTTP route cannot perform live descriptor selection. The [HTTP contract](http.md#http_presentations) defines its current request and errors.

[中文](../../../../docs_cn/isos/interfaces/api/client_cn.md#presentation)

<a id="selection"></a>
<a id="selection--generic-output-data-selection"></a>
## Generic output data selection

[中文](../../../../docs_cn/isos/interfaces/api/client_cn.md#selection) · [Presentation HTTP](http.md#http_presentations) · [Release types](../package/README.md#release)

<a id="selection--overview"></a>
### Overview

The release `DataType` and registered output port are the type authority. Release validation checks the ID/version suffix; an owner registration must prove the port/type binding before descriptors reach the selector. IsOS selects public presentation metadata for an applied output; the producer owns payload bytes, encoding, endpoint authorization, and delivery. A MIME label or UI method alone cannot authorize decoding. No second presentation manifest is introduced.

<a id="selection--type-authority"></a>
### Type authority

[Node data types](../../../node/interfaces/type.md#data_types) are the single field authority for `id`, `version`, `purpose`, `media_type`, `codec`, `schema`, and `max_payload_bytes`. A registered output port binds that type. The owner descriptor then binds node path, port, representation, delivery, opaque endpoint ID, run ID, and graph revision; see [presentation descriptor](client.md#presentation) for its fields. IsOS selects metadata only and never takes payload bytes or endpoint authorization.

<a id="selection--representation-families"></a>
### Representation families

JSON records and text use `json` and `text`; JSON time series marked `time_series` use `chart`; JPEG/PNG use `image`; bounded RGB/RGBA/depth planes use `canvas`; JSON detections marked `detections` use `overlay`; Opus or PCM audio uses `audio`. Depth requires a unit; producer frame contracts must also define invalid-value semantics. Detections need coordinate space and image association, and audio needs sample rate and channel semantics. These payload fields belong to the producer contract, so IsOS does not invent them from a type name.

<a id="selection--availability"></a>
### Availability

The current IsOS route checks input and run/revision fences, then returns 503 because the selected owner has not published a verified public descriptor. See [presentations](http.md#http_presentations) for the exact request and current error. Stop, Restart, and Apply invalidate an earlier selection.

<a id="vision"></a>
<a id="vision--vision-group-interface"></a>
## Vision group interface

[中文](../../../../docs_cn/isos/interfaces/api/client_cn.md#vision) · [Node types](../node/README.md)

`isos.ui.vision.monitor@v0.2` has one required `frame` input of `image_frame@v0.2`. It connects to the `frame` output of `eos.interface.egress.vision_flow@v0.2`, whose `frame` input connects to a compatible EOS camera. The EOS transfer is a standard interface node, not a hardware adapter. The camera node owns source selection. Apply and Run reject an incomplete or unrouted path.

`GET /api/isos/v1/vision/bindings` returns the active `run_id`, graph `revision`, and one `{monitor_path,transfer_path,source_path,label,interval_ms}` binding per monitor. A stopped, stale, or unavailable run fails closed. After reading the bindings, the browser opens `/api/isos/v1/vision/ws` and sends one text subscription `{monitor_path,run_id,revision}`. IsOS accepts only a binding in the active saved graph. Each outbound binary WebSocket message is `[u32 big-endian header length][JSON header][JPEG bytes]`. The header contains `{op:"frame",run_id,revision,monitor_path,transfer_path,source_path,output_sequence,sample,width_px,height_px,encoding:"jpeg"}`. The header is at most 4096 bytes and JPEG at most 20 MiB. `output_sequence` is the native graph's capture output sequence; `sample.sequence` is the source clock's sample sequence and may advance independently. IsOS checks each packet against the selected binding and fences; a changed graph, EOS failure, Stop, or socket closure ends the stream. The browser retains only the newest output sequence and clears the view when the stream closes or its run changes.

The camera captures at its configured frequency. EOS pushes each new JPEG through a separate bounded worker data connection after admitting the exact run, revision, transfer, and source route. IsOS applies the authored delivery interval before sending. The package output port's optional official `frequency_hz` does not schedule capture or limit delivery. The lower `desired_rate_hz.max` on the two downstream inputs limits delivery when present; otherwise delivery is uncapped. `interval_ms` reports only that delivery cap (`null` means no representable interval). A cap below one frame per year delivers the first frame and closes the stream. An upgraded socket must send its subscription within five seconds.

If the bindings read fails or the stream closes during a server or owner restart, the visible Vision panel retries with bounded backoff. Every attempt fetches bindings again and checks the exact current run ID, revision, and monitor path before subscribing. The panel clears its image on disconnect and never replays a control request. A hidden panel waits until it becomes visible.

<a id="workspace_panels"></a>
<a id="workspace_panels--u03-workspace-panel-contract"></a>
## Workspace subscription lifecycle

[中文](../../../../docs_cn/isos/interfaces/api/client_cn.md#workspace_panels)

A hidden or collapsed monitor, Vision, or Logs panel stops its poll or stream. On reopening, the client rechecks the active run and revision before resuming. One Logs consumer owns the cursor for that scope; Stop or a changed run invalidates it. The Vision display uses only frames accepted for its bound source and current run. Panel layout and labels are described in the [WebUI guide](../../core/ui/webui.md).
