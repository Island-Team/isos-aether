# EOS node interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node</summary>

Browse documents and subfolders here.

- [Parent folder](../)
- Subfolders:
  - [design/](design/README.md)
  - [dispatcher/](dispatcher/)
  - [hardware/](hardware/)
  - [interface/](interface/)
  - [skill/](skill/)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/interfaces/node/README_cn.md) · [EOS interfaces](../../interfaces.md) · [Shared node contracts](../../../node/interfaces/README.md)

EOS publishes the following `<layer>/<group>` node contracts. Each group fixes type identity, settings, typed ports, and owner behavior. The [shared node contracts](../../../node/interfaces/README.md) define profile placement, data descriptors, edges, and registry admission. EOS validates its selected graph projection and admits actuator commands only through Dispatcher.

| Layer | Groups |
| --- | --- |
| `interface` | [ingress](interface/ingress.md), [egress](interface/egress.md) |
| `skill` | [demo](skill/demo.md) |
| `dispatcher` | [arm](dispatcher/arm.md) |
| `hardware` | [mock](hardware/mock.md), [camera](hardware/camera.md), [base](hardware/base.md), [manipulator](hardware/manipulator.md), [gripper](hardware/gripper.md), [torso](hardware/torso.md) |

The [general hardware](../../interfaces.md#general_hardware), [arm control](../../interfaces.md#arm_control), and [arm observation](../../interfaces.md#arm_monitor) contracts define payload semantics and owner routes. The [IsOS–EOS server interface](../../../isos/interfaces/server/eos.md) owns setup, lifecycle, and package handoff.

For per-node behavior, see the [node design index](design/README.md).
