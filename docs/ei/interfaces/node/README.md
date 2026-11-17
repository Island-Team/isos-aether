# EI node interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node</summary>

Browse documents and subfolders here.

- [Parent folder](../)
- Subfolders:
  - [cognition/](cognition/)
  - [evidence/](evidence/)
  - [governance/](governance/)
  - [io/](io/)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/ei/interfaces/node/README_cn.md) · [EI interfaces](../../interfaces.md) · [Shared node contracts](../../../node/interfaces/README.md)

EI publishes the following `<layer>/<group>` node contracts. Each group fixes type identity, settings, typed ports, and owner behavior. The [shared node contracts](../../../node/interfaces/README.md) define profile placement, data descriptors, edges, and registry admission. IsOS supplies EI only its selected graph projection and exact verified package references; EI validates its routes and enforces its safety gate before EOS receives an activation.

| Layer | Groups |
| --- | --- |
| `io` | [input](io/input.md), [link](io/link.md) |
| `evidence` | [perception](evidence/perception.md) |
| `cognition` | [context](cognition/context.md), [decision](cognition/decision.md), [intent](cognition/intent.md), [missions](cognition/missions.md), [state](cognition/state.md) |
| `governance` | [monitor](governance/monitor.md), [safety](governance/safety.md) |

The [isolated cognition library](../../interfaces.md#cognition_isolated_library) is a direct-call interface without graph routes. The [IsOS–EI server interface](../../../isos/interfaces/server/ei.md) owns setup and run-fenced owner exchanges.
