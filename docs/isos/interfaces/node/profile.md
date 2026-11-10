# Node profile in the saved graph

[中文](../../../../docs_cn/isos/interfaces/node/profile_cn.md) · [Node interfaces](README.md) · [Instance](instance.md)

A node profile is a saved instance, not a second type declaration or a live status record. Graph format `0.2.0` contains the `isos`, `ei`, and `eos` cores. A registered type fixes `<core>.<layer>.<group>.<type>@v0.2`; the profile lives at the matching `cores.<core>.layers.<layer>.groups.<group>.nodes.<id>` path. The map key and `id` agree.

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

Each profile selects its type and may supply a label, description, declared settings, and input references when its registered type has ports. The seven starter edges have typed inputs and outputs. The [generic graph template](../../../../crates/isos/templates/generic-graph.toml) contains the full deployable example.

IsOS saves and verifies the graph, then renders owner projections. EI and EOS validate their own projections and supervise their processes. Owner status reports readiness and failure independently. The mock hardware adapter stores six joint positions in memory; it does not contact physical hardware.
