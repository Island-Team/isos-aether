# interface/egress 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core/interface</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [interface/ingress 分组](ingress_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/core/interface/egress.md)

以下概述当前 EOS 节点类型及其运行时职责。准确载荷与拒绝规则见所链接的所有者接口。

## eos.interface.egress.arm_status@v0.2

向 EI 发布已检查的机械臂状态。 输入：`observation`：`arm_monitor_state@v0.2`，进程内。 输出：`observation`：`arm_monitor_state@v0.2`，EI Unix socket。 桥接发布有界且受运行围栏约束的观测；Stop 结束实时发布。 接口：[arm-monitor_cn](../../interfaces_cn.md#arm_monitor_cn)。

## eos.interface.egress.vision_flow@v0.2

将相机帧传至 IsOS Vision。 输入：`frame`：`image_frame@v0.2`，进程内事件。 输出：`frame`：`image_frame@v0.2`，Unix socket 流。 只有图中声明的相机来源可输入此桥接。EOS 校验每帧并绑定当前运行；Stop 结束流。 接口：[vision_cn](../../interfaces/node/interface/egress_cn.md#interface_egress_cn--vision)。
