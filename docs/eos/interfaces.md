# EOS interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EOS documentation](README.md)
  - [EOS interfaces](models.md)
- Subfolders:
  - [core/](core/README.md)
  - [interfaces/](interfaces/)

</details>
<!-- doc-navigation:end -->

## Table of contents

- [EOS interfaces](#readme--eos-interfaces)
- [EOS demo arm activation](#arm_control--eos-demo-arm-activation)
- [EOS mock arm observation](#arm_monitor--eos-mock-arm-observation)
- [EOS contract sources](#contracts--eos-contract-sources)
- [General EOS hardware declarations](#general_hardware--general-eos-hardware-declarations)
- [Isolated EOS node library](#isolated_library--isolated-eos-node-library)
- [EOS v2 primitive bindings and typed command envelope](#primitive_bindings--eos-v2-primitive-bindings-and-typed-command-envelope)
- [EOS physical scene data](#scene_data--eos-physical-scene-data)
- [Rust skill executable contract](#skills_rust--rust-skill-executable-contract)


<a id="readme"></a>
<a id="readme--eos-interfaces"></a>
## EOS interfaces

[中文](../../docs_cn/eos/interfaces_cn.md#readme_cn) · [EOS](README.md) · [Architecture](core/architecture.md)

EOS node instances may omit `description` or set it to an empty string; the label remains required.

EOS owns the control path to the mock arm and its observation return path in graph format `0.2.1`. The [arm control contract](interfaces.md#arm_control) defines activation, deltas, and limits. The [arm monitor contract](interfaces.md#arm_monitor) defines the typed return path and private read route. IsOS owns the saved graph, package checks, Apply, and deployment status. The [owner lifecycle](../isos/interfaces/server/eos.md#owner_health) defines Run, Status, Stop, and worker failure. The [setup exchange](../isos/interfaces/server/eos.md#setup_v1) defines identity, catalog, validation, and rendering. The [quick start](../isos/start/quick-start.md) follows the operator sequence.

The [general hardware declarations](interfaces.md#general_hardware) define camera, base, and manipulator identities and common payload meanings. Registration and executable adapter support have separate checks.

The [package observer runtime](core/hardware/hardware.md) defines the currently executable external camera adapter, private observation route, ABI probe, and Stop boundary.

The [isolated EOS node library](interfaces.md#isolated_library) defines seven directly executable built-ins, typed payloads, validation and Stop behavior. These nodes have no graph routes.

The [physical scene data contract](interfaces.md#scene_data) defines bounded metadata and references for depth, masks, point clouds, physical detections, IMU, force, tactile, contact, and audio sources. These are data types only; their registration does not supply a decoder or graph route.

EOS communicates with external processes through the [HTTP/SSE adapter](interfaces/server.md#external_http), which calls EOS gRPC services for WebUI requests, and [model providers](models.md#external_models), which EOS calls over HTTP, WebSocket, or configured gRPC. Private model schemas live in `contract/schemas/models/`. These transports do not define a spatial namespace. The skill mailbox belongs to native skill authoring and the skill runtime.

<a id="readme--node-library-reading-map"></a>
### Node library reading map

The current EOS node library follows the registered `<layer>/<group>` path. See [interface](interfaces.md#readme--eos-interface-contracts), [skill](interfaces.md#readme--eos-skill-contracts), [dispatcher](interfaces.md#readme--eos-dispatcher-contracts), and [hardware](interfaces.md#readme--eos-hardware-contracts). These sections link to the owning contracts above.

<a id="readme--contract-source-map"></a>
### Contract source map

The `contract/proto/` schema source defines `island.eos.v1` gRPC services in `gateway.proto`, `skills.proto`, and `workflow.proto`, and the `island.eos.v2` agent capability and operation service in `capabilities.proto`. Its `actions/`, `body/`, `world/`, `system/`, `robot/`, `perception/`, `workflow/`, and `skills/` folders hold domain messages; `options/` holds shared annotations and `external/` holds HTTP route annotations and private model contracts. Project imports use paths relative to `proto/`; each package declares its own wire version. Generated public JSON schemas are under `contract/schemas/`, and HTTP OpenAPI is under `contract/schemas/north-http.openapi.json`. The [southbound hardware boundary](interfaces.md#readme--eos-hardware-contracts) is owned by the Rust runtime and has no protobuf or JSON Schema source: EOS admits selected adapters before Dispatcher releases validated commands.

<a id="readme--eos-hardware-contracts"></a>
<a id="eos-hardware-contracts"></a>

<a id="readme--eos-hardware-layer"></a>
### EOS hardware layer

Hardware node contracts describe typed device-facing graph ports and settings. The built-in mock group is used by the current demo. Other groups represent package-contributed candidates and become available only after exact package verification. Typed hardware payloads stay separate from backend package metadata and node settings.

Use the [mock](interfaces/node/hardware/mock.md#hardware_mock), [camera](interfaces/node/hardware/camera.md#hardware_camera), [base](interfaces/node/hardware/base.md#hardware_base), [manipulator](interfaces/node/hardware/manipulator.md#hardware_manipulator), [gripper](interfaces/node/hardware/gripper.md#hardware_gripper), and [torso](interfaces/node/hardware/torso.md#hardware_torso) group introductions to find node contracts. The [general hardware contract](interfaces.md#general_hardware) covers shared adapter obligations.

<a id="readme--eos-dispatcher-contracts"></a>
<a id="eos-dispatcher-contracts"></a>

<a id="readme--eos-dispatcher-layer"></a>
### EOS dispatcher layer

The dispatcher layer exposes the typed arm command and monitor nodes. A command is accepted only for the current run and a declared route; rejection leaves the actuator unchanged. The arm control contract defines payload and admission rules, while node pages identify the graph-facing ports and settings.

[Arm group](interfaces/node/dispatcher/arm.md#dispatcher_arm) lists both node contracts. See [arm control](interfaces.md#arm_control), [arm monitor](interfaces.md#arm_monitor), and the [dispatcher design](core/README.md#eos-dispatcher-layer) for their respective contract and runtime context.

<a id="readme--eos-interface-contracts"></a>
<a id="eos-interface-contracts"></a>

<a id="readme--eos-interface-layer"></a>
### EOS interface layer

Interface nodes define graph ingress and egress at the EOS boundary. The ingress trigger enters a declared graph route. Arm status and vision flow leave through their declared egress routes; the vision transfer is bound to a live camera source and run identity. These pages identify node ports and boundary obligations without redefining owner-wide transport contracts.

Read the [ingress group](interfaces/node/interface/ingress.md#interface_ingress) and [egress group](interfaces/node/interface/egress.md#interface_egress), then the [interface design](core/README.md#eos-interface-layer) for the execution path.

<a id="readme--eos-skill-contracts"></a>
<a id="eos-skill-contracts"></a>

<a id="readme--eos-skill-layer"></a>
### EOS skill layer

Skill nodes expose typed software behavior in the EOS graph. The demo group has joint-delta and arm-status nodes that connect to the declared control and observation chains. The node contract names graph identity, settings, and ports; the Rust host contract governs how a selected skill implementation runs.

The [demo group](interfaces/node/skill/demo.md#skill_demo) links its node contracts. See the [Rust skill contract](interfaces.md#skills_rust) and [skill design](core/README.md#eos-skill-layer) for host behavior.

<a id="arm_control"></a>
<a id="arm_control--eos-demo-arm-activation"></a>
## EOS demo arm activation

[中文](../../docs_cn/eos/interfaces_cn.md#arm_control_cn) · [EOS interfaces](interfaces.md#readme) · [Hardware design](core/hardware/hardware.md)

<a id="arm_control--overview"></a>
### Overview

EI calls the EOS owner on its private Unix socket with `POST /v1/activate`. The owner verifies the active run and forwards the event to its direct worker child. The EOS interface node passes the activation to the built-in demo skill; the skill emits a six-joint delta to Dispatcher; Dispatcher admits it to the mock arm adapter. No browser or package backend can call the adapter directly.

<a id="arm_control--authentication"></a>
### Authentication

The owner socket is local and mode `0600`. IsOS controls launch, run ID, revision, and the EI-to-EOS connection. The EOS owner requires the exact active run ID and revision. The worker socket is also local and mode `0600`. Stop closes admission before child exit.

<a id="arm_control--interfaces"></a>
### Interfaces

`button_activation@v0.2` is a JSON object with required `event_id` (1–64 ASCII letters, digits, `_`, `-`, or `.`) and `kind` (`activate`). `arm_joint_delta@v0.2` is a JSON object with `joint_deltas_rad`, exactly six finite numbers in radians. The built-in skill emits `[0.05, -0.04, 0.03, -0.02, 0.01, -0.01]`. Each per-click delta is at most 0.25 rad in magnitude. EOS rejects a graph-configured `max_joint_position_rad` above 3.14 rad before launch; Dispatcher rejects a resulting mock joint outside the admitted graph bound. The mock begins with six zeros and retains positions for the lifetime of one worker run. Its [observation return path](interfaces.md#arm_monitor) samples continuously and closes movement admission on stale or invalid samples.

The four control graph nodes are `eos.interface.ingress.trigger@v0.2`, `eos.skill.demo.joint_delta@v0.2`, `eos.dispatcher.arm.delta@v0.2`, and `eos.hardware.mock.arm@v0.2`. The ingress input uses the `ei/activation-to-eos/1` Unix socket bridge; its output and skill input use `eos/activation-local/1` in process. The remaining edges use `eos/arm-delta/1` in process. All ports use event delivery; activation is event data, delta is command data. The mock node is a manipulator adapter with a registered `mock_arm` safety adapter and `joint_positions` capability. The setup catalog advertises the compiled `mock-arm` adapter family and `arm.delta@1.0.0` primitive.

<a id="arm_control--routes"></a>
### Routes

<a id="arm_control--post-v1activate"></a>
### POST /v1/activate

EI sends a JSON body with exact fields `schema_version` (`1`), `core` (`eos`), `op` (`activate`), `run_id`, `revision` (64 lowercase hexadecimal characters), `source_node` (`isos/ui/button/<id>`, with a 1–48 character ASCII ID), `event_id`, and `kind` (`activate`). EOS trusts EI to admit the source against the full graph and checks that it remains a canonical IsOS button path. Unknown or malformed fields are rejected. Each event ID is synchronously reserved in the run journal before `accepted` returns; one executor then performs movement. Repeating an ID in the same run returns the same acceptance receipt and does not move again. The response uses HTTP 200 on success and 422 on rejection. Transport uncertainty is reported as `outcome_unknown` and must not trigger an automatic retry with a new event ID.

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "activate",
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "source_node": "isos/ui/button/trigger",
  "event_id": "click-1",
  "kind": "activate"
}
```

```json
{
  "schema_version": 1,
  "core": "eos",
  "ok": true,
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1",
  "command_id": "arm-click-1",
  "status": "accepted"
}
```

For example, a request fenced to an old run is rejected without a command:

```json
{
  "schema_version": 1,
  "core": "eos",
  "ok": false,
  "error": "stale_or_missing_run"
}
```

Rejected requests have `ok:false` and an `error` code: `invalid_run_identity`, `stale_or_missing_run`, `invalid_activation`, `worker_unavailable`, `outcome_unknown`, `joint_limit`, or `event_capacity`. The resulting joint array is absent on rejection. The worker keeps at most 4096 reserved event IDs per run; when full, it still accepts duplicate known IDs and rejects new IDs without movement. The owner returns `503` for an unavailable worker or uncertain worker outcome, `404` for unknown routes, and `413` for bodies over 1 MiB; the worker limits forwarded messages to 4 KiB. This v1 route and the node/data `@v0.2` identities version independently.

<a id="arm_control--post-v1result"></a>
### POST /v1/result

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "result",
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1"
}
```

A completed result after the initial all-zero joint positions is:

```json
{
  "schema_version": 1,
  "core": "eos",
  "ok": true,
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1",
  "command_id": "arm-click-1",
  "status": "succeeded",
  "joint_positions_rad": [0.05, -0.04, 0.03, -0.02, 0.01, -0.01]
}
```

EI reads a correlated result with exactly `schema_version` (`1`), `core` (`eos`), `op` (`result`), the current `run_id`, `revision`, and `event_id`. A successful reply includes those identities, `command_id`, and `status`: `accepted`, `running`, `succeeded`, `failed`, `cancelled`, or `outcome_unknown`. Only `succeeded` carries `joint_positions_rad`. An unknown ID returns `event_not_found`; Stop or an old run returns `stale_or_missing_run`. EOS monitoring samples independently of result reads, and slow readers get the latest observation.

If the worker exits, the owner reads the run journal for `/v1/result`. A reservation or running record without a terminal record reports `outcome_unknown`. On restart with the same run journal, the worker restores terminal records and positions, marks interrupted commands uncertain, and withholds further movement when any interrupted outcome is uncertain. The executor queue is bounded by the 4096-event run limit; duplicate IDs do not enter it again.

<a id="arm_monitor"></a>
<a id="arm_monitor--eos-mock-arm-observation"></a>
## EOS mock arm observation

[中文](../../docs_cn/eos/interfaces_cn.md#arm_monitor_cn) · [EOS interfaces](interfaces.md#readme) · [Arm control](interfaces.md#arm_control)

The applied EOS projection contains an independent return path from the zero-input observation source `eos.hardware.mock.arm_observer@v0.2`, `eos.dispatcher.arm.monitor@v0.2`, `eos.skill.demo.arm_status@v0.2`, and `eos.interface.egress.arm_status@v0.2`. Each port is named `observation`, uses event delivery, and carries `arm_monitor_state@v0.2` as observation data. The three internal edges use in-process contract `{owner:eos,id:arm-monitor-local,version:1}`. Egress uses the Unix socket contract `{owner:eos,id:arm-monitor-to-ei,version:1}`. EOS requires these nodes and edges before Run.

The observer is a read-only software node in the EOS hardware layer. It has no graph input or actuator binding; it reads the sole mock-arm adapter's periodic sample, which advances independently of activation events. The mock arm remains the only actuator. The earlier input-bound `eos.hardware.mock.arm_output@v0.2` type remains valid for saved graphs using that route; its monitor payload retains the mock arm path in `source_node`.

By default, the mock samples and validates its six joints every 10 ms (100 Hz). Its optional Advanced node settings `sample_interval_ms` (1–1000 ms, default 10) and `freshness_timeout_ms` (2–5000 ms, default 50) must keep freshness at least twice the sampling interval. Its initial sample is six zeros; each admitted movement samples immediately after updating the joints. A sample older than the configured freshness timeout is `stale`; a nonfinite or out-of-bounds joint sample is `degraded`. Both block movement until a fresh valid sample restores `ready`. Stop closes movement admission and reports `stopped`. Only a `ready` observation includes positions.

<a id="arm_monitor--payload"></a>
### Payload

`arm_monitor_state@v0.2` is a JSON object with exactly these fields:

| Field | Type and rule |
| --- | --- |
| `run_id`, `revision` | Active run ID and 64-character lowercase revision. |
| `source_node` | Graph-bound observation source path, such as `eos/hardware/mock/arm_observer`. |
| `sequence` | Positive sample sequence within this worker run. |
| `sampled_at_monotonic_ns` | Nonnegative monotonic sample time in nanoseconds. |
| `joint_names` | Six unique names in graph order. |
| `joint_positions_rad` | Six finite radians when `ready`; `null` otherwise. |
| `state` | `ready`, `degraded`, `stale`, or `stopped`. |
| `reason_code` | `null` when ready; otherwise a reason such as `invalid_joint_sample`, `sample_timeout`, or `stopped`. |

An initial ready sample is:

```json
{
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "source_node": "eos/hardware/mock/arm_observer",
  "sequence": 1,
  "sampled_at_monotonic_ns": 1000000000,
  "joint_names": ["joint_1", "joint_2", "joint_3", "joint_4", "joint_5", "joint_6"],
  "joint_positions_rad": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
  "state": "ready",
  "reason_code": null
}
```

<a id="arm_monitor--private-owner-read"></a>
### Private owner read

EI calls `POST /v1/monitor` on the mode `0600` EOS owner Unix socket. The exact request is `{ "schema_version":1, "core":"eos", "op":"monitor", "run_id":"run_1", "revision":"<64 lowercase hex>" }`. EOS requires the active run and revision, reads the latest worker sample without moving the arm, and returns `{ "schema_version":1, "core":"eos", "ok":true, "run_id":"run_1", "revision":"<revision>", "observation":{...} }`. EI may poll at up to 20 Hz; EOS safety sampling stays at 100 Hz independently. An unavailable worker returns `worker_unavailable` with HTTP 503. A stale run returns `stale_or_missing_run` with HTTP 422. Stop invalidates the read fence.

<a id="arm_monitor--private-mock-worker-output"></a>
### Private mock worker output

The EOS owner may read the declared observer output over the worker's mode `0600` Unix socket with `{"schema_version":1,"core":"eos","op":"observe_output","run_id":"run_1","revision":"<64 lowercase hex>","node":"eos/hardware/mock/arm_observer","port":"observation"}`. All seven fields are required and extra fields fail admission. The `node` must equal this worker's graph-bound observer path and `port` must be `observation`; this operation has no actuator ingress. A successful reply is `{"schema_version":1,"core":"eos","ok":true,"run_id":"run_1","revision":"<revision>","node":"eos/hardware/mock/arm_observer","port":"observation","payload":{...}}`, where `payload` is the exact `arm_monitor_state@v0.2` observer sample described above. The owner must check both envelope and payload identities before forwarding a graph link. A stale run, revision, or malformed envelope returns `observation_fence_stale`; an undeclared node or port returns `observation_route_unsupported`; Stop returns `worker_stopping`; an unavailable sample returns `observation_unavailable`. Each error reply has `schema_version:1`, `core:"eos"`, `ok:false`, and `error`, without a payload. The operation reads the latest sample without changing the arm or command journal; the existing `monitor` exchange remains available to EI.

<a id="contracts"></a>
<a id="contracts--eos-contract-sources"></a>
## EOS contract sources

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](interfaces.md#arm_control) for the active data and command path and [owner lifecycle](../isos/interfaces/server/eos.md#owner_health) for setup, Run, Status, and Stop.


[中文](../../docs_cn/eos/interfaces_cn.md#contracts_cn) · [Interface index](interfaces.md#readme) · [gRPC and IPC](../isos/interfaces/server/eos.md#grpc)

IsOS maintains the authoritative shared source for EOS protobuf and JSON Schemas in `contract/proto/`. EOS compiles and validates against these definitions; its runtime still owns hardware behavior and Dispatcher admission. `gateway.proto`, `skills.proto`, and `capabilities.proto` define the process-facing service messages used by the current Rust runtime. Domain folders such as `robot/`, `perception/`, `skills/`, `actions/`, `body/`, and `world/` define payloads and validation annotations. The [Rust skill executable contract](interfaces.md#skills_rust) defines the separate worker protocol.

The generator derives JSON Schemas and a manifest under `contract/schemas/`. A protobuf envelope supplies transport identity; the selected generated schema validates bounded arguments and results. Graph node type IDs, process wire schemas, source package versions, and external package versions are separate identities.

<a id="contracts--updating-a-contract"></a>
### Updating a contract

Edit the owning `.proto` or authored schema, regenerate derived artifacts, and update its interface page and Chinese mirror. Run:

```sh
python3 scripts/generate_eos_contracts.py --output contract/schemas
python3 scripts/generate_eos_contracts.py --output contract/schemas --check
cargo test --workspace
python3 scripts/check_doc_links.py --all
```

Review generated diffs and add a request, success, rejection, and failure fixture for a public contract change. Keep hardware samples and commands as typed Rust values below the transport boundary.

<a id="general_hardware"></a>
<a id="general_hardware--general-eos-hardware-declarations"></a>
## General EOS hardware declarations

[中文](../../docs_cn/eos/interfaces_cn.md#general_hardware_cn) · [EOS interfaces](interfaces.md#readme) · [Node data types](../node/interfaces/type.md#data_types)

The seven types below are declared by the MuJoCo 0.2.2 candidate, not built-in EOS registrations. A verified package may declare its own hardware types and choose layer and group names within the `eos` core. EOS retains the shared payload schemas and safety checks. The Linux v2 package service path executes selected camera, base, manipulator, gripper, and torso nodes after IsOS verifies the exact package and injects its run-bound service endpoint; commands require EOS Dispatcher admission and durable identity. The mock-arm adapter retains its built-in type and payloads.

| MuJoCo package type | Placement | Data ports |
| --- | --- | --- |
| `eos.hardware.camera.device@v0.2` | `eos/hardware/camera/<id>` | `image_frame@v0.2` output |
| `eos.hardware.base.device@v0.2` | `eos/hardware/base/<id>` | `base_state@v0.2` output; optional `base_twist_command@v0.2` input |
| `eos.hardware.manipulator.device@v0.2` | `eos/hardware/manipulator/<id>` | `joint_state@v0.2` output; optional `joint_position_command@v0.2` input |
| `eos.hardware.base.normalized_axes@v0.2` | `eos/hardware/base/<id>` | `pose_state@v0.2` output; optional `normalized_axes_command@v0.2` input |
| `eos.hardware.manipulator.normalized_cartesian@v0.2` | `eos/hardware/manipulator/<id>` | `pose_state@v0.2` output; optional normalized axes `command` and `control_mode_command@v0.2` `mode` inputs |
| `eos.hardware.gripper.binary@v0.2` | `eos/hardware/gripper/<id>` | `gripper_state@v0.2` output; optional `gripper_binary_command@v0.2` input |
| `eos.hardware.torso.normalized_axes@v0.2` | `eos/hardware/torso/<id>` | optional `normalized_axes_command@v0.2` input; no fabricated position output |

An instance names its device identity and `node.adapter.type`. A package-backed family also requires an exact `node.adapter.package` pin. `node.adapter.settings` contains family-specific endpoint fields validated by EOS; a package service records its declared service instance there. IsOS verifies the pin and process resource declarations. EOS must check the compiled family, device kind, endpoint, process readiness, and stop capability before reporting ready. Package metadata and node settings do not become hardware payload fields. A base or manipulator may publish observations without a graph producer wired to its command input. For a selected device, EOS binds each declared actuator port to its Dispatcher even when no producer edge is wired; the owner command API can target that port. A selected, verified v2 package service executes the admitted command; undeclared ports and unsupported families remain unavailable. No adapter or backend may create another command route. Stop must close command admission and quiesce admitted actuators before any selected service child exits.

<a id="general_hardware--common-data"></a>
### Common data

These ten data IDs are distinct from the starter's `camera_frame`, `arm_joint_delta@v0.2`, and `arm_monitor_state@v0.2` meanings. This first version uses `version:"0.2"`, `media_type:"application/json"`, and `codec:"json_v1"` for all ten descriptors. Observation `sample` requires `source_time_ns` (unsigned nanoseconds in the declared clock), `clock` (`system`, `simulation`, or `monotonic`), `sequence` (unsigned integer), and nonblank `frame_id` (declared coordinate frame). These fields have no defaults. Structured values use finite numbers and SI units. Command identity, sequence, deadline, run fence, and admission belong to the Dispatcher envelope rather than these payloads.

| ID | Payload meaning |
| --- | --- |
| `image_frame@v0.2` | `sample`, `width_px`, `height_px`, `encoding` (`jpeg` or `rgb8`), and bounded base64 `data` in JSON. EOS validates dimensions and encoded byte length. |
| `base_state@v0.2` | `sample`, `x_m`, `y_m`, `yaw_rad`, `vx_m_s`, `vy_m_s`, and `yaw_rate_rad_s`. |
| `joint_state@v0.2` | `sample`, ordered `joint_names`, `position_rad`, and `velocity_rad_s`; EOS checks that vector lengths match. |
| `base_twist_command@v0.2` | `frame_id`, `vx_m_s`, `vy_m_s`, `yaw_rate_rad_s`, and `lease_ms`. EOS checks frame and lease bounds. |
| `joint_position_command@v0.2` | Ordered `joint_names` and `position_rad`; EOS checks names, length, and position bounds. |
| `pose_state@v0.2` | `sample`, `position_m` (three SI metres), and `orientation_xyzw` (four quaternion values). The adapter must check unit norm and declared frame. |
| `gripper_state@v0.2` | `sample` and named `channels`, each with a finite `value` and explicit `unit`. |
| `normalized_axes_command@v0.2` | Nonempty declared `axes` map with values in `[-1,1]` and a 1–1000 ms `lease_ms`. |
| `gripper_binary_command@v0.2` | `action` equal to `open` or `close`. |
| `control_mode_command@v0.2` | Nonempty `mode`; the adapter admits only advertised modes. |

These types declare graph and payload contracts. The Linux `isos_node_v2` host observes, executes commands only after the EOS owner reserves them through Dispatcher, and demands a run-bound actuator Stop proof. The IsOS package service handoff supplies a private endpoint for the selected service instance. A declaration alone never makes an actuator ready.

For a selected service, IsOS binds the exact package pin, `service_instance`, private endpoint, generation, and configuration digest to the run. EOS validates the selected command port and payload, reserves a durable command ID through Dispatcher, and forwards one command with its private run token. Duplicate IDs return the stored result only when node, port, and payload match; ambiguous delivery inhibits new commands. On Stop, EOS closes admission and requires the adapter's run-bound `outcome:"stopped"` proof before reporting confirmation. See the [node contribution contract](../isos/interfaces/server/eos.md#node_contributions) and [package check contract](../isos/interfaces/package/README.md).

All listed payload fields are required; an omitted field has no default. `width_px` and `height_px` are positive pixel counts; image `data` is base64 text and the complete JSON payload must fit the descriptor's `max_payload_bytes`. `rgb8` has three bytes per pixel, while `jpeg` bytes must decode to the declared dimensions. Position and velocity values use metres, radians, seconds, and their indicated rates. `lease_ms` is a positive millisecond duration. Numeric ranges, frame allowlists, joint count and names, command rates, and movement limits come from the selected registered type and device instance, not from a universal hardware constant. EOS runtime adapters are required to reject missing or unknown clocks, nonfinite values, mismatched vectors, invalid image size, undeclared frames, out-of-range leases, and payloads beyond the descriptor limit. These checks apply at the executable adapter boundary; unsupported families and invalid payloads fail closed.

A representative base observation and a separate command payload are:

```json
{
  "sample": {
    "source_time_ns": 1000000000,
    "clock": "simulation",
    "sequence": 7,
    "frame_id": "map"
  },
  "x_m": 1.25,
  "y_m": 0.5,
  "yaw_rad": 0.1,
  "vx_m_s": 0.2,
  "vy_m_s": 0.0,
  "yaw_rate_rad_s": 0.05
}
```

```json
{
  "frame_id": "base_link",
  "vx_m_s": 0.2,
  "vy_m_s": 0.0,
  "yaw_rate_rad_s": 0.05,
  "lease_ms": 100
}
```

Ports and edges require the same exact data ID; IsOS performs no implicit frame, unit, or codec conversion. An explicit registered transformer is required for a different representation. Invalid declarations or an unavailable adapter fail setup/Run admission with an owner diagnostic and never become ready. A runtime payload failure must mark the device unhealthy and reject the sample or command; an uncertain actuator stop cannot be reported as successful. These payload meanings draw from ROS 2 [JointState](https://github.com/ros2/common_interfaces/blob/rolling/sensor_msgs/msg/JointState.msg), [CompressedImage](https://github.com/ros2/common_interfaces/blob/rolling/sensor_msgs/msg/CompressedImage.msg), and [REP 103](https://www.ros.org/reps/rep-0103.html) SI units and frames; they do not claim ROS message or transport compatibility. The owning EOS adapter must validate payload semantics, and the Dispatcher must validate target, limits, deadline, health, and run fence before movement.

<a id="isolated_library"></a>
<a id="isolated_library--isolated-eos-node-library"></a>
## Isolated EOS node library

[中文](../../docs_cn/eos/interfaces_cn.md#isolated_library_cn) · [EOS interfaces](interfaces.md#readme) · [Node data types](../node/interfaces/type.md#data_types)

The EOS catalog includes seven directly executable built-in types for isolated library tests. They appear in `catalog.direct_only_types`; the same IDs are absent from `identity.supported_node_types`, and normal graph setup rejects them with `isolated_node_graph_runtime_unavailable`. Each input is optional in the registered type and has declaration-only transport with no catalog route. Direct calls use the Rust library API exercised by fixture tests. The runtime owner does not expose a call route. These types do not change the 17-node starter graph. The direct caller supplies a trusted `run_id` to each handler. The four `IsolatedNode` handlers and the injected Stop handler also receive current monotonic time in nanoseconds; neither value is a graph setting. The Rust handlers validate values against their exact registered JSON v1 data types. An invalid call returns a bounded error and emits no output. A stopped instance rejects input. `read` returns the last typed output without invoking hardware where that method exists. The four `IsolatedNode` handlers expose `health_at`, which reports `degraded` with `sample_stale` when their last source time is more than one second old; the Stop, recorder, and manifest handlers expose `health` without this freshness check. `stop` confirms quiescence for local, non-actuating handlers; the emergency latch stays with the injected EOS Stop service.

| Type ID | Setting | Input → output | Behavior |
| --- | --- | --- | --- |
| `eos.hardware.mock.camera@v0.2` | required `frame_id` | `sample: sample_request@v0.2` → `frame: image_frame@v0.2` | Emits one fixed 2×2 RGB8 fixture with `clock:monotonic`. The node identity marks it mock; it does not claim a physical camera. |
| `eos.perception.physical.features@v0.2` | required `frame_id` | `state: joint_state@v0.2` → `event: physical_event@v0.2` | Reports `detected` at any absolute joint position ≥3.0 rad; otherwise `clear`. |
| `eos.dispatcher.command.binding@v0.2` | required `frame_id`, `target_node` | `request: movement_binding_request@v0.2` → `binding: binding_result@v0.2` | Requires exact `arm.delta@1.0.0`, `joint_positions`, `rad`, six deltas in ±0.25 rad, and `eos/hardware/mock/arm`. It never dispatches movement. |
| `eos.safety.device.guard@v0.2` | required `frame_id` | `state: joint_state@v0.2` → `status: device_status@v0.2` | Reports `inhibited` above 3.14 rad and `ready` otherwise. Dispatcher still checks physical limits at write time. |
| `eos.safety.device.estop@v0.2` | no graph settings | `stop: stop_request@v0.2` → `latch: stop_latch@v0.2` | Delegates once to an injected EOS Stop service; unavailable without that service. Local Stop does not clear the latch. |
| `eos.evidence.episode.recorder@v0.2` | no graph settings | `record: evidence_record@v0.2` → `gap: gap_record@v0.2`; Stop → `episode: episode_reference@v0.2` | Writes bounded metadata to an owner-supplied append-only local store. Byte-budget overflow produces a gap. |
| `eos.dataset.export.manifest@v0.2` | no graph settings | `request: manifest_request@v0.2` → `manifest: dataset_manifest_reference@v0.2` | Resolves only owner-authorized episode IDs and writes a deterministic manifest with source digests and policy IDs. |

All settings reject unknown keys. `frame_id` is 1–128 ASCII letters, digits, underscore, hyphen, or dot. The binding target is exactly `eos/hardware/mock/arm`. The four `IsolatedNode` handlers reject a different frame, future source time, or age above one second; the Stop handler checks source-time freshness but no frame. Recorder and manifest inputs check run and payload semantics without caller-time freshness. Sensor nodes also reject non-increasing source time; joint inputs require `sample.clock:monotonic`. The joint handler rejects different joint-name, position and velocity vector lengths. These handlers interpret caller-supplied samples; they do not attest that a sample came from measured hardware.

<a id="isolated_library--data-payloads"></a>
### Data payloads

Each new descriptor uses `version:0.2`, `media_type:application/json`, `codec:json_v1`, and a 16,384-byte payload bound. Unknown fields reject. `sample_request@v0.2` has required `source_time_ns` (nonnegative integer monotonic nanoseconds), `sequence` (positive integer), `frame_id` (nonblank string), and `clock:monotonic`. `movement_binding_request@v0.2` has required `source_time_ns`, `frame_id`, `target_node`, `primitive`, `mode`, `unit`, and `joint_deltas_rad` (exactly six finite radian values in ±0.25). The primitive, mode, and unit are exact constants listed above. The four `IsolatedNode` handlers compare source times to caller-supplied current monotonic time; these are not wall-clock timestamps.

The three new outputs require `run_id`. `physical_event@v0.2` also carries `source_time_ns`, `frame_id`, `source_sequence`, `event:joint_limit_proximity`, `state` (`detected`, `clear`, or `unknown`), `provenance:input_joint_state`, and `rule_version:1`. `device_status@v0.2` carries the same source fields plus `state` (`ready`, `degraded`, or `inhibited`) and nullable `reason`. `binding_result@v0.2` carries `target_node`, `primitive`, `mode`, `unit`, and `valid:true`; this proves local binding validation only. These fields have no default.

```json
{
  "source_time_ns": 100,
  "frame_id": "arm",
  "target_node": "eos/hardware/mock/arm",
  "primitive": "arm.delta@1.0.0",
  "mode": "joint_positions",
  "unit": "rad",
  "joint_deltas_rad": [0.01, 0.0, 0.0, 0.0, 0.0, 0.0]
}
```

<a id="isolated_library--stop-and-evidence-ownership"></a>
### Stop and evidence ownership

`stop_request@v0.2` requires `run_id`, `reason` and `source_time_ns`; `stop_latch@v0.2` returns `run_id`, `latched:true` and a bounded `proof_ref` supplied by the Stop service. The handler cannot reset the latch. A service error gives no proof. The production owner must supply its existing run-bound Stop service before this type can start; a catalog declaration alone does not make it ready.

`evidence_record@v0.2` requires `run_id`, `record_id`, `source_time_ns`, `kind` (`sample`, `command`, `event`, `intervention`) and `source_ref`. It carries metadata only. Duplicate IDs and a foreign run reject. `episode_reference@v0.2` returns the run, episode ID, record count and SHA-256 of the append-only JSONL bytes after a confirmed flush. `gap_record@v0.2` reports run, episode, positive lost count and `byte_budget` or `write_failure`; a write failure degrades health and returns an error with a visible gap count. The owner injects the storage root and byte budget, not a graph setting.

`manifest_request@v0.2` requires `run_id`, one or more episode references containing `episode_id`, `record_count` and `sha256`, plus `split_policy_id` and `label_policy_id`. The exporter checks each ID against its owner-supplied authorization set, reads bounded episode bytes, checks each record's run and schema, rejects changed bytes or count, and records a source SHA-256 and deterministic `train` or `test` split. `dataset_manifest_reference@v0.2` returns run, manifest ID, episode count and SHA-256. Repeating identical input produces the same bytes and reference. The manifest stores policy IDs as label provenance; it does not create labels or upload data. These three handlers have a direct test fixture, but production owner wiring to the Stop service and governed storage is still required before a selected graph can run them. The existing arm Dispatcher and owner Stop remain the actuator and Stop authority. Verify this direct boundary with `cargo test -p aether-node-profile --test isolated_library`; it covers catalog presence, input validation, health, rejection, provenance, gaps, deterministic export and Stop.

<a id="isolated_library--verification"></a>
### Verification

On 2026-11-15, `cargo fmt --all`, `cargo fmt --all -- --check`, `cargo test -p aether-node-profile`, `cargo clippy -p aether-node-profile --all-targets -- -D warnings`, `cargo build -p aether-runtime --lib`, `cargo test -p aether-runtime -- --test-threads=1`, `cargo clippy -p aether-runtime --bin aether-profile -- -D warnings`, `python3 scripts/check_doc_links.py --all`, and `git diff --check` passed. Complete outputs are under ignored `data/test/20261115-eos-isolated-library/`. The runtime library build precedes the full runtime test because its demo skill tests load the sibling shared library. Serial runtime tests avoid unrelated journal owner contention between concurrent test cases. The caller audit kept the existing arm, native package, Dispatcher and Stop paths; this slice removes no code claimed to be dead.

<a id="primitive_bindings"></a>
<a id="primitive_bindings--eos-v2-primitive-bindings-and-typed-command-envelope"></a>
## EOS v2 primitive bindings and typed command envelope

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](interfaces.md#arm_control) for the active data and command path and [owner lifecycle](../isos/interfaces/server/eos.md#owner_health) for setup, Run, Status, and Stop.


[中文](../../docs_cn/eos/interfaces_cn.md#primitive_bindings_cn) · [Hardware profile](interfaces/node/hardware/mock.md#hardware_mock--arm) · [Graph controls](../isos/interfaces/server/eos.md#control)

An EOS primitive binding names one versioned movement operation and connects it to exactly one hardware device. The IsOS graph owns the binding edge; EOS reads it, validates a typed command header, and sends the decoded Rust movement through the shared Dispatcher. The binding does not own a driver or authorize direct actuator release.

<a id="primitive_bindings--graph-contract"></a>
### Graph contract

The built-in types `isos.eos_dispatcher.primitive_base@v0.2` and `isos.eos_dispatcher.primitive_manipulator@v0.2` live at `isos/eos_dispatcher/base/<id>` and `isos/eos_dispatcher/manipulator/<id>`. Each has one `command` output of `base_movement` or `manipulator_movement`. A generic `isos.eos_hardware.base@v0.2` or `isos.eos_hardware.manipulator@v0.2` device has `inputs.command` with cardinality `many`: an explicit array of one or more `{node,output}` sources. Every binding must appear on exactly one compatible device input. The binding has no target setting; the edge is its target. The shared Dispatcher service is implicit for this route, so a graph need not contain `isos.eos_dispatcher.control@v0.2`.

```json
{
  "id": "velocity",
  "node_type": "isos.eos_dispatcher.primitive_base@v0.2",
  "label": "Base velocity",
  "description": "Bounded velocity binding",
  "settings": {
    "primitive_name": "base.velocity",
    "primitive_version": "1.0.0",
    "movement_type": "body_velocity"
  }
}
```

```json
{
  "command": [
    {
      "node": "isos/eos_dispatcher/base/velocity",
      "output": "command"
    }
  ]
}
```

| Field | Type | Unit | Purpose and validation |
| --- | --- | --- | --- |
| `settings.primitive_name` | string | primitive ID | Required; currently `base.velocity` for base or `arm.movej` for manipulator. |
| `settings.primitive_version` | string | semantic version | Required; currently `1.0.0` for both admitted primitives. |
| `settings.movement_type` | string | mode ID | Required; `body_velocity` for `base.velocity`, `joint_positions` for `arm.movej`; the device must advertise the mode. |
| `inputs.command` | array of `PortSource` | typed graph edges | Required for an actuator; each source is distinct, type compatible, and a binding output. One binding targets exactly one device. |
| `PortSource.node` | string | canonical path | Source instance `<core>/<layer>/<group>/<node>`; must exist. |
| `PortSource.output` | string | output port | Must name the source's registered `command` output. |
| `node.adapter.package` | `{id:string,version:string}` or absent | package reference | Required for a package family; exact verified package pin, independent of the built-in node type's `node.package`. |


<a id="primitive_bindings--bounded-runtime-command"></a>
### Bounded runtime command

The EOS owner-only Unix control socket currently accepts one JSON line, up to 64 KiB. `primitive_typed` is the command operation for new bindings. Its `payload` is a typed movement object from [hardware v2](interfaces/node/hardware/mock.md#hardware_mock--arm); the JSON bytes counted by the **private** `payload_length_bytes` are the compact `serde_json` UTF-8 serialization of that parsed object. The loopback HTTP bridge accepts a bounded public compact UTF-8 length and normalizes the private header after parsing because JavaScript and Rust can serialize the same finite number with different byte lengths; see [the bridge contract](../isos/interfaces/server/eos.md#control--loopback-api). The decoder rejects unknown fields, lengths above 16 KiB, unsupported codec or version, a forged graph target, and a primitive/payload mismatch before asking Dispatcher to admit the command. Dispatcher still checks health, deadlines, limits, journaling, and stop.

For a browser invocation with the merged EOS R03 implementation, the existing loopback `/api/eos/v2/control` bridge allowlists `primitive_typed` and forwards it to this private runtime operation after Origin and size checks. A caller can send `deadline_after_ms`, which EOS resolves against its own monotonic clock. Existing absolute-deadline callers obtain EOS monotonic time from the bridge. EOS validates the typed header, exact active binding and target again. The response's `command_id`, exact `Completed`/`Cancelled`/`Rejected`/`InProgress`/`OutcomeUnknown` status and reason drive the panel's result display; browser-local sending is only a pending indicator. The browser bridge rewrites a bounded public `result` to guarded private `primitive_result`: the [R07 authorized result contract](../isos/interfaces/server/eos.md#control--r07-authorized-browser-result) defines the bounded read needed before the UI follows `InProgress`. An uncertain outcome is never shown as completed. This is the UI ingress to the existing contract, not a new graph wire or gRPC service name.

```json
{
  "op": "primitive_typed",
  "header": {
    "data_type_id": "base_movement",
    "data_type_version": "1.0",
    "primitive_name": "base.velocity",
    "primitive_version": "1.0.0",
    "source_path": "isos/eos_dispatcher/base/velocity",
    "target_path": "isos/eos_hardware/base/drive",
    "command_id": "move-1",
    "sequence": 1,
    "deadline_monotonic_ns": 5000000000,
    "payload_length_bytes": 83,
    "codec": "json_v1"
  },
  "payload": {
    "type": "body_velocity",
    "x_m_s": 0.1,
    "y_m_s": 0.0,
    "yaw_rad_s": 0.0,
    "valid_for_ms": 100
  }
}
```


| Header field | Type | Unit | Purpose and validation |
| --- | --- | --- | --- |
| `data_type_id` | string | registered ID | `base_movement` or `manipulator_movement`, matching decoded payload and binding port. |
| `data_type_version` | string | IDL version | Required `1.0`. |
| `primitive_name` | string | primitive ID | Must equal the binding's `primitive_name`. |
| `primitive_version` | string | semantic version | Must equal the binding's `primitive_version`. |
| `source_path` | string | canonical graph path | Must name the exact binding node in short form. |
| `target_path` | string | canonical graph path | Must equal the one hardware device selected by the graph edge. |
| `command_id` | string | command identity | Nonempty, at most 128 UTF-8 bytes; Dispatcher also enforces its ID grammar and duplicate policy. |
| `sequence` | unsigned integer | command sequence | Required positive value; caller supplies it for ordering and tracing. |
| `deadline_monotonic_ns` | unsigned integer | ns | EOS monotonic deadline; Dispatcher rejects expired work. |
| `deadline_after_ms` | unsigned integer | ms | Alternative to the absolute deadline for the HTTP bridge; EOS resolves 1–5,000 ms against its own monotonic clock. Both deadline fields cannot appear together. |
| `delivery_mode` | string | — | `one_shot` by default; `stream` publishes a bounded movement lease through a declared gateway edge. |
| `run_id` | string | — | Required for a stream; must equal the active managed run, including after a restart with an unchanged revision. |
| `gateway_path` | string | graph path | Required for a stream; identifies the declared EOS HTTP gateway connected to the primitive binding input. |
| `payload_length_bytes` | unsigned integer | bytes | Private IPC: exact compact `serde_json` payload length, 1 through 16,384. Public HTTP: bounded hint, normalized by the bridge before private IPC. |
| `codec` | string | codec ID | Required `json_v1`; EOS decodes into `Movement` before CXX methods. |

<a id="primitive_bindings--stream-movement"></a>
#### Stream movement

The IsOS HTTP bridge sends `delivery_mode:"stream"`, the current `run_id`, declared `gateway_path`, and `deadline_after_ms` for each `base_movement` command. EOS allows streaming only for `body_velocity` and checks the active revision, run ID, and graph edge before publication. A later `primitive_result` may return `Completed` with reason `stream lease published`: this proves adapter publication, not physical standstill. Each command replaces the preceding bounded lease; on key release, a zero-velocity command promptly replaces continuing motion. The gateway and primitive binding on this path both declare `max_rate_hz:20`.

<a id="scene_data"></a>
<a id="scene_data--eos-physical-scene-data"></a>
## EOS physical scene data

[中文](../../docs_cn/eos/interfaces_cn.md#scene_data_cn) · [EOS interfaces](interfaces.md#readme) · [General hardware](interfaces.md#general_hardware)

EOS registers nine JSON v1 metadata and reference types for isolated physical sources. They declare payload meaning only: no new node, graph route, decoder, physical driver, or artifact read endpoint is registered here. A producer must prove its real device or selected package separately. A reference carries an owner-controlled artifact ID and SHA-256, never a host path, credential, socket, or inline media. The source remains authoritative for capture time, coordinate frame, calibration and validity. EI may derive task-level meaning without changing these EOS source records.

All nine descriptors use `version: "0.2"`, `media_type: "application/json"`, `codec: "json_v1"`, and `max_payload_bytes: 4096`. JSON objects reject unknown fields, missing required fields and nonfinite numbers. Every payload requires `sample`, `calibration_id`, and `validity`; no field has a default. `sample` requires nonnegative `source_time_ns` in the monotonic clock, `clock: "monotonic"`, nonnegative `sequence`, nonblank `frame_id` and `source_id`. `calibration_id`, `source_id` and artifact IDs are 1–128 ASCII letters, digits, underscore, hyphen or dot. `validity` is `valid`, `invalid`, or `unknown`; an unknown sample never silently becomes measured truth. The caller supplies its current monotonic time, expected frame and calibration to `validate_scene_payload`. A future sample, a sample older than one second, or a mismatched frame/calibration rejects before use.

| ID | Additional required fields and exact meaning |
| --- | --- |
| `depth_frame_ref@v0.2` | `artifact_id`, lowercase 64-hex `sha256`, `byte_length` 1–100,000,000 bytes, `width_px`/`height_px` 1–8192 pixels, `encoding: "u16le"`, `metres_per_unit` >0–1 m/unit, `uncertainty_m` 0–100 m. It references pixels; the descriptor does not decode them. |
| `segmentation_mask_ref@v0.2` | Same artifact and dimension fields, `source_image_id`, `mask_kind: "binary"`, `encoding: "u8"`. It identifies a physical binary mask without semantic class labels. |
| `point_cloud_ref@v0.2` | Same artifact fields, `point_count` 1–10,000,000, `unit: "m"`, `encoding: "xyz_f32_le"`, `uncertainty_m` 0–100 m. No implicit millimetre conversion. |
| `physical_detection@v0.2` | `detection_id`, `kind` (`obstacle`, `person_proximity`, `unknown`), three-element `position_m` in metres, `confidence` 0–1, `uncertainty_m` 0–100 m. It reports physical evidence, not object or task identity. |
| `imu_sample@v0.2` | Three-element `acceleration_m_s2` in m/s², three-element `angular_velocity_rad_s` in rad/s, `uncertainty_m_s2` 0–100 m/s². |
| `force_torque_sample@v0.2` | Three-element `force_n` in newtons, three-element `torque_nm` in N·m, `uncertainty_n` 0–100,000 N. |
| `tactile_sample@v0.2` | `sensor_id`, 1–32 `pressure_pa` values in 0–10,000,000 Pa, `uncertainty_pa` in the same range. |
| `contact_event@v0.2` | `event_id`, `body_a`, `body_b`, `state` (`began`, `ended`, `unknown`), nullable `force_n` in 0–100,000 N. Null means force was not measured. |
| `audio_chunk_ref@v0.2` | Same artifact fields, `encoding: "pcm_s16le"`, `sample_rate_hz` 8,000–192,000 Hz, `channels` 1–8, `duration_ms` 1–1,000 ms. It contains no transcript or speech meaning. |

For example, one depth reference and one physical detection are:

```json
{
  "sample": {
    "source_time_ns": 100,
    "clock": "monotonic",
    "sequence": 7,
    "frame_id": "sensor_frame",
    "source_id": "sensor_1"
  },
  "calibration_id": "cal_1",
  "validity": "valid",
  "artifact_id": "depth_1",
  "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
  "byte_length": 8,
  "width_px": 2,
  "height_px": 2,
  "encoding": "u16le",
  "metres_per_unit": 0.001,
  "uncertainty_m": 0.01
}
```

```json
{
  "sample": {
    "source_time_ns": 100,
    "clock": "monotonic",
    "sequence": 7,
    "frame_id": "sensor_frame",
    "source_id": "sensor_1"
  },
  "calibration_id": "cal_1",
  "validity": "valid",
  "detection_id": "det_1",
  "kind": "obstacle",
  "position_m": [1.0, 0.0, 0.0],
  "confidence": 0.9,
  "uncertainty_m": 0.1
}
```

The [fixture test](../../crates/aether-node-profile/tests/scene_data.rs) provides a valid example for each of the nine IDs and rejects missing source identity, wrong frame/calibration, stale source time, over-limit JSON size, unsafe artifact ID, and an invalid unit, shape, range or encoding for each family. Run `cargo test -p aether-node-profile --test scene_data`. These descriptors alone do not prove referenced bytes match the digest; a governed artifact reader must verify bytes and authorization before opening an artifact. Binary codecs and raw media delivery require a separate owner interface and decoder.

For `u16le` depth, `u8` mask and `xyz_f32_le` point cloud references, `validate_scene_payload` also requires `byte_length` to equal width × height × 2, width × height, and point count × 12 respectively. Audio byte length is bounded but is not inferred from rounded millisecond duration. The reference contract does not establish an artifact reader or a physical capture path.

Verification from the repository root: `cargo fmt --all`, `cargo fmt --all -- --check`, `cargo test -p aether-node-profile`, `cargo clippy -p aether-node-profile --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all` passed. Full command output is saved locally in ignored `data/test/20261115-eos-scene-data/`.

<a id="skills_rust"></a>
<a id="skills_rust--rust-skill-executable-contract"></a>
## Rust skill executable contract

[中文](../../docs_cn/eos/interfaces_cn.md#skills_rust_cn) · [SkillService](interfaces/node/skill/demo.md#skill_demo) · [Development guide](core/skills/host-rust.md)

EOS runs selected Rust skill executables as confined child processes. An author implements the `aether_skill::Skill` trait and calls `aether_skill::serve` from `main`. The skill declares one `Descriptor` with a name, SemVer version, description, object-valued parameter and result JSON Schemas, declared primitives, and reentrancy. A single executable may register up to 256 skills. Names use lowercase letters, digits, `.`, `_`, and `-`; each separator must be followed by a letter or digit.

The author contract is [`crates/aether-skill/src/lib.rs`](../../crates/aether-skill/src/lib.rs). `Skill::execute` receives a command ID, object arguments, and a `Host`. `Host::call_primitive` is the only actuator route. The result is one `Completion` with an object result. `Host::is_cancelled` reports a Stop request. EOS validates descriptor schemas and primitive declarations before publishing the catalog, then validates execution arguments and broker admission for each request.

<a id="skills_rust--graph-selection"></a>
### Graph selection

The EOS process node's optional `settings.skill_executables` is an array of at most 16 explicit entries. Each entry has `package: {id, version}` and `executable`, a relative path ending in `.skill`. IsOS verifies the package, exact path and digest at Apply and before Run or Restart. It writes `isos/skill-selection.json` into the selected generation and passes its path to EOS through `EOS_SKILL_SELECTION_FILE`. Package installation alone does not select a skill.

The selection file has integer `schema_version: 1`, the saved graph's SHA-256 `active_revision`, and an `executables` array sorted by package ID, package version and executable path. Each entry contains `package_id`, `package_version`, absolute `package_root` and `payload_root`, `receipt_sha256`, `release_sha256`, `payload_sha256`, relative `executable`, and `executable_sha256`. All digests are lowercase SHA-256 hex. EOS rejects changed files, path escapes, links, nonregular files, unsafe permissions, unsupported ELF dependencies and duplicate skill identities. It stages verified bytes privately and inspects each executable in a confined worker before publishing any descriptor.

The runtime reports feature `rust-skill-v1`. A nonempty selection requires that exact feature during IsOS candidate preflight. The preflight report echoes package identity, executable path, digest and skill count, with no package code path or grant exposed through SkillService.

<a id="skills_rust--process-and-protocol"></a>
### Process and protocol

Each inspection and execution starts a fresh `aether-runtime --skill-worker` child. The runtime passes only a private `SOCK_SEQPACKET` socket and the staged executable file descriptor. The child clears its environment, verifies its peer and descriptors, sets resource limits, Landlock and seccomp, then executes the staged Rust program. The process protocol is version `2` and carries `Inspect`, `Invoke`, `PrimitiveReply`, `Catalog`, `PrimitiveCall`, `Completion` and `Failure` messages. Messages are bounded; large messages use a sealed memory file. EOS owns cancellation, bounded reap, broker authorization and live status.

The skill process cannot open a network socket or create another process. Its primitive request is checked against the admitted descriptor, graph binding, dispatcher and deadline. Stop requests signal the process and lead to bounded termination if it does not exit. EOS stops admitted actuators before terminating its selected package service child.

<a id="skills_rust--authoring-and-verification"></a>
### Authoring and verification

The runnable [`echo` example](../../crates/aether-skill/examples/echo.rs) shows the trait and completion shape. Build a skill as a Linux executable whose final filename ends in `.skill`; use a Cargo locked build. Build the binary with Cargo, include it in a verified IsOS skill release, and select its exact path through the graph.

Run `cargo fmt --all -- --check`, `cargo test -p aether-skill`, and `cargo test -p aether-runtime --features test-hooks --test rust_skill_worker --test rust_skill_catalog`. The process tests cover catalog admission, changed payload rejection, brokered primitive calls, completion and child reap.
