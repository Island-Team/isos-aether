# EOS 文档

<!-- doc-navigation:start -->
<details>
<summary>目录：eos</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EOS interfaces](interfaces_cn.md)
  - [EOS interfaces](models_cn.md)
- 下级目录：
  - [core/](core/README_cn.md)
  - [interfaces/](interfaces/)

</details>
<!-- doc-navigation:end -->

EOS 对所有获准适配器提供同一硬件控制边界。选定的软件包和所属核心目录提供设备实现细节；Dispatcher 准入、类型化观测、健康和 Stop 使用相同的所属核心契约。

[English](../../docs/eos/README.md) · [文档索引](../README_cn.md) · [约定](../conventions.v2.0.0_cn.md)


- [入门](../isos/start/quick-start_cn.md)：构建、配置、启动、检查、停止和排障。
- [服务端 HTTP](interfaces/server_cn.md): 回环 HTTP 桥接与控制路由。
- [节点分组](interfaces/node/README_cn.md)：已注册的 `<layer>/<group>` 节点契约。
- [接口](interfaces_cn.md#readme_cn)：外部消费的载荷、控制、适配器和诊断契约。
- [架构](core/architecture_cn.md)：所有权与生命周期总览，并链接运行时和节点族设计。
