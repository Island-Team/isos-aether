# EOS package node contributions

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](arm-control.md) for the active data and command path and [owner lifecycle](owner-health.md) for setup, Run, Status, and Stop.


[中文](../../../docs_cn/eos/interfaces/node-contributions_cn.md) · [Unified hardware profile](hardware.md) · [Primitive bindings](primitive-bindings.md)

IsOS installs or links Aether packages and owns the applied graph. A package release may contribute registered node types and node sets; IsOS verifies the release and writes a derived, read-only node-type sidecar for EOS. EOS reads the graph and sidecar without changing either. This JSON interface is the EOS-specific node IDL; it describes identities, placement, settings, and typed ports, while hardware samples and commands remain Rust values inside EOS. The example below uses current registered node identities and typed ports.

## Verified node-type sidecar

IsOS passes each verified sidecar as a repeatable `--node-types <absolute-path>` argument to `aether-profile`, `aether-runtime`, and `aether-config-server`. Each file is at most 1 MiB. EOS loads all safety declarations before the corresponding types, rejects duplicate or malformed definitions, and validates the graph against the combined built-in and contributed registry. All enum values use `snake_case`; unknown fields fail decoding.

```json
{
  "package": {
    "id": "island/mujoco-sim",
    "version": "0.1.1"
  },
  "safety_adapters": [],
  "data_types": [],
  "adapter_families": [
    {
      "id": "mujoco-v2",
      "device_kinds": [
        "camera",
        "base",
        "manipulator"
      ],
      "transport": "unix_socket",
      "description": "EOS-owned MuJoCo simulator socket adapter for camera, base, and manipulator devices",
      "supported_primitives": [
        "base.velocity@1.0.0",
        "arm.movej@1.0.0"
      ]
    }
  ],
  "node_types": [
    {
      "id": "isos.eos_process.mujoco@v0.2",
      "core": "isos",
      "layer": "eos_process",
      "group": "mujoco",
      "kind": "software",
      "lifecycle": "process",
      "capabilities": [
        "process"
      ],
      "settings": [
        {
          "name": "scene",
          "data_type": "string",
          "required": true,
          "description": "Stable verified scene ID",
          "selector": {
            "provider": "package",
            "resource": "scene",
            "package_from": "node.package"
          }
        },
        {
          "name": "socket",
          "data_type": "string",
          "required": true,
          "description": "Owner-only simulator Unix socket path"
        },
        {
          "name": "env",
          "data_type": "string_table",
          "required": false,
          "description": "Simulator child environment references"
        },
        {
          "name": "start_timeout_ms",
          "data_type": "positive_integer",
          "required": false,
          "description": "Simulator readiness deadline in milliseconds"
        }
      ],
      "inputs": [],
      "outputs": [
        {
          "name": "status",
          "data_type": "status",
          "required": false,
          "description": "Simulator process lifecycle",
          "transport": "in_process",
          "data_class": "state"
        },
        {
          "name": "health",
          "data_type": "health",
          "required": false,
          "description": "Simulator process health",
          "transport": "in_process",
          "data_class": "state"
        }
      ],
      "safety_adapter_type": null,
      "description": "Starts one verified MuJoCo scene as an EOS-owned simulator child and supplies only the graph-declared simulated devices."
    }
  ]
}
```

`package` is an exact `{id, version}` reference. The ID has two valid address segments separated by `/`; version has three nonempty numeric components. A contributed node instance must carry the same `node.package` value as the type's sidecar. Built-in nodes omit `node.package`. Registration checks a declared stop capability, but EOS admits a physical actuator only when a compiled adapter and stop implementation exist for its type and package.

`node_types` is an array of `NodeType` definitions from [the hardware contract](hardware.md); `safety_adapters` is an array of `SafetyAdapterType`. Their Rust and JSON fields are identical. `core`, `layer`, and `group` are required placement segments; the graph cannot infer hardware kind from a group. The `kind` values are `camera`, `base`, `manipulator`, and `software`; lifecycle values are `internal`, `process`, and `adapter`. IsOS package node sets use these type IDs and symbolic graph references, but the applied graph contains only concrete paths.

`data_types` is an optional array of `DataTypeDescriptor` entries. Each entry has `id` (string, unique versioned identifier such as `vendor.camera.depth@v0.2`), `version` (string, `0.2`, matching the ID suffix), `purpose` (nonempty description string), `media_type` (nonempty MIME string), `codec` (nonempty registered runtime codec ID), `schema` (JSON object describing logical fields, including `description`, `x-unit`, and JSON Schema validation keywords), and `max_payload_bytes` (positive integer, bytes). Built-in IDs remain snake case, such as `camera_frame` and `vision_frame`. A contributed port can reference a data type only when its codec is compiled into EOS. `json_v1` and `vision_frame_jpeg_v1` are built-in codecs. EOS exposes the combined catalog through `aether-profile catalog` and `GET /api/eos/v2/data-types` on the config server.

