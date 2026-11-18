<a id="io_link_cn"></a>
<a id="io_link_cn--io--link"></a>
# Io / link

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/io</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [Io / input](input_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/ei/interfaces/node/io/link.md#io_link) · [层目录](../../../interfaces_cn.md#readme_cn--io) · [EI 节点边界](../../../../isos/interfaces/server/ei_cn.md#node_cn)

[共享节点契约](../../../../node/interfaces/README_cn.md)定义类型身份、设置、端口、路由和注册规则；EI 负责本组节点的行为。本目录说明该组节点的 EI 行为，运行职责见[组设计](../../../core/io/link.md)。

<a id="io_link_cn--gateway"></a>
<a id="gateway"></a>

<a id="io_link_cn--eiiolinkgatewayv02"></a>
## ei.io.link.gateway@v0.2

接收所有者注入的资源、健康和操作结果记录。直接调用 `GatewayStage` 时，输入必须是对象，包含 `owner:"eos"`、距当前时间不超过五分钟且未来不超过一秒的带时区 `source_time`。以选定的 `run_id` 构造 `GatewayStage` 时，输入必须与之匹配；未提供时不检查运行身份。整个 JSON 记录上限为 65,536 字节。有效记录在对应类型端口原样输出；所有者、运行、时间、大小错误或停止状态均被拒绝。读取不会触发 EOS 操作。图中没有为这些独立记录新增桥接路由。
