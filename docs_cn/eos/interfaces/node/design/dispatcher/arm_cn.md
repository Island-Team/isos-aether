# dispatcher/arm 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/design/dispatcher</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [底盘原语绑定节点](base_cn.md)
  - [共享控制调度器节点](control_cn.md)
  - [机械臂原语绑定节点](manipulator_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/eos/interfaces/node/design/dispatcher/arm.md)

以下概述当前 EOS 节点类型及其运行时职责。准确载荷与拒绝规则见所链接的所有者接口。

## eos.dispatcher.arm.delta@v0.2

在模拟机械臂移动前接纳六关节演示命令。 输入：`delta`：`arm_joint_delta@v0.2`，进程内。 输出：`delta`：`arm_joint_delta@v0.2`，进程内。 在命令交给图中声明的模拟适配器前，检查运行身份、健康和移动限值。 接口：[arm-control_cn](../../../../interfaces_cn.md#arm_control_cn)。

## eos.dispatcher.arm.monitor@v0.2

检查模拟机械臂的返回观测。 输入：`observation`：`arm_monitor_state@v0.2`，进程内。 输出：`observation`：`arm_monitor_state@v0.2`，进程内。 拒绝过期或无效样本；监视路径独立于控制结果读取。 接口：[arm-monitor_cn](../../../../interfaces_cn.md#arm_monitor_cn)。
