# Shared node type interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: node/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Shared node interfaces](README.md)
  - [Shared node graph interfaces](graph.md)
  - [Shared node registry interfaces](registry.md)

</details>
<!-- doc-navigation:end -->

<a id="type"></a>
<a id="type--registered-node-type"></a>
## Registered node type

[中文](../../../docs_cn/node/interfaces/type_cn.md#type) · [Node interfaces](../../isos/interfaces/node/README.md) · [Instance](graph.md#instance)

A registered type fixes `core`, `layer`, `group`, settings, ports, capabilities, and lifecycle. Its ID has the form `<core>.<layer>.<group>.<type>@v0.2`, where `core` is `isos`, `ei`, or `eos`. An instance supplies identity and declared values; it cannot change placement or behavior. IsOS merges owner catalogs and verifies each selected type at Apply. EI and EOS implement their own interface behavior; graph nodes are wrappers over those interfaces.

| Field | Current meaning |
| --- | --- |
| `id`, `core`, `layer`, `group` | Exact versioned identity and saved placement. |
| `kind`, `description`, `lifecycle` | Display role and owner lifecycle. |
| `interaction_roles` | Optional communication roles: `publisher`, `subscriber`, `service`, and `action`; more than one may apply. |
| `capabilities` | Owner-advertised functions; an empty array is valid. |
| `settings` | Allowed instance values and defaults. |
| `inputs`, `outputs` | Typed connection declarations. |
| `deprecated` | Legacy type remains valid for saved graphs but is hidden from IsosUI's add menus. |
| `implementation`, `service_inputs`, `service_outputs` | Optional owner implementation and service declarations; a declaration alone does not enable a route. |

The starter graph has 17 nodes and 17 declared input connections; the control route passes through an EI context node before the forwarder. The button emits `button_activation@v0.2`; the EOS skill and dispatcher pass `arm_joint_delta@v0.2` to the mock arm. The mock arm is a Manipulator/Adapter instance with six joint names and bounded movement settings. See the [starter graph](../../../crates/isos/templates/generic-graph.toml) for exact IDs and fields.

The fields above define the type declaration for readers; [ports](type.md#ports) and [edges](graph.md#edges) explain how it connects. Contributors maintaining validation can find the machine-readable definition in [`contract/node/`](../../../contract/node/README.md).

`interaction_roles` describes how a registered type participates in communication; it is not a transport, graph edge, or authority grant. A publisher emits typed data and a subscriber consumes it. A service handles a bounded request and reply. An action represents a command with admission, progress or result, and Stop semantics. A processing node may declare both publisher and subscriber. The four roles apply across IsOS, EI, and EOS. The owner still declares exact ports, delivery mode, carrier, and route proof separately. An omitted role list keeps existing type declarations readable; readers derive publisher and subscriber presentation from live output and input ports, but never infer service or action authority from a port name.

The built-in button type illustrates the placement and output declaration. Its output identifies the payload and delivery contract; the emitted event is shown under [data types](type.md#data_types).

```json
{
  "id": "isos.ui.button.trigger@v0.2",
  "core": "isos",
  "layer": "ui",
  "group": "button",
  "kind": "software",
  "description": "Run-fenced, general-purpose one-click activation",
  "lifecycle": "internal",
  "capabilities": ["bridge"],
  "settings": [],
  "inputs": [],
  "outputs": [
    {
      "name": "activation",
      "data_type": "button_activation@v0.2",
      "required": false,
      "transport": "unix_socket",
      "delivery_mode": "event",
      "cardinality": "one",
      "data_class": "event",
      "contract": {
        "owner": "isos",
        "id": "button-activation",
        "version": "1"
      }
    }
  ]
}
```

<a id="ports"></a>
<a id="ports--node-ports"></a>
## Node ports

[中文](../../../docs_cn/node/interfaces/type_cn.md#ports) · [Type](type.md#type) · [Edges](graph.md#edges)

An input port receives and an output port sends. A live port declares a communication channel in `transport`, a delivery mode, an exact registered payload `data_type`, and a protocol `contract` with `owner`, `id`, and `version`. Both ends must agree. When both ports declare `data_class`, their classes must also agree. IsOS admits the edge only when an owner-supported route implements it. A matching payload name alone does not establish delivery.

The starter graph's 17 types have typed ports. The IsOS → EI and EI → EOS boundaries use `unix_socket`; owner-local edges use `in_process`. Activation edges carry `button_activation@v0.2`; the two EOS command edges carry `arm_joint_delta@v0.2`.

| Field | Meaning |
| --- | --- |
| `name` | Unique port name within its direction. |
| `data_type` | Exact registered payload type, or `topology_link` for a static declaration. |
| `data_class` | Optional semantic class: `image`, `observation`, `command`, `event`, `state`, or `opaque` for package ports. If both endpoints declare it, their values must match. It does not substitute for the exact `data_type`. |
| `frequency_hz` | Optional positive finite nominal port rate in hertz; it does not set the runtime sampling cadence. The former `expected_rate_hz` field is rejected in package declarations. |
| `required` | Whether admission requires a connection. |
| `transport` | Required channel: `none` for a static declaration; an explicit supported carrier for a live port. |
| `delivery_mode` | `declaration` for a static declaration; the exact live delivery behavior otherwise. |
| `contract` | Required `{owner,id,version}` protocol identity for a live port; forbidden on a static port. |
| `cardinality` | Connection count, normally `one`. |

`none` and `declaration` must occur together. IsOS checks source and destination declarations during Apply. EI and EOS independently validate their owner projections. Runtime health comes from owner process status, not from a port. This table defines the caller-visible port fields; contributors maintaining validation can use the machine-readable definition under [`contract/node/`](../../../contract/node/README.md).

<a id="data_types"></a>
<a id="data_types--node-data-types"></a>
## Node data types

This contract is for producers, consumers, and IsosUI renderers. A release contribution or core catalog declares a data type once; registered [ports](type.md#ports) reference its exact `id`. IsOS rejects duplicate IDs, missing payload metadata, and mismatched port identities before Apply. An incompatible structure or meaning needs a distinct versioned ID. A producer owns payload bytes and authorization.

| Field | Type / unit | Meaning and validation | Absence/default |
| --- | --- | --- | --- |
| `id` | nonblank string / type ID | Stable identity used by ports and edges. Package-contributed IDs end in `@v<version>`. | Required. |
| `version` | nonblank string / schema version | Version of this payload meaning, independent of package and HTTP versions. | Required. |
| `purpose` | nonblank string / display text | Explains what consumers may use the value for. | Required. |
| `media_type` | nonblank string / MIME type | Describes representation; `json_v1` requires `application/json`. Consumers must also check codec and schema. | Required. |
| `codec` | nonblank string / encoding ID | Identifies exact encoding such as JSON, UTF-8, image, pixel, or audio encoding. | Required. |
| `schema` | valid JSON Schema object / payload structure | Owner-defined shape and units for structured values or planes. `x-unit`, when present, is a nonblank string. | Required. |
| `max_payload_bytes` | integer 1–100,000,000 / bytes | Caps one encoded payload before presentation or delivery. | Required. |

The same fields can describe records, text, time series, images, pixel planes, depth, detections, and audio. A renderer chooses a method only after checking the registered port, codec, media type, schema, bound, and its own capabilities; [UI selection](../../isos/interfaces/api/client.md#selection) describes that choice. Type names alone do not authorize decoding.

IsOS checks descriptor completeness, JSON Schema syntax, unit annotations, and the declared byte bound at package verification or Apply. The owner checks each encoded value against that bound, the schema, and its semantic rules, including source identity, freshness, frame, unit, finite numbers, vector and image dimensions, and resource availability. An exact data ID does not imply conversion, delivery, or permission to actuate. Large media and datasets use bounded references; a new binary codec requires a decoder and byte bound before registration.

<a id="data_types--isolated-library-and-scene-descriptor-identities"></a>
### Isolated library and scene descriptor identities

The isolated node library keeps graph format `0.2.1`, node type `@v0.2`, and the starter's existing data meanings. Its additional JSON v1 descriptors use exact IDs; the owning interfaces define their fields and executable validation. New isolated handler input ports are optional declarations; direct owner calls do not create a graph edge or route. The scene descriptors below have no handler or port.

| Owner | Existing registered IDs | Additional IDs |
| --- | --- | --- |
| EI | `resource_bundle@v0.2`, `observation_set@v0.2`, `context_snapshot@v0.2`, `selected_mission@v0.2`, `primitive_proposal@v0.2`, `health_update@v0.2` | `language_instruction@v0.2`, `vla_input@v0.2`, `grounded_goal@v0.2`, `motion_proposal@v0.2`, `prediction@v0.2`, `memory_reference@v0.2` |
| EOS | `image_frame@v0.2`, `joint_state@v0.2` and the existing hardware and arm types below | `sample_request@v0.2`, `movement_binding_request@v0.2`, `binding_result@v0.2`, `physical_event@v0.2`, `device_status@v0.2`, `stop_request@v0.2`, `stop_latch@v0.2`, `evidence_record@v0.2`, `episode_reference@v0.2`, `gap_record@v0.2`, `manifest_request@v0.2`, `dataset_manifest_reference@v0.2` |
| EOS scene data only | — | `depth_frame_ref@v0.2`, `segmentation_mask_ref@v0.2`, `point_cloud_ref@v0.2`, `physical_detection@v0.2`, `imu_sample@v0.2`, `force_torque_sample@v0.2`, `tactile_sample@v0.2`, `contact_event@v0.2`, `audio_chunk_ref@v0.2` |

These names are distinct meanings, not aliases. EI owns cognition and proposal semantics; an EI motion proposal is never an EOS command. EOS owns measurements, physical status, evidence provenance, and Dispatcher admission. The [EI isolated library](../../ei/interfaces.md#cognition_isolated_library) and [EOS isolated library](../../eos/interfaces.md#isolated_library) specify direct handler validation, unavailable states, and Stop behavior. The [EOS scene data contract](../../eos/interfaces.md#scene_data) specifies the descriptor-only types and their limits.

The starter registers `button_activation@v0.2` and `arm_joint_delta@v0.2` as JSON v1 payloads with a 1024-byte bound. Activation has `event_id` and `kind: "activate"`; it contains no movement target. The EOS delta has exactly six finite `joint_deltas_rad` values in radians. IsOS passes the first type through EI; EOS alone creates the second. See [EOS arm control](../../eos/interfaces.md#arm_control) for bounds and admission.

A button emits this complete activation value on its `activation` output. The event ID is fresh for a distinct click; the value itself has no target or movement arguments.

```json
{
  "event_id": "click-1",
  "kind": "activate"
}
```

The general EOS hardware declarations add `image_frame@v0.2`, `base_state@v0.2`, `joint_state@v0.2`, `base_twist_command@v0.2`, and `joint_position_command@v0.2`. They use JSON v1 descriptors and exact edge type identity. See the [EOS general hardware contract](../../eos/interfaces.md#general_hardware) for their semantics and command admission boundary.

[中文](../../../docs_cn/node/interfaces/type_cn.md#data_types)

<a id="selectors"></a>
<a id="selectors--settings-and-selectors"></a>
## Settings and selectors

This page is for type publishers and UI authors. IsOS currently validates instance settings against the registered type and projects selector choices from declared sources. A selector is a setting declaration, not a package-specific form or a request-time actuator parameter. [Resources](../../isos/interfaces/package/README.md#resources) explain verified package choices.

| Field | Type / unit | Meaning and validation | Absence/default |
| --- | --- | --- | --- |
| `name` | nonblank string / setting ID | Unique within one registered type. | Required. |
| `data_type` | enum / JSON value kind | `string`, `number`, `integer`, `boolean`, `array`, `object`; current aliases also include `string_array`, `string_table`, `positive_integer`, and `nonnegative_integer`. | Required. |
| `required` | boolean / presence rule | A missing required instance value needs a type-declared default. | Required. |
| `description` | nonblank string / display text | Tells the author what to choose and its unit. | Required. |
| `unit` | nonblank string / physical or display unit | States how a numeric or textual value is measured. | Optional. |
| `min`, `max` | finite numbers / setting unit | Bound numeric instance values; `min` cannot exceed `max`. | Optional; no corresponding bound. |
| `default` | value matching `data_type` | Supplies an omitted instance value. | Optional; no default. |
| `choices` | nonempty array of matching values | Bounds valid selections. | Optional; no finite list. |
| `selector.provider` | nonblank string / choice provider | Names `package`, `graph`, `owner`, or local `ui` choice discovery. `owner` asks the selected EOS or EI setup service for contextual choices; `ui` supplies browser-local choices. | Required when `selector` exists. |
| `selector.resource` | nonblank string / resource family | Names the kind of choices requested. | Required when `selector` exists. |
| `selector.package_from` | nonblank string / setting reference | Selects the exact package pin supplying choices. | Optional. |
| `selector.filter` | object / provider filter | Restricts options without interpreting owner payload. | Optional; empty filter. |

IsosUI renders provider-backed choices from the declaration. Graph, package, and owner choices are previews; Apply repeats validation so stale selections cannot be admitted. A missing owner result blocks the selector instead of permitting unverified text. A key or key chord illustrates a fixed node setting: it belongs to the instance profile, while any emitted signal belongs to the output port's separate payload contract. This slice does not enable a browser key selector or a keyboard control route.

[中文](../../../docs_cn/node/interfaces/type_cn.md#selectors)
