# hardware/mock 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/design/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/base 分组](base_cn.md)
  - [hardware/camera 分组](camera_cn.md)
  - [hardware/gripper 分组](gripper_cn.md)
  - [hardware/manipulator 分组](manipulator_cn.md)
  - [hardware/torso 分组](torso_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/eos/interfaces/node/design/hardware/mock.md)

以下概述当前 EOS 节点类型及其运行时职责。准确载荷与拒绝规则见所链接的所有者接口。

## eos.hardware.mock.arm@v0.2

保存六个模拟关节位置。 输入：`delta`：`arm_joint_delta@v0.2`，进程内。 输出：`observation`：`arm_monitor_state@v0.2`，进程内。 `mock_arm` 适配器在每次运行中从零开始，应用获准且不重复的增量并采样关节位置。Stop 在子进程退出前关闭接纳。 接口：[arm_cn](../../hardware/mock_cn.md#hardware_mock_cn--arm)。

## eos.hardware.mock.arm_observer@v0.2

独立读取唯一的模拟机械臂采样器。 输入：无图输入。 输出：`observation`：`arm_monitor_state@v0.2`，进程内。 观测来源绑定当前运行；此节点不能接纳或移动执行器。 接口：[arm-monitor_cn](../../../../interfaces_cn.md#arm_monitor_cn)。

## eos.hardware.mock.arm_output@v0.2

将模拟机械臂观测传给监视链。 输入：`observation`：`arm_monitor_state@v0.2`，进程内。 输出：`observation`：`arm_monitor_state@v0.2`，进程内。 用于已保存图的兼容类型；仅转发观测，不创建另一条执行器路径。 接口：[arm_cn](../../hardware/mock_cn.md#hardware_mock_cn--arm)。
