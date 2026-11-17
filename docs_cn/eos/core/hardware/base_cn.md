# hardware/base 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/camera 分组](camera_cn.md)
  - [hardware/gripper 分组](gripper_cn.md)
  - [开发 EOS 硬件适配器](hardware-guide_cn.md)
  - [EOS 硬件运行时设计](hardware_cn.md)
  - [hardware/manipulator 分组](manipulator_cn.md)
  - [hardware/mock 分组](mock_cn.md)
  - [hardware/torso 分组](torso_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/core/hardware/base.md)

以下概述当前 EOS 节点类型及其运行时职责。准确载荷与拒绝规则见所链接的所有者接口。

## eos.hardware.base.device@v0.2

软件包候选节点。软件包提供的底盘设备。 输入：可选 `command`：`base_twist_command@v0.2`。 输出：`observation`：`base_state@v0.2`。 EOS 把声明的执行器端口绑定 Dispatcher，验证设备族及端点就绪，并在服务退出前要求运行绑定的 Stop 证明。 接口：[general-hardware_cn](../../interfaces_cn.md#general_hardware_cn)。

## eos.hardware.base.normalized_axes@v0.2

软件包候选节点。软件包提供的归一化轴底盘。 输入：可选 `command`：`normalized_axes_command@v0.2`。 输出：`observation`：`pose_state@v0.2`。 EOS 验证所选设备族、坐标系和租期边界；Dispatcher 接纳命令，Stop 使执行器静止。 接口：[general-hardware_cn](../../interfaces_cn.md#general_hardware_cn)。

<a id="移动底盘适配器节点"></a>

## 移动底盘适配器节点

**位置：**`isos/eos_hardware/base/<instance-id>` · **类型族：**`isos.eos_hardware.base@v0.2`。注册类型是可复用定义；部署时选择实例 ID 和非空实例说明。

**状态：** 历史 `isos.eos_*` 设计参考。此类型不是当前内置 EOS 节点库条目。

### 用途与边界

一个底盘拥有观测和物理停止接口。输入边描述命令路径，但不能直接授权执行。

### 端口与设置

输入：`command: BaseMovement`（可有多个兼容来源）。输出：`observation: BaseObservation`、`outcome: CommandOutcome`、`health: Health`、`status: Status`。仅允许声明过的 `body_velocity` 和/或 `relative_pose` 模式。所属接口页定义精确字段类型、单位、默认值、校验和示例。注册类型须有非空 `NodeType.description`。

### 生命周期与算法

解析已验证适配器，检查命令来源和类型化限值，经 Dispatcher 准入后才交给唯一适配器。租约到期或停止时调用适配器停止接口；无法确认完成时保留 `OutcomeUnknown`，不自动重试。 节点状态、拒绝原因和关联 ID 写入共享部署日志。
