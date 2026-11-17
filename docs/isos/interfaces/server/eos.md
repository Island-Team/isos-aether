# IsOS–EOS server interface

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/server</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS owner server interfaces](README.md)
  - [IsOS–EI server interface](ei.md)

</details>
<!-- doc-navigation:end -->

## Table of contents

- [EOS configuration projection](#configuration--eos-configuration-reference)
- [EOS control admission](#control--eos-control-admission)
- [EOS local JSONL log contract](#events--eos-local-jsonl-log-contract)
- [EOS gRPC and owner IPC](#grpc--eos-grpc-and-owner-ipc)
- [EOS package node contributions](#node_contributions--eos-package-node-contributions)
- [EOS owner lifecycle](#owner_health--eos-owner-lifecycle)
- [Package observer runtime](#package_observer--package-observer-runtime)
- [Owner package process handoff](#package_process--owner-package-process-handoff)
- [EOS reader identity](#reader_features--eos-reader-identity)
- [EOS setup exchange](#setup_v1--eos-setup-exchange)


<a id="configuration"></a>
<a id="configuration--eos-configuration-reference"></a>
## EOS configuration projection

[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#configuration_cn) · [IsOS graph](../../../node/interfaces/graph.md#instance) · [EOS lifecycle](eos.md#owner_health)

IsOS supplies EOS only the selected `cores.eos` projection of the saved graph and the exact verified package pins. EOS reads that projection without editing it. Each registered EOS type declares placement, settings, ports, capabilities, and lifecycle; instance values and typed edges select devices and routes. IsOS rejects an unknown pin or resource before launch, and EOS validates its own route and adapter semantics before reporting readiness.

A primitive output connects to exactly one compatible actuator input. Dispatcher checks the active revision, target, capability, health, limits, deadline, and Stop state before release. Camera routes are read-only. The [EOS design](../../../eos/core/architecture.md) describes generated files and process startup.

<a id="control"></a>
<a id="control--eos-control-admission"></a>
## EOS control admission

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](../../../eos/interfaces.md#arm_control) for the active data and command path and [owner lifecycle](eos.md#owner_health) for setup, Run, Status, and Stop.


[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#control_cn) · [Primitive bindings](../../../eos/interfaces.md#primitive_bindings) · [Rust skill contract](../../../eos/interfaces.md#skills_rust)

The current graph has separate `isos`, `ei`, and `eos` cores and uses the owner activation route in [arm control](../../../eos/interfaces.md#arm_control). The older loopback primitive-binding route admits movement through `isos.eos_dispatcher.primitive_base@v0.2` and `isos.eos_dispatcher.primitive_manipulator@v0.2` bindings. Each binding targets one compatible hardware node through a typed edge. Dispatcher is the only actuator route. Rust skill workers request primitives through the same broker; graph-authored action sequences are not part of this contract.

<a id="control--loopback-api"></a>
### Loopback API


The HTTP bridge listens at `127.0.0.1:16020` and forwards bounded JSON to EOS over its private Unix socket. Requests use `POST /api/eos/v2/control` with `Content-Type: application/json`; bodies are limited to 64 KiB. Browser calls require an approved `Origin`: `http://127.0.0.1:<ISOSUI_PORT>` and `http://localhost:<ISOSUI_PORT>` (default 16001), `http://127.0.0.1:<ISOS_HTTP_PORT>` (default 16000), `https://www.os-island.com`, or `https://os-island.com`. The bridge answers approved CORS/private-network preflights. The bridge allowlist excludes raw actuator, result and shutdown routes for browser and local HTTP callers. The EOS R03 private socket also rejects raw `manual`, `primitive`, `skill_execute`, `skill_result`, `dispatch`, `stop`, `manual_stop`, `cancel`, `result` and `skill_cancel` routes; internal owner-only `shutdown` remains. Every request must use the loopback bridge address.

For public `primitive_typed`, the browser supplies `header.payload_length_bytes` as a positive 1–16,384 byte compact UTF-8 payload length. The bridge parses the bounded public JSON, requires that field and a payload, rejects a declared or parsed payload length outside 1–16,384, then replaces the private header length with `serde_json`'s compact serialization length of the parsed payload before Unix IPC. The public declaration is a bounded hint rather than byte-exact proof across serializers; the private header is authoritative. The private runtime still requires an exact match with its own parsed-payload serialization and rejects malformed typed commands before Dispatcher. This bridge normalization is necessary because JavaScript and Rust render some finite JSON numbers differently, such as `0.000001` versus `1e-6`; the browser cannot use a wall-clock or string-character count to infer the private bytes. A missing, noninteger or out-of-range public length, or an absent payload, returns HTTP 400 with `{"ok":false,"error":"invalid typed request"}` before reading the grant. The bridge does not change typed payload values, graph binding, target, revision, command ID, deadline or grant checks.

<a id="control--r07-managed-browser-submission"></a>
### R07 managed browser submission

**R07 scoped implementation merged and qualified; evidence is recorded in the shared TODO.** For a managed, authorized `primitive_typed` request, EOS returns a correlated `DispatchResult` with `InProgress` only after the Dispatcher has durably reserved that exact command and accepted one bounded execution slot. No new public field or operation is added. The bridge's existing two-second private I/O bound and the browser's four-second timeout apply to this acknowledgement, not to the later physical call. EOS may return a terminal result instead if validation, admission or execution finishes before the reply; the browser must honor that exact status. A transport timeout or lost acknowledgement is **submission outcome unavailable**, never an invitation to resend movement. The browser retains the generated ID and may make only the authorized, read-only `result` request under the same current IsOS run and active revision. An `InProgress` acknowledgement proves durable reservation, not actuator release or physical completion.

EOS admits at most eight managed browser movement submissions that have returned or may return `InProgress`, counting workers waiting for the release gate and workers inside an adapter call. This limit is separate from private-socket client and safety-stop capacity; a busy movement lane cannot consume the safety path. For a new ID, EOS checks the exact active binding, target, typed payload limits, grant, run, revision and current deadline before reservation, obtains a slot, and durably writes `Prepared` before acknowledging `InProgress`. It rechecks live admission at the Dispatcher release gate, durably writes `Releasing` before one adapter call, and persists one terminal `Completed`, `Failed`, `Cancelled`, `Rejected` or `OutcomeUnknown` result before releasing the slot. If all eight slots are occupied, EOS records a terminal `Rejected` result for a valid new ID **in one atomic journal insert** containing the full command signature and managed identity, `phase:Terminal`, and `accepted:false`; there is no intermediate `Prepared` record, no worker and no adapter call. The Dispatcher implements this distinct crash-safe transition as `Journal::insert_terminal_rejection`, separate from the `Prepared` insert. If it cannot persist that rejection, EOS returns `{"ok":false,"error":"movement capacity unavailable"}` without acknowledging or releasing the command. An expired deadline or invalid admission is rejected before physical release. This asynchronous path still enters the existing Dispatcher; it does not create a second actuator route or change gRPC movement semantics.




If a queued worker decides **before** `Releasing` that its deadline expired, health failed or `Closing` began, but writing the terminal result fails, the durable record remains `Prepared`. EOS fences that ID against `Releasing` and every adapter call, latches movement inhibit for the live generation, keeps its slot until a durable seal or generation exit, and makes authorized result reads and identical duplicate submissions return `{"ok":false,"error":"result unavailable"}` instead of stale `InProgress`; a duplicate never creates a worker. It retries a durable terminal `Rejected` for expiry or health rejection, or `Cancelled` for Stop, under the journal lock, with Stop's `Cancelled` winning a race. A successful seal releases the slot and makes that terminal result readable. If the generation exits before sealing, normal journal recovery converts durable `Prepared` to terminal `Cancelled` without release before readiness; failure to persist recovery fails startup. Inject the pre-release terminal-write fault at deadline, health and Stop rejection, then test stale-read opacity, duplicate nonrelease, slot retention, durable retry and restart recovery.

<a id="control--r07-authorized-browser-result"></a>
### R07 authorized browser result



| Request field | Type | Unit | Purpose, default and validation |
| --- | --- | --- | --- |
| `op` | string | — | Required exact `result`; no default. |
| `command_id` | string | command identity | Required 1–128 UTF-8 bytes, matching the submitted typed header; no default. |
| `active_revision` | string | saved graph SHA-256 | Required 64 lowercase hex digits equal to EOS's current IsOS active revision; no default. |

```json
{
  "op": "result",
  "command_id": "move-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```


```json
{
  "ok": true,
  "result": {
    "command_id": "move-1",
    "status": "InProgress",
    "reason": null
  }
}
```


The public result route returns HTTP `400` with `{"ok":false,"error":"invalid result request"}` for malformed or extra public keys, and HTTP `403` with the existing bridge error for a forged `authority_grant`, invalid Host or disallowed Origin. Missing or unreadable bridge grant returns HTTP `503` with `{"ok":false,"error":"operator control unavailable"}` before IPC. A private grant rejected by EOS returns HTTP `200` with `{"ok":false,"error":"operator control grant required"}`; a stale revision returns HTTP `200` with `{"ok":false,"error":"stale active revision"}`; opaque IDs return HTTP `200` with `{"ok":false,"error":"unknown command ID"}`. The bridge returns HTTP `502` with `{"ok":false,"error":"runtime unavailable"}` for private IPC transport failure. These result-route errors are bounded fixed strings; no private grant or journal data is echoed.

<a id="events"></a>
<a id="events--eos-local-jsonl-log-contract"></a>
## EOS local JSONL log contract

[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#events_cn) · [EOS interfaces](../../../eos/interfaces.md#readme) · [Owner lifecycle](eos.md#owner_health)

EOS writes one local JSONL file and a readable `eos.log` companion per deployment: `data/eos/log/<deployment>/eos.jsonl`. Standalone EOS uses `data/eos/log/local/eos.jsonl`. IsOS supplies absolute `EOS_LOG_DIR` to the persistent owner and reads this file for the log panel. Logs do not cross owner Run, Status, activation, or Stop replies. The current graph starts the owner and one arm worker; the worker applies admitted deltas to the mock arm.

Each UTF-8 line is one JSON object with these required fields:

| Field | Value |
| --- | --- |
| `timestamp` | UTC RFC 3339 timestamp |
| `level` | Lowercase `debug`, `info`, `warn`, or `error` |
| `source` | `eos.<layer>.<group>.<node>@aether-v0.2.3` for a node, or `eos.<source-file>@aether-v0.2.3` for a system process |
| `message` | Human-readable diagnostic text; no embedded unescaped newline |

Run-scoped records include `run_id`. The mock source is `eos.hardware.mock.arm@aether-v0.2.3`. Owner and worker system sources are `eos.aether_profile.owner@aether-v0.2.3` and `eos.aether_runtime.worker@aether-v0.2.3`. A Run or Stop failure includes the owner error code in `message`. The owner emits one worker-loss error when health or status first observes it. Logs provide diagnostics; owner lifecycle and activation replies determine readiness, command outcome, and Stop confirmation.

Retained bridge/config binaries write bounded diagnostics into this same file through `LocalCoreLog`. They do not write a second `events.jsonl` file. The current graph does not launch those binaries.

<a id="grpc"></a>
<a id="grpc--eos-grpc-and-owner-ipc"></a>
## EOS gRPC and owner IPC

[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#grpc_cn) · [EOS interfaces](../../../eos/interfaces.md#readme)

The current owner exchanges use a private Unix socket. See [owner lifecycle](eos.md#owner_health) for setup, Run, Status, and Stop; [arm control](../../../eos/interfaces.md#arm_control) and [arm monitor](../../../eos/interfaces.md#arm_monitor) define the current data routes. The earlier public gRPC surface is a target design in the [EOS architecture](../../../eos/core/roadmap.md#target-architecture-not-implemented).

<a id="grpc--r02-frozen-rust-subset"></a>
### R02 frozen Rust subset

The former R02 gRPC subset is outside the current demo graph. Its archived design remains in the [runtime gateway design](../../../eos/core/gateway/grpc-gateway.md).

<a id="grpc--r03-governed-actuator-admission-target-contract"></a>
### R03 governed actuator admission target contract

The current activation enters EOS through its fenced owner exchange and dispatcher. The earlier R03 transport design remains in the [admission design](../../../eos/core/dispatcher/dispatcher.md#r03-governed-actuator-admission-design).

<a id="node_contributions"></a>
<a id="node_contributions--eos-package-node-contributions"></a>
## EOS package node contributions

> Graph `0.2.1` uses this registry at EOS setup and Run. The two built-in demo skill nodes and contributed native nodes use the Linux ABI below; the mock-arm actuator remains an EOS adapter.


[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#node_contributions_cn) · [Unified hardware profile](../../../eos/interfaces/node/hardware/mock.md#hardware_mock--arm) · [Primitive bindings](../../../eos/interfaces.md#primitive_bindings)

For package schemas `0.2.0` and `0.2.1`, `node_contributions.libraries` binds a safe package-relative `.so` path to its SHA-256 digest. EOS resolves the path under the verified package payload (including a verified local link), checks containment and bytes, and rejects duplicate type or library IDs. An EOS type declares `implementation.kind` as `native_plugin` with ABI `isos_node_v1` or `isos_node_v2` and a declared library, or `external` with interface, version `1`, and lifecycle adapter. The owner setup uses schema `3` and publishes its accepted `port_routes` and `bridge_adapters` with the catalog.

The Linux [v2 ABI](../../../../contract/node/isos_node.h) separates `observe` and `command` from `stop`, and requires a bounded run-bound stop proof through the host callback. Its create JSON carries `run_id`, `revision`, and the selected instance; a package service instance selects v2 and receives its private `adapter.settings.socket_path` in that instance. The EOS owner validates each declared command port and payload, durably reserves its command ID, then forwards it to the worker with a private run token. A duplicate ID returns the recorded result only for the same node, port, and payload; an ambiguous delivery inhibits further commands. Stop closes admission before the worker calls every v2 Stop callback. `stop` succeeds only when its proof repeats the run ID and revision with `outcome:"stopped"`. The old v1 ABI remains available to old package pins. macOS and Windows library loaders are future work.

At Run, IsOS supplies `endpoints.package_services` keyed by the exact graph node path. Each entry carries an exact package `{id,version}`, `service_instance`, absolute private `socket_path`, nonempty `service_generation`, and SHA-256 `config_digest`. EOS checks the package pin and instance against the saved node before injecting the endpoint into the worker instance. A graph-authored endpoint or generation is rejected. The owner-only `POST /v1/command` accepts `{schema_version:1,core:"eos",op:"command",run_id,revision,node,port,command_id,payload}` for a declared service-bound actuator port. It validates the typed payload, checks base frame and arm joint dimensions against the applied node, and routes it through the run's Dispatcher. `aether-runtime --probe-adapter <absolute .so>` reports the ABI it actually found, preferring `isos_node_v2` when both symbols exist.

The native ABI is defined by [isos_node_v1.h](../../../../contract/node/isos_node_v1.h). A native worker calls create, start, input, health, stop, and destroy in that order, fences input by run ID and graph revision, and requires successful Stop before unloading. A native graph worker loads up to 64 contributed software instances and routes emitted `{port,payload}` envelopes along registered in-process edges, with a bound of 256 deliveries per input. It validates each emitted port and typed payload against the verified output declaration before delivery. The owner accepts input at `/v1/input` with `source_node`, `source_port`, `node`, `port`, and `payload`, checks the exact authored input link and typed payload, and admits only declared EI or IsOS source nodes; EOS-local worker links are owner-managed. For declared EOS→EI edges, it forwards emitted payloads to the EI owner's `/v1/input` with the source and target paths and the same run and revision. Service-bound actuators use the run's Dispatcher; unbound actuators are rejected. The two demo skill types load the shipped sibling `libaether_runtime.so`; the mock-arm worker passes their output through its existing dispatcher and hardware adapter. A complete demo chain can run beside native nodes under one owner. EOS rejects an incompatible callback table or changed library digest.

For a Dispatcher-admitted v2 actuator `op:"command"`, the worker accepts exactly one private `{port:"command_result",payload:{accepted:<boolean>,...}}` from the targeted node and returns it with that exact node path to the owner. It does not require a graph output declaration and never routes this result on graph edges. A missing, duplicate, wrong-port, or malformed result fails the command. `command_result` is reserved: graph output declarations and edges using that port are rejected, and ordinary graph input or observation cannot emit it.

The owner gives the native worker a private verified `output_types` list of `{node,port,data_type}` for each declared native output, plus full `data_types` descriptors for referenced contributed types. Before queuing any local edge or returning an emission, the worker requires an exact output-port declaration and validates the payload against that type's bound and schema. An invalid emission fails the input without invoking downstream nodes.

Setup validation and Run use the same execution admission check. EOS compiles a route plan from the validated projection and registered `port_routes`: each executable EOS-local link retains its exact source node, output, target node, input, and data type. Declaration-only links stay out of the executable plan. The complete eight-node mock-arm chain stays on its dispatcher worker, contributed native nodes and service adapters are hosted by the native worker, and `vision_flow` is an owner bridge. A mixed graph is admitted only when the complete demo chain remains intact and every added executable local link has a registered route. The mock worker exposes only `arm_observer/observation` to other hosts and has no generic ingress from native nodes or owner bridges. An unregistered route, command link outside the fixed demo Dispatcher chain, unsupported cross-host transport or demo ingress or output, partial demo chain, unavailable implementation, or contributed actuator without dispatcher integration receives an `unsupported_execution` diagnostic before Apply or Run. Package service nodes remain dependencies of the native worker rather than independent graph-routing hosts.

The [cross-backend design](../../../eos/core/roadmap.md#cross-backend-graph-execution) specifies worker exchange and lifecycle behavior. The owner starts both workers under one run identity, requires both to become ready, and delivers the mock observer's typed output along exact admitted links to the native worker. The current v2 service ABI supports observation and command callbacks but has no typed input callback, so a v2 service can produce an observation for a v1 software consumer; it cannot itself consume a typed software input. The existing node-keyed v2 package-service handoff and legacy single-process EOS handoff remain distinct.



IsOS installs or links Aether packages and owns the applied graph. A package release may contribute registered node types and node sets; IsOS verifies the release and writes a derived, read-only node-type sidecar for EOS. EOS reads the graph and sidecar without changing either. This JSON interface is the EOS-specific node IDL; it describes identities, placement, settings, and typed ports, while hardware samples and commands remain Rust values inside EOS. The example below uses current registered node identities and typed ports.

<a id="node_contributions--verified-node-type-sidecar"></a>
### Verified node-type sidecar

IsOS passes each verified sidecar as a repeatable `--node-types <absolute-path>` argument to `aether-profile` and `aether-server`; the persistent EOS owner reloads selected sidecars from verified package references at setup and Run. The worker receives only the selected library path. Each sidecar is at most 1 MiB. EOS loads all safety declarations before the corresponding types, rejects duplicate or malformed definitions, and validates the graph against the combined built-in and contributed registry. All enum values use `snake_case`; unknown fields fail decoding.

The following sidecar shape illustrates an unrelated EOS package; the complete type schema is linked below.

```json
{
  "package": {"id": "vendor/camera-driver", "version": "1.2.3"},
  "safety_adapters": [],
  "data_types": [],
  "libraries": [{"path":"lib/camera.so","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}],
  "adapter_families": [],
  "node_types": [{
    "id": "eos.hardware.camera.driver@v0.2",
    "core": "eos",
    "layer": "hardware",
    "group": "camera",
    "kind": "camera",
    "lifecycle": "adapter",
    "description": "Vendor camera driver",
    "capabilities": [],
    "settings": [],
    "inputs": [],
    "outputs": [],
    "implementation": {"kind": "native_plugin", "abi": "isos_node_v1", "library": "lib/camera.so"}
  }]
}
```

`package` is an exact `{id, version}` reference. The ID has two valid address segments separated by `/`; version has three nonempty numeric components. A contributed node instance must carry the same `node.package` value as the type's sidecar. Built-in nodes omit `node.package`. Registration checks a declared stop capability, but EOS admits a physical actuator only when a compiled adapter and stop implementation exist for its type and package.

`node_types` is an array of `NodeType` definitions from [the hardware contract](../../../eos/interfaces/node/hardware/mock.md#hardware_mock--arm); `safety_adapters` is an array of `SafetyAdapterType`. Their Rust and JSON fields are identical. `core`, `layer`, and `group` are required placement segments; the graph cannot infer hardware kind from a group. The `kind` values are `camera`, `base`, `manipulator`, `gripper`, `torso`, and `software`; lifecycle values are `internal`, `process`, and `adapter`. IsOS package node sets use these type IDs and symbolic graph references, but the applied graph contains only concrete paths.

`data_types` is an optional array of `DataTypeDescriptor` entries. Each entry has `id` (string, unique versioned identifier such as `vendor.camera.depth@v0.2`), `version` (string, `0.2`, matching the ID suffix), `purpose` (nonempty description string), `media_type` (nonempty MIME string), `codec` (nonempty registered runtime codec ID), `schema` (JSON object describing logical fields, including `description`, `x-unit`, and JSON Schema validation keywords), and `max_payload_bytes` (positive integer, bytes). Built-in IDs remain snake case, such as `camera_frame` and `vision_frame`. A contributed port can reference a data type only when its codec is compiled into EOS. `json_v1` and `vision_frame_jpeg_v1` are built-in codecs. EOS exposes the combined catalog through `aether-profile catalog` and `GET /api/eos/v2/data-types` on the config server.

An edge requires the exact same registered `data_type` ID at its source and input. Its schema's `x-unit` annotations describe payload fields for display and validation within that type; they do not make two different type IDs interchangeable. No implicit unit conversion occurs on a graph edge. A contributed descriptor cannot redefine a registered ID with different units; duplicate IDs fail registration. IsOS owns edge-addressed Apply diagnostics, while EOS owns runtime health after launch. The [hardware edge contract](../../../eos/interfaces/node/hardware/mock.md#hardware_mock--arm) defines source QoS, route classification, transport and safety restrictions. A package-contributed port cannot claim a command route merely by setting `data_class: "command"` or matching a built-in movement type: a hardware command input accepts only the registered Dispatcher primitive-binding source.

`adapter_families` is an optional array. Each descriptor requires `id` (valid family segment), `device_kinds` (nonempty list of `camera`, `base`, `manipulator`, `gripper`, or `torso`), `transport` (one of `in_process`, `unix_socket`, `websocket`, or `ros2`), and `description` (nonblank string). Optional `settings_schema` is a JSON object; optional `supported_primitives` is an array of registered `<name>@<version>` strings. The sidecar's outer `package` supplies exact provenance. Package authors cannot set `compiled`; EOS derives it from its compiled adapter factory and exposes `compiled` and `reason` in the catalog. Generic hardware instances carry this package in `node.adapter.package`, independently of the built-in node type.

Each input or output port requires `name` (graph-local string), `data_type` (registered ID), and `required` (boolean). Optional `description` explains its purpose; `frequency_hz` is a positive finite rate in hertz; `transport` may be `in_process`, `unix_socket`, `websocket`, or `ros2`; `data_class` may be `image`, `observation`, `command`, `event`, `state`, or `opaque`; and `cardinality` is `one` or `many` (default `one`). Edge validation uses cardinality, transport, and data class. Camera package authors may declare the official output frequency on the camera frame port. For a v1 or v2 native Vision source, `node.adapter.settings.frequency_hz` on the graph instance must be within `(0, 1000]` Hz to schedule capture into the source pool; the official output frequency alone does not schedule capture. When omitted, EOS passes samples without an official frequency comparison until a later node declares one; EOS does not infer it from `observation.max_rate_hz`, which is only a ceiling. For an output port, optional `frequency_hz` is the registered official frequency for safety and diagnostics, not a delivery cap; a processing node declares its own output frequency even when it differs from its input. A graph node input may use `{"sources": <source object or array>, "desired_rate_hz": {"min": <Hz>, "max": <Hz>}}` in place of the legacy direct source object or array. Each desired bound is optional and must be finite and positive, with `min <= max` when both are present. The minimum is a preference and cannot create source frames; IsOS derives the edge delivery cap from `max`. Omitted bounds add no cap. Every `@v0.2` type declares transport and data class for its ports. Runtime data identity is the exact registered `data_type` ID. See the [vision interface](../../../eos/interfaces/node/interface/egress.md#interface_egress--vision) for JPEG codec and read-only transport.


<a id="node_contributions--typed-data-identity"></a>
### Typed data identity

EOS may wrap an in-process hardware value in `NodeDataEnvelope<T>`. The envelope carries `source_node` (canonical graph path), `port` (registered output name), `data_type` (`PortDataType`), `contract_version` (`2.0.0`), and the typed `payload`. `NodeDataEnvelope::output` rejects an output port whose declared type differs from `T::DATA_TYPE`. The envelope is metadata for identifying data against the IsOS graph; it does not grant command admission or create a second actuator path.

`TypedDataHub` publishes such envelopes to registered output keys. `subscribe<T>` validates the node's output port and returns a typed cursor; `poll_latest` returns only a newer value. The hub retains one value per output, so a slow subscriber skips intermediate values without a growing queue. Dropping the cursor disconnects that reader. A compiled `DataTransformer<Input, Output>` names its codec ID and accepts typed options; `CameraJpegTransformer` converts `camera_frame` to `vision_frame` with `vision_frame_jpeg_v1`.

Invalid sidecar JSON, duplicate type IDs, missing safety adapters, package mismatches, unknown types, placement mismatches, and incompatible port edges fail validation before runtime construction. Missing compiled actuator implementations fail runtime adapter construction before the EOS socket binds.

<a id="node_contributions--type-descriptions"></a>
### Type descriptions

New contributed `@v0.2` node types require a nonblank `description` explaining the node's purpose, inputs, outputs, and operator-visible effect. The EOS catalog displays it. The built-in types also provide this field. Registration rejects a blank description, duplicate type ID, mismatched package pin, or unsupported codec before Apply. The outer `package` and adapter-family descriptions keep their existing meanings.

<a id="node_contributions--target-setting-selector"></a>
### Target setting selector

`Setting.selector` is an optional JSON object on new type definitions. It describes where IsOS obtains options; it never embeds executable code or an arbitrary filesystem path. Existing `name`, `data_type`, `required`, `description`, `unit`, bounds, and static choices retain the rules in [hardware](../../../eos/interfaces/node/hardware/mock.md#hardware_mock--arm).

| Field | JSON type and unit | Purpose, default, validation, compatibility |
| --- | --- | --- |
| `provider` | string enum, — | Required when `selector` exists: `package`, `registry`, or `graph`; absent `selector` keeps the ordinary editor. |
| `resource` | nonblank string ID, — | Required option family, such as `scene`, `device`, `adapter_family`, or `node`; resolved only by the selected provider. Unknown families fail registration. |
| `package_from` | optional string enum, — | Required for `provider=package`: `node.package` or `node.adapter.package`; forbidden otherwise. It selects the exact verified pin in the draft graph. |

```json
{
  "name": "scene",
  "data_type": "string",
  "required": true,
  "description": "Verified package resource ID",
  "selector": {
    "provider": "package",
    "resource": "scene",
    "package_from": "node.package"
  }
}
```


<a id="node_contributions--structured-setting-metadata"></a>
### Structured setting metadata

A registered `object_array` setting declares an `items_schema` and entry bounds. The registry validates its shape and size; IsOS validates package pins and executable paths. The EOS process node's `settings.skill_executables` selects verified Rust `.skill` files. Each item identifies an exact package and relative executable. The [Rust skill contract](../../../eos/interfaces.md#skills_rust) defines the selection handoff and worker boundary.

<a id="owner_health"></a>
<a id="owner_health--eos-owner-lifecycle"></a>
## EOS owner lifecycle

[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#owner_health_cn) · [EOS interfaces](../../../eos/interfaces.md#readme) · [Arm control](../../../eos/interfaces.md#arm_control)

The persistent `aether-profile owner --socket <absolute-path>` process owns EOS setup and its direct `aether-runtime` worker child. IsOS connects over a private Unix socket. The owner can survive an IsOS process exit. Both owner and worker produce local JSONL records under the supplied absolute `EOS_LOG_DIR`; a worker loss emits one error record when observed. IsOS first reads the owner's small `GET /v1/health` reply and checks its run identity before asking for setup catalogs or other inner configuration.

<a id="owner_health--setup-and-graph-projection"></a>
### Setup and graph projection

The owner transport uses schema version `1`. Setup requests use `POST /v1/identity`, `/v1/catalog`, `/v1/options`, `/v1/validate`, and `/v1/render` with `core:"eos"` and matching `op`; setup replies use schema version `3`. The graph projection has format `0.2.1` and one `cores.eos` entry. The catalog contains the eight live demo nodes and the three payload types described in [arm control](../../../eos/interfaces.md#arm_control) and [arm monitor](../../../eos/interfaces.md#arm_monitor). Removed health wrappers and `topology_link` are rejected as unknown types.

<a id="owner_health--run-status-and-stop"></a>
### Run, Status, and Stop

`POST /v1/run` requires `{schema_version:1,core:"eos",op:"run",run_id,revision,graph,launch}`. `run_id` is a bounded ASCII identifier and `revision` is a 64-character hexadecimal digest. The launch SHA-256 must match the sibling `aether-runtime` executable; args and env are empty. The owner starts `aether-runtime --serve <private-socket> --run-id <id> --revision <digest> --joint-limit <rad> --source-node <path> --joint-names <csv> --sample-interval-ms <ms> --freshness-timeout-ms <ms>` and waits for readiness. A run conflict fails closed.

For an admitted graph containing the complete mock-arm chain and native or package nodes, Run starts the native graph worker and a separate mock-arm worker under the same run ID and revision. Both workers must report ready before Run succeeds. Activation, result lookup, and monitoring go to the mock worker; native input, observation, vision, and package adapter calls go to the native worker. The owner polls the mock observer every 20 ms through its fenced `observe_output` exchange and delivers the latest typed sample only along exact admitted observation links into the native worker. Monitor and explicit observation reads may also trigger delivery; repeated sample sequences are skipped. This is latest-sample delivery rather than lossless streaming: when the mock sequence jumps, `observation_samples_skipped` in Status and health counts the missed sequences and the owner writes a run-scoped warning for the first gap and each additional 100 skipped samples. A caller cannot inject a payload through one of those owner-managed links with `/v1/input`. Cross-worker delivery is bounded to 256 outputs; native graph routing also caps cumulative queued and emitted bytes at 24 MiB each. A stale response, invalid payload, or delivery failure marks the run failed until Stop.

`POST /v1/status`, `POST /v1/monitor`, and `POST /v1/stop` require the same identity envelope. Monitor returns the latest run-fenced observation without actuating; see [arm monitor](../../../eos/interfaces.md#arm_monitor). Status and GET health preserve the live worker’s `ready`, `degraded`, or `stale` state; Status reports `mock_hardware:"failed"` after child loss. A degraded or stale worker stays available for read-only monitoring while movement admission fails closed. Stop closes admission, requests child exit, waits, and reaps it. If exit or socket cleanup cannot be confirmed, Stop returns `ok:false` and retains the run identity for retry. The child receives a parent-death signal if the owner crashes. Requests over 1 MiB, malformed HTTP, and unsupported operations fail closed.

`GET /v1/health` needs no run request body and returns `schema_version:1`, `core:"eos"`, `setup_schema_version:3`, `state`, and nullable `run_id` and `revision`. Both identity fields are null while idle; a selected run keeps its exact fields even when a worker exits or health degrades. `run_id` is a bounded ASCII identifier and `revision` is the lowercase 64-hex applied graph digest. IsOS may reattach only when both non-null values match its retained run receipt; an unexpected identity is a conflict, not permission to replace or stop that run. Health is a quick owner and identity snapshot; worker detail and Stop proof still require fenced Status and Stop.

<a id="owner_health--owner-diagnostic-page"></a>
### Owner diagnostic page

IsOS reads `GET /v1/logs?after=<decimal byte offset>` on the same private socket after the health probe and during operation. The response is `{schema_version:1,core:"eos",next_cursor,entries}`. `next_cursor` is a nonnegative byte offset into the owner's append-only `eos.jsonl`; `entries` contains original JSONL objects. At most 64 KiB of source bytes are read per call, and the cursor advances only past complete newline-terminated records. An absent file returns an empty page with cursor `0`; an offset past the current file length restarts at `0`. A malformed offset returns HTTP 400 with `invalid_log_cursor`, and an unreadable file returns HTTP 503 with `owner_logs_unavailable`.

The page includes all warning and error records and selected owner, Run, and health-worker information records. It omits routine low-level information. IsOS keeps a bounded, durable copy for later workbench connections; the owner file remains the source for replay after IsOS reattaches. A log page is diagnostic data and never substitutes for fenced Status or Stop proof.

In a mixed run, Status and health require both worker children to remain reachable and report the mock worker state in `mock_hardware`; loss of either worker reports a failed run. Stop closes Dispatcher admission, obtains the mock-arm Stop proof, obtains the native worker Stop proof, and only then stops selected package service children. A missing proof returns `stop_uncertain` and keeps the run fenced for retry.

Exact typed links between package nodes hosted by the native graph worker execute within that worker. The owner retains the existing IsOS-selected package pins and service handoffs; a package adapter cannot open a new actuator route through a graph link.

For each selected v2 service node, Run derives the expected service generation from the run ID, revision, package pin, and service instance, then sends the IsOS framed `health` request to its verified handoff socket for the selected `device_id`. Readiness requires an accepted reply with that generation and the `strict-socket-v1` feature. A missing socket or stale reply rejects Run as `service_not_ready` or `service_generation_mismatch`. Status and GET health repeat these probes within one 500 ms aggregate deadline, deduplicating nodes that share a service device; service loss or a failed native observation route latches the run as `failed` until Stop. This applies to every selected service node, including software nodes with a service handoff.

<a id="package_observer"></a>
<a id="package_observer--package-observer-runtime"></a>
## Package observer runtime

[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#package_observer_cn) · [EOS interfaces](../../../eos/interfaces.md#readme) · [General hardware](../../../eos/interfaces.md#general_hardware)

This page defines the older v1 observation-only package process path. The current v2 service command and Stop boundary is in [node contributions](eos.md#node_contributions) and [general hardware](../../../eos/interfaces.md#general_hardware). EOS may run a verified, observation-only package process as its direct child. IsOS supplies one exact process handoff and the selected adapter shared library. EOS rechecks their digests, binds a private Unix socket, expands only the declared argument placeholders, and starts the process before loading the adapter into its native node worker. The graph must contain the selected process node and camera node using the same adapter family. For this v1 path, base and manipulator package movement is unavailable; the v2 service path has its own Dispatcher and Stop proof.

The private owner route `POST /v1/observe` accepts schema 1, core `eos`, op `observe`, the active run ID and revision, and a declared camera node path. The native adapter receives `{"op":"observe"}`. EOS accepts only outputs on declared ports whose payloads match the registered data schema. It forwards declared EOS to EI edges. Unknown nodes, stale runs, changed bytes, invalid ports or payloads fail closed. `aether-runtime --probe-adapter <absolute .so>` checks the `isos_node_v1` table and required callbacks in a separate process; it does not start hardware.

EOS binds the process, adapter family, and camera node to the same exact package ID and version. It rechecks each selected sidecar against the installed receipt at Run. A failed worker readiness check rolls back both children; uncertain cleanup returns `stop_uncertain`. Camera payloads are bounded to 20 MiB, native output to 24 MiB, and the owner reply to 20 MiB.

Stop first requires every native adapter's `stop()` success and worker exit, then terminates and reaps the direct package child process group. EOS reports `stop_uncertain` if either proof fails. The observation-only adapter's `stop()` proves sampling has quiesced; actuator packages require a separate owner-specific physical Stop receipt before they can run.

The built-in camera type permits `rgb8` and `jpeg` capabilities. A package instance advertises the encoding it actually emits; EOS checks the `image_frame@v0.2` payload schema at the owner boundary.

<a id="package_process"></a>
<a id="package_process--owner-package-process-handoff"></a>
## Owner package process handoff

[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#package_process_cn) · [Interface catalog](../../../eos/interfaces.md#readme) · [IsOS process contract](../package/README.md#process) · [JSONL events](eos.md#events)

A package process is a registered `<core>.process.<group>.<node>@v0.2` type placed under the same core's `process` layer. The instance selects an exact `node.package` pin. IsOS verifies the release, contained executable, resource descriptors, and selected resource IDs before creating the run-bound `isos.package-process@v1` handoff. The selected owner must advertise and implement the named lifecycle adapter. An executable file alone does not grant launch authority.

The handoff includes the owning core, node path and type, exact package pin, verified entrypoint and resource paths, lifecycle adapter, run ID, saved graph revision, release digest, and payload digest. The owner revalidates its inputs, supervises its direct child, and reports readiness only after live health proof. Unknown fields, changed bytes, unsupported adapters, and cross-owner placements fail before launch. The common wire shape is defined by the [IsOS handoff contract](../core.md#core_handoff).

EOS closes actuator admission and obtains Stop proof before terminating an EOS-owned child. IsOS accepts a confirmed Stop only after the selected owner proof and direct-child exit agree. Missing or uncertain proof remains an unknown terminal outcome. Package child stdout and stderr are diagnostic output; bounded owner events carry authoritative run identity.

<a id="reader_features"></a>
<a id="reader_features--eos-reader-identity"></a>
## EOS reader identity

[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#reader_features_cn) · [Setup](eos.md#setup_v1) · [Owner lifecycle](eos.md#owner_health)

IsOS probes the selected EOS executable before Apply. `aether-profile --capabilities` reports source package version `0.2.3`, graph format `0.2.1`, node type version `0.2`, and four built-in EOS node types:

- `eos.interface.ingress.trigger@v0.2`
- `eos.skill.demo.joint_delta@v0.2`
- `eos.dispatcher.arm.delta@v0.2`
- `eos.hardware.mock.arm@v0.2`

The `binary` is `aether-profile`; features are `catalog`, `setup-v3`, `arm-activation-owner`, and `validate-file`. The catalog also provides `button_activation@v0.2` and `arm_joint_delta@v0.2` data types. Live health and activation results come from the [owner lifecycle](eos.md#owner_health) and [arm control](../../../eos/interfaces.md#arm_control).

`aether-profile validate --file <graph> [--node-types <verified-sidecar>]...` reads an EOS-only graph `0.2.1` JSON projection (at most 1 MiB), checks it against the merged registry, and applies the same executable-mode admission as setup and Run. It exits successfully without output only for an executable graph. A parse, type, topology, or unavailable-runtime error exits unsuccessfully with a diagnostic on stderr; it never starts a worker. IsOS supplies repeatable verified sidecar paths when needed.

<a id="setup_v1"></a>
<a id="setup_v1--eos-setup-exchange"></a>
## EOS setup exchange

[中文](../../../../docs_cn/isos/interfaces/server/eos_cn.md#setup_v1_cn) · [Owner lifecycle](eos.md#owner_health) · [EOS interfaces](../../../eos/interfaces.md#readme)

IsOS calls the persistent EOS owner over an owner-private Unix socket. The transport envelope uses `schema_version: 1`, `core: "eos"`, and an `op` matching `POST /v1/<op>`. Setup replies use schema version `3`; transport, setup, graph, and node type versions are independent. The one-shot `aether-profile setup --json` accepts the same setup content with schema version `3`.

<a id="setup_v1--operations"></a>
### Operations

`catalog.direct_only_types` lists EOS types with direct test handlers but no graph worker integration. These types are absent from `identity.supported_node_types`. A graph selecting one fails `validate` with `unsupported_execution` and a reason beginning `isolated_node_graph_runtime_unavailable`. The [isolated library contract](../../../eos/interfaces.md#isolated_library) defines their direct Rust API and the owner services still needed for runtime wiring.

`identity` returns product and supported graph and node type versions. `catalog` returns the built-in demo types and selected package contributions. `options` reads the incomplete EOS draft and returns one graph-scoped `adapter.type` group per registered hardware node, including package-contributed nodes. Each choice must match the node's exact package pin and device kind; it is available only when the EOS implementation is compiled or its verified library can load. An unselected adapter remains null; options do not make the draft runnable. `validate` checks the EOS-only graph projection and returns diagnostics without files. `render` additionally returns a validated `eos/graph.json` file for IsOS to publish atomically after checking the saved revision again. The one-shot command and persistent owner both load only the package sidecars named by the request's exact IsOS-verified `packages` references before type admission; an identity, payload-root, or contribution mismatch fails setup. Setup never launches a worker.

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "validate",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "graph": {
    "version": "0.2.1",
    "robot": {"id": "robot", "name": "Robot"},
    "cores": {"eos": {"layers": {"interface": {"groups": {"ingress": {"nodes": {}}}}}}}
  },
  "endpoints": {},
  "packages": []
}
```

The example shows the envelope shape; a valid graph needs at least one registered EOS node in each present group. The projection contains only `cores.eos`; its declared cross-core input may name an EI source admitted by IsOS. Invalid graph versions, missing EOS layers, unknown types, malformed settings, and incompatible local links return `ok:false`, bounded diagnostics, and no files. Requests and responses are limited to 1 MiB and 4 MiB respectively. The [owner lifecycle contract](eos.md#owner_health) defines Run, Status, Stop, and health.

A valid `validate` request returns an empty file list. For a malformed request, the diagnostic identifies the rejected field and code; EOS does not render a partial graph.

```json
{
  "schema_version": 3,
  "ok": true,
  "diagnostics": [],
  "files": []
}
```

```json
{
  "schema_version": 3,
  "ok": false,
  "diagnostics": [
    {
      "path": "request",
      "code": "invalid_json",
      "message": "setup request is not JSON"
    }
  ],
  "files": []
}
```

An absent registered required input is reported by IsOS as a warning. EOS validates connected edges and requires a supported executable mock-arm topology before Run. The mock worker may run with disconnected inputs, but its owner rejects activation until the complete declared command chain is connected and rejects monitor reads until the complete observation chain is connected. A missing edge never creates an implicit actuator route.
