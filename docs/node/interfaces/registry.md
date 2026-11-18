# Shared node registry interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: node/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Shared node interfaces](README.md)
  - [Shared node graph interfaces](graph.md)
  - [Shared node type interfaces](type.md)

</details>
<!-- doc-navigation:end -->

<a id="authoring"></a>
<a id="authoring--node-declaration-rules"></a>
## Node declaration rules

[中文](../../../docs_cn/node/interfaces/registry_cn.md#authoring) · [Profile](graph.md#profile) · [Type](type.md#type)

Graph format `0.2.1` requires exactly `isos`, `ei`, and `eos` cores. Each registered ID follows `<core>.<layer>.<group>.<type>@v0.2`; its instance must use the same core, layer, and group. The [starter graph](../../../crates/isos/templates/generic-graph.toml) is a representative declaration.

Choose a current execution path before writing the graph:

| Core | Supported contributed implementation | Current graph limit |
| --- | --- | --- |
| IsOS | `isos.ui.external.v1`, version `1`, `unix-socket-v1` direct child; [request/reply contract](../../isos/interfaces/node/README.md) | Its declared UI ports support independent input and read; graph edges to the node are rejected. |
| EI | Linux `isos_node_v1` `.so` or `ei.owner.stage.v1` external worker; [EI node contract](../../isos/interfaces/server/ei.md#node) | The EI safety stage remains mandatory for actuator delivery. |
| EOS | Linux `isos_node_v1` `.so` for software nodes and `isos_node_v2` for service-bound adapters; [EOS contribution contract](../../isos/interfaces/server/eos.md#node_contributions) | A complete mock-arm demo can run with contributed observation and software nodes on registered routes. Cross-package actuator commands are rejected; service-bound v2 actuators require the declared Dispatcher route. The v2 ABI has no typed software input callback. |

A contributed type declares versioned data types and exact ports as described in the [type interface](type.md#type). Its verified [package release](../../isos/interfaces/package/README.md#release) lists the type, native library or external process, and only routes the owner can deliver. An instance uses a matching map key and `id`, an exact `{id,version}` package pin, allowed settings, and required input references at `<core>/<layer>/<group>/<id>`. IsOS rejects a type without a verified implementation, mismatched placement or port, or an undeclared route.

The [graph API](../../isos/interfaces/api/http.md#http_graph) defines revision admission; [registry](registry.md#registry) defines package and owner claims. For creation, validation, and runtime commands, follow the [node first-use guide](../../isos/start/quick-start.md).

The starter graph declares typed control and observation edges. One activation travels through EI to EOS; EOS converts it to a bounded six-joint delta for the mock adapter. The same adapter's observation returns through EOS and EI to the UI monitor asynchronously. The mixed example leaves the control path without an activation source and also routes a package camera frame through EOS vision flow to a UI vision monitor. Unknown types, wrong placement, incompatible ports fail admission; missing required inputs produce warnings. The mock adapter has no physical actuator route.

The mixed demo's Vision chain is `front_camera` → `front_vision_flow` → `front_monitor`. The MuJoCo camera setting remains 1280×720 at 30 Hz with JPEG output. Both downstream desired maximum rates are 32 Hz; this permits 30 Hz capture through the millisecond delivery interval. These settings are not a guarantee of measured browser FPS.

A disconnected required input remains in the authored graph. Apply can save it, and the affected path cannot deliver data until the input is connected and the graph is applied again.

An input port may wrap its source as `{"sources":{"node":"<source-path>","output":"<port>"},"desired_rate_hz":{"min":10,"max":30}}`. Both rate bounds are optional positive finite hertz values; omit the wrapper to leave the port uncapped. IsOS derives the delivery cap from `max` and warns when the registered source output's optional official `frequency_hz` is below `min`. The minimum is a preference, not a frame generator. Edge `frequency_hz` is no longer an authored field. A source may omit the package output `frequency_hz`; samples then pass without an official frequency comparison until a later output declares one. For a source with configurable capture cadence, set `node.adapter.settings.frequency_hz` to a value within `(0, 1000]` Hz on the graph instance. For a v1 or v2 native Vision source, this setting is required to capture frames into the source pool; the official package frequency alone does not schedule capture, and without the instance setting no frames are pooled. This custom adapter setting controls that source but does not become the official package frequency. Migrate old package `expected_rate_hz` declarations to `frequency_hz`.

<a id="registry"></a>
<a id="registry--custom-node-registry-v023"></a>
## Custom node registry (v0.2.3)

[中文](../../../docs_cn/node/interfaces/registry_cn.md#registry) · [Node interfaces](../../isos/interfaces/node/README.md) · [Package release](../../isos/interfaces/package/README.md#release)

IsOS assembles a graph-specific registry from its own UI declarations and the selected EI and EOS owner catalogs. A package contributes declarations through `node_contributions`; its exact `{id,version}` pin selects verified bytes. Registration does not grant execution. IsOS requires owner catalog claims for a cross-core route before graph admission.

The optional `interaction_roles` on a registered type describe its general communication behavior: `publisher`, `subscriber`, `service`, or `action`. A type may declare multiple roles. The roles help graph readers and the Web UI distinguish node cards; they do not replace exact port declarations, owner route claims, or dispatcher admission. A catalog without roles remains valid, and the UI may infer publisher or subscriber from live output or input ports. Service and action require an explicit declaration.

<a id="registry--package-and-core-registry-boundary"></a>
### Package and core registry boundary

The package manager is the sole source of installed package releases, verified bytes, contributed node types, data types, resources, processes, and exact pins. IsOS passes only the selected core projection and verified package references to that core's setup interface. The core returns supported types, routes, adapters, and diagnostics through catalog and validation operations. IsOS combines claims by full type ID and exact package pin, checks each authored edge against both endpoint owners, and publishes the graph only after every selected owner accepts it. Package name, group name, executable filename, and hardware implementation do not select a special registry path. Unknown packages use the same admission sequence and remain unknown until their release is installed or linked and verified.

Graph format `0.2.1` keeps the `isos`, `ei`, and `eos` cores and existing `@v0.2` type identities. A package may choose layer and group names under one of those cores; its type ID must match the declared placement. Each type fixes its placement, settings, ports, carrier, and implementation. An instance selects the type at that placement and supplies values and its exact package pin. The non-mock EOS hardware types are package declarations rather than built-ins. Duplicate type or data IDs, mismatched placement, undeclared libraries, changed bytes, unsupported ABI, and unavailable owner implementations fail closed.

<a id="registry--implementation-forms"></a>
### Implementation forms

`implementation.kind: "native_plugin"` selects Linux `isos_node_v1` or `isos_node_v2` and a safe package-relative `.so` path declared once in `node_contributions.libraries` with its SHA-256. The owning process loads it and owns its memory, threads, lifecycle, and Stop proof. The [v1 C header](../../../contract/node/isos_node_v1.h) and [v2 C header](../../../contract/node/isos_node.h) define the ABIs. All calls return a bounded status; the host copies emitted bytes before returning from the callback. A nonzero Stop result cannot establish actuator quiescence.

`implementation.kind: "external"` names a version `1` owner interface and lifecycle adapter. The owner validates the interface, starts or attaches the declared worker, fences it to the selected run and revision, and proves Stop before reporting completion. The interface is owner-private; a node declaration cannot supply an arbitrary executable command or actuator route.


<a id="registry--data-and-carrier"></a>
### Data and carrier

The [type interface](type.md#type) defines node metadata for readers; a registered [data type](type.md#data_types) carries its JSON Schema, codec, media type, and byte limit. A live port selects exactly one `in_process` or `unix_socket` carrier, delivery mode, data type, and `{owner,id,version}` protocol contract. `none` remains available only for static declarations. An owner advertises exact `port_routes` and `bridge_adapters`; matching port names alone do not authorize delivery. Unix socket envelopes include the run ID, graph revision, source node, output port, data type, and bounded payload. IsOS and both endpoint owners check these fences at the crossing.
