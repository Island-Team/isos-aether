# hardware/manipulator 分组

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
  - [hardware/mock 分组](mock_cn.md)
  - [hardware/torso 分组](torso_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/core/hardware/manipulator.md)

以下概述当前 EOS 节点类型及其运行时职责。准确载荷与拒绝规则见所链接的所有者接口。

## eos.hardware.manipulator.device@v0.2

软件包候选节点。软件包提供的关节位置机械臂。 输入：可选 `command`：`joint_position_command@v0.2`。 输出：`observation`：`joint_state@v0.2`。 EOS 验证关节顺序及限值；Dispatcher 接纳移动，Stop 在服务退出前使已接纳命令静止。 接口：[general-hardware_cn](../../interfaces_cn.md#general_hardware_cn)。

## eos.hardware.manipulator.normalized_cartesian@v0.2

软件包候选节点。软件包提供的笛卡尔机械臂。 输入：可选 `command`：归一化轴；可选 `mode`：`control_mode_command@v0.2`。 输出：`observation`：`pose_state@v0.2`。 EOS 检查已公布模式及声明的坐标系；Dispatcher 接纳命令，Stop 要求执行器证明。 接口：[general-hardware_cn](../../interfaces_cn.md#general_hardware_cn)。

<a id="机械臂适配器节点"></a>

## 机械臂适配器节点

**位置：**`isos/eos_hardware/manipulator/<instance-id>` · **类型族：**`isos.eos_hardware.manipulator@v0.2`。注册类型是可复用定义；部署时选择实例 ID 和非空实例说明。

**状态：** 历史 `isos.eos_*` 设计参考。此类型不是当前内置 EOS 节点库条目。

### 用途与边界

一个机械臂拥有按顺序排列的关节状态、可选末端位姿和物理停止接口。节点配置中的关节顺序为准。

### 端口与设置

输入：`command: ManipulatorMovement`（可有多个兼容来源）。输出：`observation: ManipulatorObservation`、`outcome: CommandOutcome`、`health: Health`、`status: Status`。仅允许声明过的 `joint_positions` 和/或 `end_effector_pose` 模式。所属接口页定义精确字段类型、单位、默认值、校验和示例。注册类型须有非空 `NodeType.description`。

### 生命周期与算法

在 Dispatcher 准入前验证关节名、顺序、有限目标值、模式和限值。适配器报告新鲜观测状态；确认接收不等于达到目标。图边不能掩盖停止失败或不确定结果。 节点状态、拒绝原因和关联 ID 写入共享部署日志。
