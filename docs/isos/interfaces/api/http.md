<a id="http"></a>
<a id="http--local-http-routes"></a>
# Local HTTP routes

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/api</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Client API and frontend interfaces](README.md)
  - [Client interactions](client.md)

</details>
<!-- doc-navigation:end -->

## Table of contents

- [Catalog HTTP API](#http_catalog--catalog-http-api)
- [Cores API v1](#http_cores--cores-api-v1)
- [Local HTTP envelope v1](#http_envelope--local-http-envelope-v1)
- [Graph pool API v1](#http_graph_pool--graph-pool-api-v1)
- [Graph API v1](#http_graph--graph-api-v1)
- [Owner control projection](#http_owners--owner-control-projection)
- [Packages HTTP API](#http_packages--packages-http-api)
- [Presentation HTTP route](#http_presentations--presentation-http-route)
- [Runtime API v1](#http_runtime--runtime-api-v1)
- [Version and health API](#http_version--version-and-health-api)


This page is for IsOS server and local-client implementers. The routes begin at `/api/isos/v1/`; a request passes local Host/Origin admission and is decoded by `app`, while graph, package, and runtime owners make the decision. The [external node-event request](client.md#button) is documented with the node contract.

- [Envelope](http.md#http_envelope) defines loopback request admission and success and error response fields, including diagnostic code, stage, and graph path.
- [Version](http.md#http_version) returns API identity so a client can select its supported interface.
- [Graph](http.md#http_graph) covers draft validation, saved revision, and Apply; the saved instance schema is in [nodes](../../../node/interfaces/graph.md#instance).
- [Graph pool](http.md#http_graph_pool) stores named drafts independently of the active deployment.
- [Catalog](http.md#http_catalog) exposes registered types and selector options without changing the saved graph.
- [Packages](http.md#http_packages) covers install, link, inspect, and removal around exact verified pins.
- [Cores](http.md#http_cores) exposes owner discovery and readiness to local clients.
- [Runtime](http.md#http_runtime) covers Run, Restart, Stop, status, and logs with run/revision fences.
- [Owner control projection](http.md#http_owners) forwards bounded public EOS operations and Vision streams through IsOS while EOS retains admission.
- [Graph Vision monitor](client.md#vision) defines the graph-bound camera stream shown in the IsOS UI.
- [Presentations](http.md#http_presentations) exposes selected output metadata; [UI selection](client.md#selection) owns the generic display rules.

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md)

<a id="http_catalog"></a>
<a id="http_catalog--catalog-http-api"></a>
## Catalog HTTP API

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_catalog) · [Envelope](http.md#http_envelope) · [Graph](../node/README.md)

The catalog routes expose registered node types, selected types, node options, and node-set previews. They derive results from verified generic `node_contributions` and the selected owner catalog; no package ID changes parsing or priority. All routes require loopback Host/Origin admission.

Catalog entries for contributed types include their exact `package` `{id, version}`. The same type ID and placement may appear more than once when distinct installed releases contribute it. A graph instance selects one definition with its top-level `package` pin; selectors, settings, and edge admission use that same identity. An unpinned or mismatched contributed instance is rejected rather than assigned to an arbitrary release. The current EOS runtime registry is keyed by type ID; Apply rejects a graph selecting two releases that contribute the same EOS type ID to one owner.

| Route | Result | State effect |
| --- | --- | --- |
| `GET /api/isos/v1/catalog/node-types` | Registered types and availability. | None. |
| `GET /api/isos/v1/catalog/node-types/selected` | Types referenced by applied graph. | None. |
| `POST /api/isos/v1/catalog/node-options` | Choices for declared settings, stable resource IDs, and contextual EOS/EI owner selectors for the supplied draft. | None. |
| `GET /api/isos/v1/catalog/node-sets` | Verified declared node sets. | None. |
| `POST /api/isos/v1/catalog/node-sets/preview` | Candidate graph composition; never publication. | None. |

Node-set preview preserves existing nodes. When an added node's path or EOS hardware instance ID is already used, IsOS assigns the first free `_2`, `_3`, … suffix to the added instance and rewrites references within the set to the assigned path. The response's `added_paths` reports the final paths. A preview still validates the complete candidate graph and rejects unsupported owner types, adapters, routes, and mixed execution modes; it never repairs an invalid existing node. A rejected preview returns `node_set_preview_rejected` with a bounded reason instead of an opaque validation error.

Requests use `schema_version:"0.1.0"` where a body is required. Malformed schema is 400; unknown exact pin, type, resource, or owner returns a bounded admission diagnostic. A stale draft revision is 409. Details of field identity and validation are in [graph](../../../node/interfaces/graph.md#instance), [options](../../../node/interfaces/type.md#selectors), and [resources](../package/README.md#resources).

For a package camera, node-options also offers `adapter.settings.service_instance` and `adapter.settings.device_id` from that exact verified package's camera node-set profiles. The service list is not fixed to RoboCasa. A service is available only when its package configuration and local prerequisites validate. Choosing a service clears the previous device choice; choosing a device applies its declared frame and adapter profile. A newly added camera has no implicit device. A selected value absent from the verified package remains visible as unavailable; Apply and Run reject an undeclared camera source.

Owner choice groups retain `node_path`, `setting`, the draft's `selected` value, and bounded `options` with availability and reason fields. Catalog sidecar selection verifies each exact package pin once per request and skips EOS or EI option previews when that owner supplies no selector in the draft. Required owner previews still validate the current draft. Debug timing events `catalog_queue_wait` and `node_options_stage` report bounded milliseconds for queue wait, package verification, catalog construction, owner preview, and total work without package identities or payloads. IsOS checks owner scope and group identity before returning them. The workbench ties the response to the current draft; a failed owner request leaves the selector unavailable. Draft type catalogs and options use the same selected local EOS/EI setup adapters, even when a persistent owner still runs an older admitted generation. Apply repeats validation with the live owner because a preview can become stale; a rejected Apply cannot start that draft.

<a id="http_cores"></a>
<a id="http_cores--cores-api-v1"></a>
## Cores API v1

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_cores) · [Owner boundaries](../manager.md)


<a id="http_cores--get-apiisosv1coresreadiness"></a>
### GET /api/isos/v1/cores/readiness

Returns `data.cores`, graph and IsOS/UI configuration readiness. It validates the saved graph against the selected package and EI owner stage catalog, including each registered `isos.<layer>.<name>@v0.2` placement. It describes selected installed paths and compatibility; use deployment status and owner events for live readiness.

An enabled core with no selected generation reports `graph_configured: false` and asks the operator to Apply the saved graph. If a previously applied pool file is saved with new bytes, `configuration_diagnostic.code` is `pending_apply` and Run remains fenced until Apply. If a selected binary changed after Apply, `configuration_diagnostic.code` is `generated_launch_input_mismatch`, `ready` is false, and the action asks for another Apply. An invalid selected snapshot reports its diagnostic instead of being presented as an unsaved graph. A stopped but ready core asks the operator to run the saved graph. A core with owner layers but no process node reports that missing process explicitly. A truly empty graph leaves EOS and EI disabled without an error.

For the health-only three-core graph, EOS owner state `ready` and EI owner state `running` both set `running: true`. This field reflects the owner worker reported by deployment status; `ready` separately describes whether the saved configuration can run. If an owner fails, deployment status becomes degraded and the workbench records the affected owner state in its browser log.

<a id="http_cores--get-apiisosv1coressetup-status"></a>
### GET /api/isos/v1/cores/setup-status

Returns `data.ready`, `issues`, `graph_revision`, backend choices, and process summary. Example response: `{"schema_version":"0.1.0","ok":true,"data":{"ready":false,"issues":["selected owner setup is unavailable"],"graph_revision":null,"backends":[],"process":null},"error":null}`.

<a id="http_cores--get-apiisosv1coressetuprolecatalog"></a>
### GET /api/isos/v1/cores/setup/{role}/catalog

Returns the selected EOS or EI owner setup catalog for the saved graph without publishing configuration. The route accepts only `eos` or `ei`; an unavailable owner returns 503.

<a id="http_cores--post-apiisosv1coressetuprolevalidate"></a>
### POST /api/isos/v1/cores/setup/{role}/validate

Request `{"schema_version":"0.1.0","graph":{"version":"0.2.1","robot":{"id":"robot","name":"Robot"},"cores":{"isos":{"layers":{}}}},"revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}`. IsOS passes the draft and revision to the selected owner; no configuration is published. The owner reply contains numeric `schema_version:3`, `ok`, `diagnostics`, `files`, and optional `identity` or `catalog`.

<a id="http_cores--post-apiisosv1coressetuprolerender"></a>
### POST /api/isos/v1/cores/setup/{role}/render

Uses the same request shape as validate and previews bounded relative generated files without publishing them. A missing graph is 400; unknown role or operation is 404; timeout, malformed reply, or setup identity without `setup_schema_version:3` is 503.

`schema_version` is string/— and must equal `0.1.0`; `role` is a closed string/— to prevent selecting arbitrary executables; `graph` is the registered profile object/—; `revision` is an optional 64-character lowercase SHA-256 string when the caller already has a saved-byte fence. The external setup wire uses schema 3 and bounded stdin/stdout; a product version does not imply setup compatibility.

<a id="http_cores--failure-example-and-field-contract"></a>
### Failure example and field contract

For `POST /api/isos/v1/cores/setup/eos/validate`, an illustrative structured failure is:

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "owner_unavailable",
    "message": "required owner is unavailable",
    "details": null,
    "correlation_id": "req-42"
  }
}
```


<a id="http_cores--verification"></a>
### Readiness and event admission

Readiness uses the selected EI stage catalog accepted at admission. An EI or EOS event with a node path must name an admitted node in the saved graph under that owner. A same-revision Apply can regenerate missing launch inputs; the resulting readiness reply reflects the new generated state.

<a id="http_envelope"></a>
<a id="http_envelope--local-http-envelope-v1"></a>
## Local HTTP envelope v1

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_envelope) · [Interface catalog](../README.md)

<a id="http_envelope--overview"></a>
### Overview

The loopback API exposes several domains under `/api/isos/v1/`. Each handler returns a JSON envelope. Domain pages own route payloads; this page owns transport-wide rules. The API schema is independent of the IsOS product, graph, package, and owner protocol versions.

<a id="http_envelope--request-admission"></a>
### Request admission

This page is for implementers of the IsOS loopback API. The current listener binds to `127.0.0.1` on `ISOS_HTTP_PORT` (default `16000`). Before a route runs, the transport checks that `Host` equals the configured loopback host and port. A supplied `Origin` must be the local IsOS origin, the configured IsosUI origin, or one of the hosted UI origins. A cross-site `sec-fetch-site` request without `Origin` is rejected. Current local requests do not carry an owner bearer token.

CORS preflight requires `Origin`; allowed requested methods are `GET`, `POST`, and `PUT`, and the only permitted requested header is `Content-Type`. A rejected Host, Origin, or preflight returns 403 before graph or runtime mutation. Individual routes still validate payload, run fence, and graph authority. The [node-event contract](client.md#button) gives the first UI control request.

For a preflight from an approved local or hosted IsosUI origin with `Access-Control-Request-Private-Network: true`, the API returns `Access-Control-Allow-Private-Network: true`. Other origins receive no private-network grant.

<a id="http_envelope--interfaces"></a>
### Interfaces

Success has `schema_version:"0.1.0"`, `ok:true`, domain-specific `data`, and `error:null`. Example from the implemented system route:

```json
{
  "schema_version": "0.1.0",
  "ok": true,
  "data": {
    "api_schema_version": "0.1.0",
    "isos_version": "0.1.0"
  },
  "error": null
}
```

Failure has `schema_version:"0.1.0"`, `ok:false`, `data:null`, and a structured error. The `correlation_id` value below is illustrative; the server generates one for each failure.

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "invalid_request",
    "message": "invalid request",
    "details": null,
    "correlation_id": "req-42"
  }
}
```

| Field | Type / unit | Purpose and validation |
| --- | --- | --- |
| `schema_version` | string / version | Required wire discriminator, exactly `0.1.0`; a product version cannot substitute. |
| `ok` | boolean / none | True only for a successful operation. |
| `data` | JSON value or null / domain-specific | Domain result; null on failure. Do not infer a result from an error response. |
| `error` | object or null / none | Null on success; structured fault on failure. |
| `error.code` | string / none | Stable machine-readable category or domain diagnostic. |
| `error.message` | string / none | Safe human-readable fault; do not parse it as a protocol field. |
| `error.details` | JSON value or null / domain-specific | Optional bounded diagnostics; null means none supplied. |
| `error.correlation_id` | string / none | Server-issued identifier for matching sanitized logs. |

Malformed JSON and invalid input use 400. Host/Origin rejection uses 403, missing resources 404, revision conflicts 409, admission failure 422, and unavailable owners 503. A domain may issue a more specific stable `error.code`; clients branch on code, not message. JSON mutations requiring `schema_version` reject another value before publication. Unknown fields are rejected where the route's request type declares a closed shape; clients must not assume one global unknown-field policy.

An internal operation failure uses the same `correlation_id` in its HTTP response and the server's `http_request_failed` event, so the operator can find the underlying cause without exposing it in the browser. A Run rejected because EOS or EI still holds a prior run returns HTTP 409 with `error.code:"run_conflict"` and a diagnostic at `/owners/eos` or `/owners/ei`; the diagnostic includes bounded `run_id` and `revision` strings when the owner supplies them. Confirm Stop for that run before retrying. This differs from a graph revision conflict, which has its own code.

<a id="http_envelope--routes"></a>
### Routes

[Version](http.md#http_version), [graph](http.md#http_graph), [catalog](http.md#http_catalog), [packages](http.md#http_packages), [cores](http.md#http_cores), [runtime](http.md#http_runtime), and [presentations](http.md#http_presentations) define their route payloads and side effects. The [node-event contract](client.md#button) defines the eighth domain. The router is the source of truth for method/path pairs.

<a id="http_graph_pool"></a>
<a id="http_graph_pool--graph-pool-api-v1"></a>
## Graph pool API v1

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_graph_pool) · [Active graph API](http.md#http_graph)

The graph pool is the workspace for graph files under the local IsOS configuration directory. A deployment binding records only the selected pool name; the graph bytes exist only in the pool. All routes use the same loopback Host and Origin checks as the graph API. Saving the bound file changes its saved revision, so generated configuration must be refreshed with `PUT /api/isos/v1/graph` before Run. Saving does not start or stop a process.

<a id="http_graph_pool--get-apiisosv1graph-pool"></a>
### GET /api/isos/v1/graph-pool

Returns `data.graphs`, an array of `name`, exact-byte SHA-256 `revision`, and `version`, sorted by name. The pool accepts at most 128 drafts. Unsafe or oversized entries fail closed.

An empty pool returns:

```json
{
  "schema_version": "0.1.0",
  "ok": true,
  "data": {
    "graphs": []
  },
  "error": null
}
```

<a id="http_graph_pool--get-apiisosv1graph-poolname"></a>
### GET /api/isos/v1/graph-pool/{name}

Returns `data:{name,revision,graph}` for one draft. Names contain 1–48 ASCII letters, digits, hyphens, or underscores. An absent name returns 404.

<a id="http_graph_pool--put-apiisosv1graph-poolname"></a>
### PUT /api/isos/v1/graph-pool/{name}

Saves a `0.2.1` graph file. Start from the 17-node demo with button and mock-arm observation paths joined at EI context returned by `GET /api/isos/v1/graph/new`, then send `{"schema_version":"0.1.0","revision":"","graph":<draft>}` to create a new name. To replace an existing file, send its exact revision returned by GET; the editor informs the user that saving to the same name replaces that file. A changed revision returns 409. Files may be incomplete while edited; Apply requires all three cores and validates the complete graph. Graphs are limited to 1 MiB.

The Web editor opens one named file from the pool. It combines the revision-checked selected owner catalog with the global catalog, so owner-only node types keep their settings and ports. Canvas positions and the last opened pool name remain browser-local. If that file disappears, the editor asks for a new name when saving.

<a id="http_graph"></a>
<a id="http_graph--graph-api-v1"></a>
## Graph API v1

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_graph) · [Node interface](../node/README.md)

IsOS owns the saved graph and its exact-byte SHA-256 revision. All `/api/isos/v1/*` routes require loopback Host and Origin admission. JSON mutations use `schema_version: "0.1.0"`. Successful responses use `{ "schema_version": "0.1.0", "ok": true, "data": ..., "error": null }`; failures set `ok: false` and return a stable error code, safe message, and correlation ID. A stale revision returns 409, and graph admission failure returns 422.

`GET /api/isos/v1/graph` reads `{pool_name, revision, graph, read_only, diagnostics, warnings, missing_file}` from the bound pool file without modifying its bytes. If that file is missing, it returns an editable three-core starter draft, an empty revision, and `missing_file:true`; the editor must save it under a pool name before Apply. `GET /api/isos/v1/graph/new` returns the 17-node demo with button and observation paths joined at EI context without saving it.

`GET /api/isos/v1/graph/convert?revision=<sha256>` returns an unsaved `0.2.1` draft converted from the bound `0.2.1` graph only when `revision` matches its exact saved bytes. Conversion changes only `version`; instance IDs, inputs, settings, and package pins remain intact. Save the draft through the normal graph-pool PUT using that revision; a concurrent edit returns 409.
The workbench can open a legacy pool entry read-only and preview the same conversion. Its Save action checks the entry revision; Apply then verifies and binds the converted graph.

`POST /api/isos/v1/graph/validate` accepts `{schema_version, graph}` and returns `{valid, diagnostics}`. `PUT /api/isos/v1/graph` accepts `{schema_version, pool_name, revision}` and returns the applied `{revision, graph, pool_name, warnings}`. The revision must equal the named pool file's exact-byte digest. IsOS verifies the three cores, registered owner types, static links, package pins, and owner projections, publishes a generation, and binds the deployment to that pool file. A missing file returns 404; a stale revision returns 409. Legacy callers may still provide `{schema_version, revision, graph}`; this updates the currently bound pool file and should not be used by the editor.

After saving `demo` in the graph pool, Apply uses its returned revision. The digest below is illustrative; the caller copies the actual value from the pool GET response.

```json
{
  "schema_version": "0.1.0",
  "pool_name": "demo",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

`POST /api/isos/v1/graph/connection` previews one draft connection. It accepts `{schema_version, graph, source, output, target, input}` after the draft contains that input reference. `source` and `target` are canonical four-part node paths; `output` and `input` are port names. It returns `{allowed:true, edge}` with the exact admitted `source`, `output`, `target`, `input`, route class, channel, and QoS fields, or `{allowed:false, diagnostic}`. Other required inputs and fixed binding targets may still be unconnected during this preview. The selected catalog must be available (503 otherwise); malformed API versions return 400. This read does not save the draft, and Apply repeats full admission.

The current graph format is `0.2.1`, with `isos`, `ei`, and `eos` cores and type IDs `<core>.<layer>.<group>.<node>@v0.2`. The repository's [mixed example](../../../../examples/mixed-mujoco.nodes.json) has 17 nodes. It keeps the arm observation and camera vision paths, but leaves the mock-arm control path without an activation source to exercise incomplete-chain warnings; it cannot command the arm. Applying that example requires the verified `island/mujoco-sim@0.2.4` package and its configuration. The package-free 17-node starter returned by `/graph/new` has a connected button-to-arm path. The repository demo extends it with a `island/mujoco-sim@0.2.6` camera and Vision path. The mixed example launches no IsOS child process specs. Live worker health is read through deployment status; graph reads remain configuration reads.

`POST /api/isos/v1/graph/validate` returns `warnings` alongside `valid`, `diagnostics`, and admitted `edges`. A missing registered required input is an `edge_required_input` warning at the input pointer, with target node and input name; it does not make the graph invalid or read-only. Connected edges still require full type and route admission. The warning never authorizes runtime delivery along a disconnected path.

<a id="http_owners"></a>
<a id="http_owners--owner-control-projection"></a>
## Owner control projection

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_owners) · [HTTP routes](http.md) · [EOS control contract](../server/eos.md#control)

<a id="http_owners--overview"></a>
### Overview

IsOS exposes one bounded projection of the selected EOS public control bridge for its Web UI. IsOS checks the active run and saved graph revision, resolves the bridge's loopback address from the verified graph, and forwards only public operations. EOS remains responsible for command admission and actuator safety. This route cannot select an arbitrary owner endpoint or supply a private grant.

<a id="http_owners--authentication"></a>
### Authentication

The caller must pass the IsOS loopback Host and approved Origin checks. The request carries no token or owner secret. IsOS accepts only a running deployment with a known run ID, matching saved graph bytes, and a running bridge process. The owner bridge performs its own admission.

<a id="http_owners--interfaces"></a>
### Interfaces

Requests and replies use JSON. The IsOS [envelope](http.md#http_envelope) has schema `0.1.0`; its `data` is the bounded EOS public reply, retaining EOS's `ok` and operation fields. Request bodies are at most 20,480 bytes and owner replies at most 65,536 bytes. The operation must be one of `discover`, `status`, `health`, `observe`, `time`, `result`, `primitive`, `primitive_typed`, or `manual`. Top-level `grant`, `token`, and `endpoint` fields are rejected. EOS's [control contract](../server/eos.md#control) owns operation-specific fields and validation.

<a id="http_owners--routes"></a>
### Routes

<a id="http_owners--post-apiisosv1ownerseoscontrol"></a>
#### POST /api/isos/v1/owners/eos/control

The IsOS Web UI sends an EOS public operation as a JSON object. For example, `{"op":"time"}` returns `{"schema_version":"0.1.0","ok":true,"data":{"ok":true,"monotonic_ns":123},"error":null}` when EOS accepts it. Status `400` rejects malformed JSON or an unlisted operation; `403` rejects Host or Origin; `409` rejects a stale graph revision; `413` rejects an oversized request; `422` reports EOS rejection; `502` reports an invalid or oversized owner reply; `503` reports unavailable run, manager, endpoint, or bridge. A failure uses the same envelope, for example `{"schema_version":"0.1.0","ok":false,"data":null,"error":{"code":"owner_bridge_unavailable","message":"EOS bridge is unavailable","details":null,"correlation_id":"req-1"}}`.

<a id="http_packages"></a>
<a id="http_packages--packages-http-api"></a>
## Packages HTTP API

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_packages) · [Envelope](http.md#http_envelope) · [Generic package contract](../package/README.md)

All routes require loopback Host/Origin admission. Requests and responses use `schema_version:"0.1.0"`. Install/link/preview never change the saved graph or active run. The package ID is data, never a parser switch.

| Method and route | Input | Success | Failure |
| --- | --- | --- | --- |
| `GET /api/isos/v1/packages` | No body | `data.installed` rows with exact pin and verification status. | 403 Host/Origin; 503 store error. |
| `POST /api/isos/v1/packages/preview-download` | Exact `id`, `version`. | Selected asset URL, size, SHA-256, publisher/source, trust state, and an approval digest; no asset download. | 400 invalid or incompatible release. |
| `POST /api/isos/v1/packages/install` | Exact `id`, `version`, `approval` from the preview. | Verified receipt, path, updated inventory. | 400 missing/stale approval; 409 duplicate pin; 422 unsupported contract or unsafe asset. |
| `POST /api/isos/v1/packages/link` | Exact `id`, `version`, absolute `path`, optional absolute `manifest`. | Verified link receipt and inventory. | 400 malformed; 409 duplicate pin; 422 unsupported or changed payload. |
| `POST /api/isos/v1/packages/preview-link` | Absolute bundle `path`, optional `manifest`. | `id`, `version`, `ready`, bounded `issues`; no publication. | 400 malformed; 422 unsafe bundle. |
| `GET /api/isos/v1/packages/config?id=<id>&version=<version>` | Exact installed pin in query. | `data` contains `descriptor`, `toml`, parsed `values`, and `revision` for this deployment and pin. | 422 missing or invalid descriptor or configuration. |
| `POST /api/isos/v1/packages/config/validate` | `schema_version`, exact `id` and `version`, and exactly one of `toml` or `values`. | `data.valid` and bounded `data.diagnostics`; does not save. | 400 malformed edit; 422 missing configuration. |
| `PUT /api/isos/v1/packages/config?id=<id>&version=<version>` | Same edit plus the current `revision`; query and body identity must match. | Saved `revision` and `toml`; a changed configuration needs Apply before Run. | 400 malformed or mismatched edit; 409 stale revision or invalid configuration. |

```json
{
  "schema_version": "0.1.0",
  "id": "vendor/sample-a",
  "version": "1.2.3",
  "path": "/absolute/bundle",
  "manifest": "/absolute/bundle/release.json"
}
```

A successful link returns `{"schema_version":"0.1.0","ok":true,"data":{"verified":true,"source_kind":"local_link"},"error":null}` plus the documented receipt and inventory fields. A contribution missing required registered declarations fails admission. Package removal refuses saved or active exact-pin references. Fields of the release, receipt, resources, and diagnostics are authoritative under [package interfaces](../package/README.md).

The approval digest binds the package ID, version, source commit and URL, and selected asset URL, size, and SHA-256. The install handler fetches current catalog metadata again and rejects an approval for changed bytes before downloading. CLI installation prints the same preview and requires `yes` on a terminal or `--approve-download <digest>` in scripts. Local source builds and local archives are explicit separate paths.

<a id="http_presentations"></a>
<a id="http_presentations--presentation-http-route"></a>
## Presentation HTTP route

This contract is for local clients requesting a representation of one active graph output. The route is implemented but no owner currently registers verified public descriptors, so a valid current request returns 503. The [data type](client.md#selection) and [descriptor](client.md#presentation) contracts define the internal selection boundary.

<a id="http_presentations--authentication-and-request"></a>
### Authentication and request

`POST /api/isos/v1/presentations/select` requires loopback Host/Origin admission. It accepts a JSON body with `schema_version: "0.1.0"`; unknown fields and malformed JSON fail. The shared [envelope](http.md#http_envelope) defines errors and correlation IDs.

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `schema_version` | string / version | Must equal `0.1.0`; numeric values fail with `unsupported_schema_version`. | Required. |
| `node_path` | string / graph path | Canonical four-segment output owner path; bounded and validated. | Required. |
| `port` | string / output port ID | Identifies one output; bounded and validated. | Required. |
| `accept` | nonempty array / renderer methods | Client-supported `json`, `text`, `chart`, `image`, `canvas`, `overlay`, or `audio`; order is preference. | Required. |
| `run_id` | string / run identity | Must match the active run. | Required. |
| `graph_revision` | string / SHA-256 | Must match the active revision. | Required. |

```json
{
  "schema_version": "0.1.0",
  "node_path": "isos/eos_interface/gate/front",
  "port": "preview",
  "accept": [
    "image",
    "canvas"
  ],
  "run_id": "r7-cde",
  "graph_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

<a id="http_presentations--current-results"></a>
### Current results

Malformed selection returns 400 `invalid_request`; wrong schema returns 400 `unsupported_schema_version`. A stale run or revision returns 409 `revision_conflict`. A current request with no registered descriptor returns 503 `owner_unavailable`:

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "owner_unavailable",
    "message": "presentation owner has not registered a public delivery descriptor",
    "details": null,
    "correlation_id": "req-43"
  }
}
```


[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_presentations)

<a id="http_runtime"></a>
<a id="http_runtime--runtime-api-v1"></a>
## Runtime API v1

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_runtime) · [Stop proof](../core.md#core_dependent_stop) · [Log cursors](../manager.md#manager_log_cursors)

**Implemented on the IsOS side.** IsOS owns deployment command admission, status, and bounded log reads. All routes require loopback Host/Origin admission. The common HTTP envelope is `{schema_version:"0.1.0",ok,data,error}`; errors carry stable `code`, safe `message`, and `correlation_id`. A command returns only after the manager responds, and a stop result is never called confirmed without owner proof.

<a id="http_runtime--get-apiisosv1runtimeprocess-status"></a>
### GET /api/isos/v1/runtime/process-status

Returns the manager's status including saved and active revisions, process state, run ID, and retained terminal outcome. Example response shape: `{"schema_version":"0.1.0","ok":true,"data":{"state":"stopped","run_id":null,"processes":[],"packages":[]},"error":null}`. `packages` is an array of exact `{id,version,path}` graph pins; an empty array means no selected pins. Additional health fields follow the [status contract](../core.md#core_dependent_stop).

The manager keeps status responsive while Run or Restart rebuilds a pinned local source package and starts owners. During that operation, status returns `state:"starting"`, `startup_phase` (`checking_source`, `building_source`, `validating_graph`, or `starting_owners`), and null `run_id` and `active_revision`; it does not expose a prior run as the active startup identity. The shared log stream records source build start, success, cancellation, or failure, followed by graph validation and owner startup. A Stop request during source build cancels and reaps that build before returning a confirmed Stop result, and status reports `state:"stopping"` until the operation completes. The source package is still rebuilt and verified before EOS or EI Run. Simultaneous read-only status polls for the same deployment share one pending manager request. A partially started or unavailable owner is reported as `state:"degraded"` with process details after the command completes. A stopped or unresponsive manager returns 503. Clients use `run_id` and `active_revision` to fence each read and refresh after a command reply.

<a id="http_runtime--get-apiisosv1runtimestatusws"></a>
### GET /api/isos/v1/runtime/status/ws

The browser upgrades this read-only route to WebSocket. The same loopback Host and approved Origin admission as the HTTP API applies at upgrade; there are no query parameters, client messages, or credentials. The first event is sent immediately. Each `snapshot` contains the raw `data` shapes of [process status](#http_runtime--get-apiisosv1runtimeprocess-status) and [system health](#http_version--get-apiisosv1systemhealth), without HTTP envelopes:

```json
{
  "type": "snapshot",
  "sequence": 1,
  "process_status": { "schema_version": "0.1.0", "state": "stopped", "run_id": null, "processes": [] },
  "system_health": { "service": "isos", "api_schema_version": "0.1.0", "state": "ready", "owners": [] }
}
```

IsOS probes at most every 200 ms and sends a new event only when either projection changes. `sequence` is a positive integer starting at one per connection, not a durable cursor. If a projection fails, the server sends `type:"error"` with the next sequence and `error:{"code":"status_unavailable","unavailable":["process_status"]}`; the `unavailable` array may contain `process_status`, `system_health`, or both. Recovery sends a complete snapshot. A WebSocket ping occurs every 15 seconds. Closing the socket ends its probes; clients reconnect and use HTTP status routes as fallback. These snapshots report state, not graph admission, readiness, or Stop proof.

<a id="http_runtime--post-apiisosv1runtimecontrol"></a>
### POST /api/isos/v1/runtime/control

Request `{"schema_version":"0.1.0","operation":"stop","revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}`. `operation` is `run`, `start`, `stop`, or `restart`; `revision` is an optional exact saved-byte SHA-256 fence. Successful `data` is the manager's operation result. A stale revision returns 409; owner failure or uncertain stop returns a structured fault and never claims success. This route cannot bypass EOS actuator admission.

<a id="http_runtime--get-apiisosv1runtimelogs"></a>
### GET /api/isos/v1/runtime/logs

Reads bounded sanitized JSONL with query `source`, `cursor`, `limit`, `level`, and optional current-run `view`. Example request: `GET /api/isos/v1/runtime/logs?source=all&limit=100&level=info`; `data` contains `lines`, `next_cursor`, and `rotated`. EOS and EI sources are read from IsOS's durable owner log pools, so a new UI connection can page through retained startup and runtime records. `limit` is at most 200 records and 64 KiB per call; a rotated cursor is explicit. See [log cursors](../manager.md#manager_log_cursors) for current-run anchors and gap diagnostics.

### GET /api/isos/v1/runtime/logs/ws

The browser upgrades this read-only route to WebSocket to follow current-run logs. The route accepts the same `source`, `level`, `node_path`, `correlation_id`, `cursor`, and `limit` query fields as the current log read. A `page` message contains the same current log page as the HTTP `data` field, including its opaque `next_cursor`, scope, anchor, and gap diagnostics. The server advances the cursor and emits pages when rows, scope, or anchor change; it probes at most every 200 ms and pings every 15 seconds. A failure emits `{"type":"error","status":503,"error":{"code":"log_scope_unavailable"}}`. The same loopback Host and approved Origin admission applies. The client closes the socket when the log panel is hidden and reconnects from its last cursor. HTTP remains available for bounded historical reads and environments without WebSocket support.

`schema_version` is string/— and must equal `0.1.0` on mutations; `operation` is string/— and required; `revision` is optional string/SHA-256, absent only when caller accepts current saved revision; `cursor` is opaque string/— and never parsed by clients; `level` is one of five shared severities. These fields separate a saved edit from the active run and keep log reads bounded. Stop, readiness, and errors are observable through the manager result and the deployment event stream.

<a id="http_runtime--failure-example-and-field-contract"></a>
### Failure example and field contract

For `POST /api/isos/v1/runtime/control`, an illustrative structured failure is:

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "invalid_request",
    "message": "operation must be run, start, stop, or restart",
    "details": null,
    "correlation_id": "req-42"
  }
}
```

`operation` is required `run`, `start`, `stop`, or `restart`; `run` and `start` select the same manager start operation. `revision` is an optional exact saved-byte fence. `cursor` is opaque, and `limit` is a bounded record count. A successful control response contains the manager operation data inside the shared envelope. The example correlation ID is generated per failure; see [HTTP](http.md#http_envelope) for common envelope fields and status mapping.

<a id="http_version"></a>
<a id="http_version--version-and-health-api"></a>
## Version and health API

[中文](../../../../docs_cn/isos/interfaces/api/http_cn.md#http_version)

**Implemented on the IsOS side.** IsOS owns the local compatibility identity and owner connection overview. Both routes require loopback Host/Origin admission and use the shared schema-0.1.0 structured HTTP envelope. Neither route launches another core.

<a id="http_version--get-apiisosv1systemversion"></a>
### GET /api/isos/v1/system/version

Returns the independent API schema, IsOS package version, and bundled local version-support map. Example response: `{"schema_version":"0.1.0","ok":true,"data":{"api_schema_version":"0.1.0","isos_version":"0.2.3","version_support":{"schema":"isos.version-support@v2","product_version":"0.2.3","target_graph_format":"0.2.1","isos_http_schema":"0.1.0","package_contract":"isos.package@v1","package_schema":"0.2.1"}},"error":null}`. The version map is compiled from `config/version-support.toml` and does not contact GitHub. Package inventory and exact release metadata come from the IsOS package manager, not this build map. Clients reject an unsupported `api_schema_version`; a product version is not a wire compatibility test. A rejected Host or Origin returns a safe structured 403 error.

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `api_schema_version` | string / contract version | Exact current IsOS HTTP envelope schema, `0.1.0`. | Required. |
| `isos_version` | string / product SemVer | Installed IsOS source package version; independent of HTTP. | Required. |
| `version_support.schema` | string / identity | Bundled map contract, `isos.version-support@v2`. | Required. |
| `version_support.product_version` | string / product SemVer | Must match the built IsOS package version. | Required. |
| `version_support.target_graph_format` | string / graph format | Target graph format for this build; `0.2.1`. | Required. |
| `version_support.isos_http_schema` | string / contract version | Must match this response's HTTP schema. | Required. |
| `version_support.package_contract`, `.package_schema` | strings / package contract | Exact manifest identity and schema accepted by the current package reader. | Required. |


<a id="http_version--failure-example-and-field-contract"></a>
### Failure example and field contract

For `GET /api/isos/v1/system/version`, an illustrative structured failure is:

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "unauthorized",
    "message": "unauthorized",
    "details": null,
    "correlation_id": "req-42"
  }
}
```

The route has no request body and no query fields. The example correlation ID is generated per failure; see [HTTP](http.md#http_envelope) for common envelope fields and status mapping.

<a id="http_version--get-apiisosv1systemhealth"></a>
### GET /api/isos/v1/system/health

Returns IsOS `state:"ready"` and a bounded `owners` array for EOS and EI. Each available entry has `core`, `state`, and nullable `run_id` and `revision` from the owner's private `GET /v1/health`; an unavailable entry has `core`, `state:"unavailable"`, and a reason. An idle owner has null run identity. A selected owner's identity is surfaced before IsOS requests its catalog or other inner configuration. This quick overview does not claim deployment readiness or Stop proof; use runtime process status for the fenced run. A slow or absent owner is shown as unavailable while the IsOS HTTP server remains responsive.
