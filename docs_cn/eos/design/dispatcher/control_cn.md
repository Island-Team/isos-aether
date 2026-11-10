# 共享控制调度器节点

[English](../../../../docs/eos/design/dispatcher/control.md) · [EOS 架构](../architecture_cn.md) · [control](../../interfaces/control_cn.md) · [primitive-bindings](../../interfaces/primitive-bindings_cn.md)

**位置：**`isos/eos_dispatcher/control/<instance-id>` · **类型族：**`isos.eos_dispatcher.control@v0.2`。注册类型是可复用定义；部署时选择实例 ID 和非空实例说明。

**状态：** 已注册用于图格式 `0.2.0`。

## 用途与边界

唯一的执行器准入方在后端执行前检查身份、健康、新鲜度、期限、限值和安全状态。图节点表示该准入方及其类型化路径，不创建第二个调度器。

## 端口与设置

输入：`request: ControlRequest`。输出：`base_command: BaseMovement`、`manipulator_command: ManipulatorMovement`、`status: Status`。所属接口页定义精确字段类型、单位、默认值、校验和示例。注册类型须有非空 `NodeType.description`。

## 生命周期与算法

规范化请求，拒绝无效或过期任务，保留命令身份，记录准入，只执行一次，然后观测或停止。路由输入上的优先级只影响交付顺序，不绕过这些检查。 节点状态、拒绝原因和关联 ID 写入共享部署日志。
