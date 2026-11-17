# Shared node graph interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: node/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Shared node interfaces](README.md)
  - [Shared node registry interfaces](registry.md)
  - [Shared node type interfaces](type.md)

</details>
<!-- doc-navigation:end -->

<a id="profile"></a>
<a id="profile--node-profile-in-the-saved-graph"></a>
## Node profile in the saved graph

[中文](../../../docs_cn/node/interfaces/graph_cn.md#profile) · [Node interfaces](../../isos/interfaces/node/README.md) · [Instance](graph.md#instance)

A node profile is a saved instance, not a second type declaration or a live status record. Graph format `0.2.1` contains the `isos`, `ei`, and `eos` cores. A registered type fixes `<core>.<layer>.<group>.<type>@v0.2`; the profile lives at the matching `cores.<core>.layers.<layer>.groups.<group>.nodes.<id>` path. The map key and `id` agree.

```text
cores.isos.layers.ui.groups.button.nodes.trigger
cores.ei.layers.io.groups.input.nodes.trigger
cores.ei.layers.cognition.groups.intent.nodes.forwarder
cores.ei.layers.governance.groups.safety.nodes.gate
cores.eos.layers.interface.groups.ingress.nodes.trigger
cores.eos.layers.skill.groups.demo.nodes.joint_delta
cores.eos.layers.dispatcher.groups.arm.nodes.delta
cores.eos.layers.hardware.groups.mock.nodes.arm
```

Each profile selects its type and may supply a label, description, declared settings, and input references when its registered type has ports. The seven starter edges have typed inputs and outputs. The [generic graph template](../../../crates/isos/templates/generic-graph.toml) contains the full deployable example.

IsOS saves and verifies the graph, then renders owner projections. EI and EOS validate their own projections and supervise their processes. Owner status reports readiness and failure independently. The mock hardware adapter stores six joint positions in memory; it does not contact physical hardware.

<a id="instance"></a>
<a id="instance--saved-node-instance"></a>
## Saved node instance

[中文](../../../docs_cn/node/interfaces/graph_cn.md#instance) · [Profile](graph.md#profile) · [Type](type.md#type)

Graph format `0.2.1` has exactly three core keys: `isos`, `ei`, and `eos`. Instances live at `cores.<core>.layers.<layer>.groups.<group>.nodes.<id>`. The map key equals `id`; `node_type` selects a registered `<core>.<layer>.<group>.<type>@v0.2` at that exact placement. The canonical reference path is `<core>/<layer>/<group>/<id>`.

| Field | Meaning |
| --- | --- |
| `id`, `node_type` | Instance identity and registered wrapper type. |
| `label`, `description` | Display text. `label` must be nonblank; `description` may be omitted or an empty string. |
| `settings` | Only settings declared by the selected type. |
| `inputs` | References to declared source ports. |
| `package` | Exact verified release pin when a package type requires one. |

Type identity is the full `node_type` ID plus its exact `package` `{id, version}` pin. Two installed releases may declare the same type ID and placement; their settings and ports do not merge. A contributed type without a matching instance pin is rejected. A built-in type has no top-level package pin. For package hardware, `package` selects the verified type and `adapter.package` selects the verified adapter and service; EOS checks both pins. Only the mock hardware types remain built in.

Communication roles belong to the registered type's optional `interaction_roles`, not to each saved instance. A graph profile does not repeat or override those roles. A node with both live inputs and outputs may be presented as both subscriber and publisher; route admission still depends on its exact typed ports and owner-supported edges.

The starter graph contains 17 nodes with 17 declared input connections, including EI context inputs from activation and arm observation. The saved graph does not contain live process state, owner socket paths, credentials, event values, or a hardware command. IsOS owns revisions and Apply; EI and EOS receive their respective core projections. The EOS mock arm instance declares six joints and its adapter and movement limits. Contributors maintaining the validator can find its machine-readable instance definition in [`contract/node/`](../../../contract/node/README.md).

The following saved instance is the IsOS arm monitor from the mixed graph. It sits under `cores.isos.layers.ui.groups.monitor.nodes.arm`; `inputs.observation` names its source node and output port. The `refresh_rate_hz` setting is a display limit in hertz, not the source sampling rate.

```json
{
  "id": "arm",
  "node_type": "isos.ui.monitor.arm@v0.2",
  "label": "Return: arm state",
  "description": "Read the latest arm state independently of movement completion",
  "settings": {
    "refresh_rate_hz": 5
  },
  "inputs": {
    "observation": {
      "node": "ei/governance/monitor/arm_status",
      "output": "observation"
    }
  }
}
```

<a id="edges"></a>
<a id="edges--graph-edges"></a>
## Graph edges

[中文](../../../docs_cn/node/interfaces/graph_cn.md#edges) · [Ports](type.md#ports)

An input reference has the form `{"node":"core/layer/group/node","output":"out"}`. The destination stores it under its declared input name. IsOS checks that both instances exist at registered placements, that the ports match, and that the graph has no cycle. A cross-core `topology_link` is admitted when both ends declare `transport: "none"` and `delivery_mode: "declaration"`; it needs no runtime bridge because no payload crosses the boundary.

A live edge additionally requires matching transport, delivery mode, exact registered payload ID, and the same protocol `contract` on sender and receiver ports. When both ports declare `data_class`, IsOS rejects differing classes with `edge_data_class`. The trusted owner catalog must contain a `port_routes` entry naming the exact source type and output port, target type and input port, transport, delivery mode, payload ID, and protocol contract. For cross-core Unix socket bridges, both endpoint profiles must also declare the bridge port and have owner bridge adapters. IsOS rejects an edge without this route proof. The validation response names the exact `source`, `output`, `target`, and `input` so clients can identify the admitted connection. Static `topology_link` edges remain declarations and carry no payload.

The starter graph has 17 input connections. A button activation passes through the EI context and safety nodes before EOS; EOS alone translates it into a six-joint delta and admits it to the mock arm.

The built-in catalog publishes only the exact live `port_routes` used by the demo. Other live edges require an owner implementation and matching route registration.
