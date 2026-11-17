# Edge contract fixtures / 连线契约夹具

The JSON fixtures exercise EOS validation of IsOS-owned format `0.2.0` graphs with one `isos` core. Tests clone them in memory to check typed inputs, transport, cardinality, QoS, command admission, and cycles. The graph fixtures do not grant a runtime command route.

JSON 夹具用于验证 IsOS 所拥有的 `0.2.0` 图及唯一 `isos` 核心。测试在内存中复制夹具，检查类型化输入、传输、基数、QoS、命令准入和环路。图夹具不授予运行时命令路由。

| Fixture | Contract / 契约 |
| --- | --- |
| `valid_evidence.json` | Camera and primitive declarations used for typed graph validation. / 用于类型化图验证的相机和原语声明。 |
| `valid_bindings.json` | Pinned base and manipulator each have one matching primitive binding under Dispatcher admission. / 固定的底盘和机械臂各有一个与 Dispatcher 准入一致的原语绑定。 |

Mutation tests report precise graph pointers for invalid data types, transports, cardinality, QoS, command sources, and cycles. / 变异测试对无效的数据类型、传输、基数、QoS、命令来源和环路报告精确的图指针。
