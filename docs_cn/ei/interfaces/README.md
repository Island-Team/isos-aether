# EI 外部接口

[English](../../../docs/ei/interfaces/README.md) · [EI 架构](../design/architecture.md)

本目录描述其他所有者和客户端可使用的边界。EI 的实现与算法位于[设计](../design/architecture.md)。

| 契约 | 消费者与边界 |
| --- | --- |
| [配置](setup.md) | IsOS 的 `ei-config` 一次性及所有者套接字交换。 |
| [运行时](runtime.md) | 面向操作员的 EI 运行时 WebSocket API。 |
| [节点](node.md) | IsOS 图节点引用与 EI–EOS 路径所有权。 |
| [EOS 操作](eos.md) | EI–EOS 发现、操作、取消及兼容性。 |
| [事件](events.md)、[安全复位](safety-reset.md)、[视觉证据](vision-evidence.md)、[读取特性](reader-features.md) | 外部事件和专题所有者交换。 |
