# 外部进程接口

[接口指南](../README_cn.md) · [English](../../../../docs/eos/interfaces/external/README.md)

这些接口定义 EOS 与外部进程的通信。HTTP 与 MuJoCo 契约见[服务进程](../../design/runtime/services_cn.md)和 [MuJoCo](../mujoco_cn.md)。传输协议不构成方位命名空间。

- [HTTP/SSE 适配器](http_cn.md)：WebUI 入口；适配器调用 EOS gRPC 服务。
- [模型提供方](models_cn.md)：EOS 通过 HTTP、WebSocket 或配置的 gRPC 发起能力请求；私有 schema 位于 `share/eos/contracts/eos/external/models/schemas/`。
- [模拟器进程](simulator_cn.md)：模拟器边界的进程侧传输与生命周期。

技能 mailbox 属于技能运行时，归在原生技能编写文档中。
