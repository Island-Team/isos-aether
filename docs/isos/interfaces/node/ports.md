# Node ports

[中文](../../../../docs_cn/isos/interfaces/node/ports_cn.md) · [Type](type.md) · [Edges](edges.md)

An input port receives and an output port sends. A live port declares a communication channel in `transport`, a delivery mode, an exact registered payload `data_type`, and a protocol `contract` with `owner`, `id`, and `version`. Both ends must agree, and IsOS admits the edge only when an owner-supported route implements it. A matching payload name alone does not establish delivery.

The starter graph's eight types have typed ports. The IsOS → EI and EI → EOS boundaries use `unix_socket`; owner-local edges use `in_process`. Activation edges carry `button_activation@v0.2`; the two EOS command edges carry `arm_joint_delta@v0.2`.

| Field | Meaning |
| --- | --- |
| `name` | Unique port name within its direction. |
| `data_type` | Exact registered payload type, or `topology_link` for a static declaration. |
| `required` | Whether admission requires a connection. |
| `transport` | Required channel: `none` for a static declaration; an explicit supported carrier for a live port. |
| `delivery_mode` | `declaration` for a static declaration; the exact live delivery behavior otherwise. |
| `contract` | Required `{owner,id,version}` protocol identity for a live port; forbidden on a static port. |
| `cardinality` | Connection count, normally `one`. |

`none` and `declaration` must occur together. IsOS checks source and destination declarations during Apply. EI and EOS independently validate their owner projections. Runtime health comes from owner process status, not from a port. The [shared type schema](../../../../share/isos/contracts/node/node-type.schema.json) defines the full port vocabulary.
