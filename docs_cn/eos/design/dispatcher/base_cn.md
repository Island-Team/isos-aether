# 底盘原语绑定节点

[English](../../../../docs/eos/design/dispatcher/base.md) · [EOS 架构](../architecture_cn.md) · [primitive-bindings](../../interfaces/primitive-bindings_cn.md) · [hardware](../../interfaces/hardware_cn.md)

**位置：**`isos/eos_dispatcher/base/<instance-id>` · **类型族：**`isos.eos_dispatcher.base@v0.2`。注册类型是可复用定义；部署时选择实例 ID 和非空实例说明。

**状态：** 已注册用于图格式 `0.2.0`。

## 用途与边界

节点命名一个带版本的底盘操作，并通过一条图边连接兼容底盘设备；它不拥有适配器，也不能绕过准入。

计划中的 IsosUI EOS 面板把这个**活动实例**显示为一项可选原语控制。面板显示类型与实例描述、唯一目标、声明的运动模式及 EOS 实时健康；调用经现有 `primitive_typed` bridge，并显示关联结果。图连线是绑定，不是可执行调用。

## 端口与设置

设置：`primitive_name`、`primitive_version`、`movement_type`。输出：`command: BaseMovement`；无输入。设备的 `inputs.command` 边确定目标。所属接口页定义精确字段类型、单位、默认值、校验和示例。注册类型须有非空 `NodeType.description`。

## 生命周期与算法

从显式边解析唯一兼容设备，将名称、版本和模式与准入原语目录比对，再经 Dispatcher 发送类型化命令头。执行前拒绝歧义。 节点状态、拒绝原因和关联 ID 写入共享部署日志。
