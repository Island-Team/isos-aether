# hardware/torso 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/base 分组](base_cn.md)
  - [hardware/camera 分组](camera_cn.md)
  - [hardware/gripper 分组](gripper_cn.md)
  - [开发 EOS 硬件适配器](hardware-guide_cn.md)
  - [EOS 硬件运行时设计](hardware_cn.md)
  - [hardware/manipulator 分组](manipulator_cn.md)
  - [hardware/mock 分组](mock_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/core/hardware/torso.md)

以下概述当前 EOS 节点类型及其运行时职责。准确载荷与拒绝规则见所链接的所有者接口。

## eos.hardware.torso.normalized_axes@v0.2

软件包候选节点。软件包提供的归一化轴躯干。 输入：可选 `command`：`normalized_axes_command@v0.2`。 输出：无虚构的位置输出。 EOS 验证所选设备族及租期，经 Dispatcher 接纳移动，并要求运行绑定的 Stop 证明。 接口：[general-hardware_cn](../../interfaces_cn.md#general_hardware_cn)。
