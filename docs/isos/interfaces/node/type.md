# Registered node type

[中文](../../../../docs_cn/isos/interfaces/node/type_cn.md) · [Node interfaces](README.md) · [Instance](instance.md)

A registered type fixes `core`, `layer`, `group`, settings, ports, capabilities, and lifecycle. Its ID has the form `<core>.<layer>.<group>.<type>@v0.2`, where `core` is `isos`, `ei`, or `eos`. An instance supplies identity and declared values; it cannot change placement or behavior. IsOS merges owner catalogs and verifies each selected type at Apply. EI and EOS implement their own interface behavior; graph nodes are wrappers over those interfaces.

| Field | Current meaning |
| --- | --- |
| `id`, `core`, `layer`, `group` | Exact versioned identity and saved placement. |
| `kind`, `description`, `lifecycle` | Display role and owner lifecycle. |
| `capabilities` | Owner-advertised functions; an empty array is valid. |
| `settings` | Allowed instance values and defaults. |
| `inputs`, `outputs` | Typed connection declarations. |
| `deprecated` | Legacy type remains valid for saved graphs but is hidden from IsosUI's add menus. |
| `implementation`, `service_inputs`, `service_outputs` | Optional owner implementation and service declarations; a declaration alone does not enable a route. |

The starter graph uses one IsOS button, three EI pass nodes, and four EOS nodes, connected by seven declared edges. The button emits `button_activation@v0.2`; the EOS skill and dispatcher pass `arm_joint_delta@v0.2` to the mock arm. The mock arm is a Manipulator/Adapter instance with six joint names and bounded movement settings. See the [starter graph](../../../../crates/isos/templates/generic-graph.toml) for exact IDs and fields.

The authoritative declaration shape is the [shared type schema](../../../../share/isos/contracts/node/node-type.schema.json). See [ports](ports.md) and [edges](edges.md).