An edge requires the exact same registered `data_type` ID at its source and input. Its schema's `x-unit` annotations describe payload fields for display and validation within that type; they do not make two different type IDs interchangeable. No implicit unit conversion occurs on a graph edge. A contributed descriptor cannot redefine a registered ID with different units; duplicate IDs fail registration. IsOS owns edge-addressed Apply diagnostics, while EOS owns runtime health after launch. The [hardware edge contract](hardware.md) defines source QoS, route classification, transport and safety restrictions. A package-contributed port cannot claim a command route merely by setting `data_class: "command"` or matching a built-in movement type: a hardware command input accepts only the registered Dispatcher primitive-binding source.

`adapter_families` is an optional array. Each descriptor requires `id` (valid family segment), `device_kinds` (nonempty list of `camera`, `base`, or `manipulator`), `transport` (one of `in_process`, `unix_socket`, `websocket`, or `ros2`), and `description` (nonblank string). Optional `settings_schema` is a JSON object; optional `supported_primitives` is an array of registered `<name>@<version>` strings. The sidecar's outer `package` supplies exact provenance. Package authors cannot set `compiled`; EOS derives it from its compiled adapter factory and exposes `compiled` and `reason` in the catalog. Generic hardware instances carry this package in `node.adapter.package`, independently of the built-in node type.


## MuJoCo contribution

The MuJoCo package currently contributes `isos.eos_process.mujoco@v0.2` at `isos/eos_process/mujoco` and the `mujoco-v2` adapter family. Its new-graph sidecar contributes `isos.eos_process.mujoco@v0.2` with a nonblank type description and complete setting and port metadata; it does not contribute a new MuJoCo-specific hardware type. Its node set uses generic built-in camera, base, and manipulator `@v0.2` types at `isos/eos_hardware/<kind>/<id>` with `node.adapter.package = {id = "island/mujoco-sim", version = "0.1.1"}`. The process node carries `node.package` because its type is contributed. Its settings are `scene` and `socket`, with optional `env` and `start_timeout_ms`; package identity is not duplicated in settings. Each device adapter references that process node. Base velocity and arm joint-position bindings link to device command inputs; the shared EOS Dispatcher still admits every actuator command.


## Typed data identity

EOS may wrap an in-process hardware value in `NodeDataEnvelope<T>`. The envelope carries `source_node` (canonical graph path), `port` (registered output name), `data_type` (`PortDataType`), `contract_version` (`2.0.0`), and the typed `payload`. `NodeDataEnvelope::output` rejects an output port whose declared type differs from `T::DATA_TYPE`. The envelope is metadata for identifying data against the IsOS graph; it does not grant command admission or create a second actuator path.

`TypedDataHub` publishes such envelopes to registered output keys. `subscribe<T>` validates the node's output port and returns a typed cursor; `poll_latest` returns only a newer value. The hub retains one value per output, so a slow subscriber skips intermediate values without a growing queue. Dropping the cursor disconnects that reader. A compiled `DataTransformer<Input, Output>` names its codec ID and accepts typed options; `CameraJpegTransformer` converts `camera_frame` to `vision_frame` with `vision_frame_jpeg_v1`.

Invalid sidecar JSON, duplicate type IDs, missing safety adapters, package mismatches, unknown types, placement mismatches, and incompatible port edges fail validation before runtime construction. Missing compiled actuator implementations fail runtime adapter construction before the EOS socket binds.

## Target setting selector

`Setting.selector` is an optional JSON object on new type definitions. It describes where IsOS obtains options; it never embeds executable code or an arbitrary filesystem path. Existing `name`, `data_type`, `required`, `description`, `unit`, bounds, and static choices retain the rules in [hardware](hardware.md).

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
  "description": "Verified MuJoCo scene ID",
  "selector": {
    "provider": "package",
    "resource": "scene",
    "package_from": "node.package"
  }
}
```


## Structured setting metadata

A registered `object_array` setting declares an `items_schema` and entry bounds. The registry validates its shape and size; IsOS validates package pins and executable paths. The EOS process node's `settings.skill_executables` selects verified Rust `.skill` files. Each item identifies an exact package and relative executable. The [Rust skill contract](skills/rust.md) defines the selection handoff and worker boundary.
