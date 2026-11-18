# EOS 节点设计

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/design</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 下级目录：
  - [dispatcher/](dispatcher/)
  - [hardware/](hardware/)
  - [interface/](interface/)
  - [skill/](skill/)

</details>
<!-- doc-navigation:end -->

这里记录各节点的职责、生命周期与故障行为。精确的类型、端口、设置和载荷由相邻节点接口契约定义。

| 分组 | 设计 |
| --- | --- |
| `dispatcher` | [arm](dispatcher/arm_cn.md), [base](dispatcher/base_cn.md), [control](dispatcher/control_cn.md), [manipulator](dispatcher/manipulator_cn.md) |
| `hardware` | [base](hardware/base_cn.md), [camera](hardware/camera_cn.md), [gripper](hardware/gripper_cn.md), [manipulator](hardware/manipulator_cn.md), [mock](hardware/mock_cn.md), [torso](hardware/torso_cn.md) |
| `interface` | [egress](interface/egress_cn.md), [ingress](interface/ingress_cn.md) |
| `skill` | [demo](skill/demo_cn.md) |
