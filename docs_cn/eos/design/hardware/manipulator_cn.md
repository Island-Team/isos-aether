# 机械臂适配器节点

[English](../../../../docs/eos/design/hardware/manipulator.md) · [EOS 架构](../architecture_cn.md) · [hardware](../../interfaces/hardware_cn.md) · [primitive-bindings](../../interfaces/primitive-bindings_cn.md) · [mujoco](../../interfaces/mujoco_cn.md)

**位置：**`isos/eos_hardware/manipulator/<instance-id>` · **类型族：**`isos.eos_hardware.manipulator@v0.2`。注册类型是可复用定义；部署时选择实例 ID 和非空实例说明。

**状态：** 已注册用于图格式 `0.2.0`。

## 用途与边界

一个机械臂拥有按顺序排列的关节状态、可选末端位姿和物理停止接口。节点配置中的关节顺序为准。

## 端口与设置

输入：`command: ManipulatorMovement`（可有多个兼容来源）。输出：`observation: ManipulatorObservation`、`outcome: CommandOutcome`、`health: Health`、`status: Status`。仅允许声明过的 `joint_positions` 和/或 `end_effector_pose` 模式。所属接口页定义精确字段类型、单位、默认值、校验和示例。注册类型须有非空 `NodeType.description`。

## 生命周期与算法

在 Dispatcher 准入前验证关节名、顺序、有限目标值、模式和限值。适配器报告新鲜观测状态；确认接收不等于达到目标。图边不能掩盖停止失败或不确定结果。 节点状态、拒绝原因和关联 ID 写入共享部署日志。
