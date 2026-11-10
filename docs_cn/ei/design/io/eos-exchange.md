# EOS 南向接口

[English](../../../../docs/ei/design/io/eos-exchange.md) · [机身](../../../../docs/ei/design/io/body.md) · [世界](../../../../docs/ei/design/io/world.md) · [Primitive](../../../../docs/ei/design/io/primitives.md) · [Skill](../../../../docs/ei/design/io/skills.md) · [EOS 2 线协议示例](../../../../docs/ei/interfaces/eos.md)


EOS 拥有线协议契约，EI 使用其生成的绑定。准确流程见 [EI–EOS 契约边界](../../../island/architecture_cn.md#跨所有者契约)和 [operation 示例](../../interfaces/eos.md)。


## 数据流

\`\`\`mermaid
flowchart LR
  discover["Gateway discovery"] --> generation["Atomic robot/device generation"]
  capabilities["CapabilityService catalog"] --> binding["Exact ready capability join"]
  generation --> binding
  resources["Read/watch resources"] --> typed["Canonical schema validation"]
  binding --> registry["Executable registry"]
  typed --> perception["World + detection + ASR admission"]
  safety --> operation["SubmitOperation with operation_id"]
  operation --> terminal["WatchOperation terminal acknowledgement"]
  interrupt["Timeout or cancellation"] --> cancel["CancelOperation by operation_id"]
  cancel --> terminal
  terminal --> mission["Mission outcome + structured log"]
\`\`\`

每个可执行 binding 必须健康、匹配安装的精确版本 schema，并通过本地 policy。异常 binding 会保留用于诊断，但不可执行。刷新会原子替换完整 generation。

## 接口分组

- [机身](../../../../docs/ei/design/io/body.md)介绍 robot/device identity、状态、health、binding scope、rediscovery 和 status。
- [世界](../../../../docs/ei/design/io/world.md)介绍 typed world state、detection、speech、resource watch 和 camera capture。
- [Primitive](../../../../docs/ei/design/io/primitives.md)介绍 action version、持久 operation identity、终态、显式取消和不确定结果。
- [Skill](../../../../docs/ei/design/io/skills.md)介绍专用 skill service 提供的 descriptor 与异步执行。

