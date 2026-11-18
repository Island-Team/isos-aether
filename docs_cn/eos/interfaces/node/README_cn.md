# EOS 节点接口

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node</summary>

浏览本目录的文档及下级目录。

- [上级目录](../)
- 下级目录：
  - [design/](design/README_cn.md)
  - [dispatcher/](dispatcher/)
  - [hardware/](hardware/)
  - [interface/](interface/)
  - [skill/](skill/)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/interfaces/node/README.md) · [EOS 接口](../../interfaces_cn.md) · [共享节点契约](../../../node/interfaces/README_cn.md)

EOS 发布以下 `<layer>/<group>` 节点契约。每个分组固定类型身份、设置、类型化端口和所有者行为。[共享节点契约](../../../node/interfaces/README_cn.md)定义配置位置、数据描述符、连线及注册表准入。EOS 验证选定的图投影，并且只通过 Dispatcher 准入执行器命令。

| 层 | 分组 |
| --- | --- |
| `interface` | [ingress](interface/ingress_cn.md)、[egress](interface/egress_cn.md) |
| `skill` | [demo](skill/demo_cn.md) |
| `dispatcher` | [arm](dispatcher/arm_cn.md) |
| `hardware` | [mock](hardware/mock_cn.md)、[camera](hardware/camera_cn.md)、[base](hardware/base_cn.md)、[manipulator](hardware/manipulator_cn.md)、[gripper](hardware/gripper_cn.md)、[torso](hardware/torso_cn.md) |

[通用硬件](../../interfaces_cn.md#general_hardware_cn)、[机械臂控制](../../interfaces_cn.md#arm_control_cn)和[机械臂观测](../../interfaces_cn.md#arm_monitor_cn)契约定义载荷语义与所有者路线。[IsOS–EOS 服务端接口](../../../isos/interfaces/server/eos_cn.md)定义设置、生命周期及软件包交接。

节点行为设计参见[节点设计索引](design/README_cn.md)。
