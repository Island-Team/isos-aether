# Graph 0.2.0 node interface

[中文](../../../../docs_cn/isos/interfaces/node/README_cn.md) · [IsOS interfaces](../README.md)

A graph contains exactly three cores: `isos`, `ei`, and `eos`. A registered type ID has the form `<core>.<layer>.<group>.<node>@v0.2`; its prefix must match the instance path. The registered type owns placement, ports, and lifecycle. An instance supplies its ID, type, label, settings, and declared inputs. Each owner publishes and validates its own types. IsOS combines those catalogs and validates the graph at Apply.

The starter has two disconnected eight-node chains in its 16-node button and monitor demo. IsOS owns the saved graph and run fence. EI validates and passes the activation through input, cognition, and safety. EOS maps it to a bounded six-joint delta, admits it through the dispatcher, and updates the mock arm adapter. The button carries no movement target. A read-only chain returns the same arm's observed state through every EOS and EI layer to an IsOS UI monitor node.

| Core | Interface wrappers in the starter graph | Runtime responsibility |
| --- | --- | --- |
| IsOS | `isos.ui.button.trigger@v0.2`, `isos.ui.monitor.arm@v0.2` | Emit one run-fenced activation and display the return observation. |
| EI | Input, cognition, and governance nodes for activation and monitor state. | Validate and pass both declared paths. |
| EOS | Interface, skill, dispatcher, and hardware nodes for control and monitoring. | Admit movement, sample the same mock arm, and check its state before further movement. |

The 14 edges are explicit in the saved graph. `button_activation@v0.2` crosses IsOS → EI and EI → EOS owner sockets; `arm_joint_delta@v0.2` stays within EOS. Accepted activation and its eventual result share one event ID; the result is read separately rather than sent back along the observation chain. The return chain carries latest `arm_monitor_state@v0.2` snapshots from EOS through EI to IsOS independently of button requests. The mock adapter holds six positions in memory and does not contact physical hardware. See [node events](../ui/node-events.md) and [arm monitor](../ui/node-monitor.md) for the client boundaries.

IsOS sends each owner only its own `cores.<owner>` projection. Owner services can outlive IsOS. On restart, IsOS reconnects to them and reads the persisted run identity; a failure in one owner does not stop the other. Explicit deployment Stop asks both owners to stop and requires both replies before retiring the run identity.

See [EI setup](../../../ei/interfaces/setup.md), [EOS owner health](../../../eos/interfaces/owner-health.md), [graph API](../http/graph.md), and the [starter graph](../../../../crates/isos/templates/generic-graph.toml).
