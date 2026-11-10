# EOS v2 primitive bindings and typed command envelope

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](arm-control.md) for the active data and command path and [owner lifecycle](owner-health.md) for setup, Run, Status, and Stop.


[中文](../../../docs_cn/eos/interfaces/primitive-bindings_cn.md) · [Hardware profile](hardware.md) · [Graph controls](control.md)

An EOS primitive binding names one versioned movement operation and connects it to exactly one hardware device. The IsOS graph owns the binding edge; EOS reads it, validates a typed command header, and sends the decoded Rust movement through the shared Dispatcher. The binding does not own a driver or authorize direct actuator release.

## Graph contract

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
| `node.adapter.type` | string | family ID | Selects one device-owned compiled factory such as `mujoco-v2` or `mock-cxx`. |
| `node.adapter.package` | `{id:string,version:string}` or absent | package reference | Required for a package family; exact verified package pin, independent of the built-in node type's `node.package`. |
| `node.adapter.settings` | object | family specific | Validated device endpoint settings; MuJoCo uses `backend`, `socket_path`, `simulator_node`, and `device_id`. |

The registry catalog includes `primitive_specs` with `name`, `version`, `device_class`, `movement_type`, and `description`. It includes `adapter_families` with `id`, `device_kinds`, `transport`, `description`, `supported_primitives`, optional `settings_schema`, optional exact `package`, and EOS-computed `compiled` and `reason`. Package sidecars may declare the family fields but cannot assert that code is compiled. `GET /api/eos/v2/primitive-specs` and `GET /api/eos/v2/adapter-families` expose these catalogs. `aether-profile catalog --node-types <verified-sidecar>` exposes the same data. The compiled `mujoco-v2` family admits `island/mujoco-sim@0.1.1` camera, base, and manipulator devices. One adapter owns a physical device and its typed methods, including stop; a primitive does not create another adapter.

## Bounded runtime command

The EOS owner-only Unix control socket currently accepts one JSON line, up to 64 KiB. `primitive_typed` is the command operation for new bindings. Its `payload` is a typed movement object from [hardware v2](hardware.md); the JSON bytes counted by the **private** `payload_length_bytes` are the compact `serde_json` UTF-8 serialization of that parsed object. The loopback HTTP bridge accepts a bounded public compact UTF-8 length and normalizes the private header after parsing because JavaScript and Rust can serialize the same finite number with different byte lengths; see [the bridge contract](control.md#loopback-api). The decoder rejects unknown fields, lengths above 16 KiB, unsupported codec or version, a forged graph target, and a primitive/payload mismatch before asking Dispatcher to admit the command. Dispatcher still checks health, deadlines, limits, journaling, and stop.

For a browser invocation with the merged EOS R03 implementation, the existing loopback `/api/eos/v2/control` bridge allowlists `primitive_typed` and forwards it to this private runtime operation after Origin and size checks. A caller can send `deadline_after_ms`, which EOS resolves against its own monotonic clock. Existing absolute-deadline callers obtain EOS monotonic time from the bridge. EOS validates the typed header, exact active binding and target again. The response's `command_id`, exact `Completed`/`Cancelled`/`Rejected`/`InProgress`/`OutcomeUnknown` status and reason drive the panel's result display; browser-local sending is only a pending indicator. The browser bridge rewrites a bounded public `result` to guarded private `primitive_result`: the [R07 authorized result contract](control.md#r07-authorized-browser-result) defines the bounded read needed before the UI follows `InProgress`. An uncertain outcome is never shown as completed. This is the UI ingress to the existing contract, not a new graph wire or gRPC service name.

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

### Stream movement

The IsOS HTTP bridge sends `delivery_mode:"stream"`, the current `run_id`, declared `gateway_path`, and `deadline_after_ms` for each `base_movement` command. EOS allows streaming only for `body_velocity` and checks the active revision, run ID, and graph edge before publication. A later `primitive_result` may return `Completed` with reason `stream lease published`: this proves adapter publication, not physical standstill. Each command replaces the preceding bounded lease; on key release, a zero-velocity command promptly replaces continuing motion. The gateway and primitive binding on this path both declare `max_rate_hz:20`.
