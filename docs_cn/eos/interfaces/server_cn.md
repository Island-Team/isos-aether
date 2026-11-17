<a id="external_http_cn"></a>
<a id="external_http_cn--eos-回环-http-bridge"></a>
# EOS 回环 HTTP bridge

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 下级目录：
  - [node/](node/README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/eos/interfaces/server.md#external_http) · [控制准入](../../isos/interfaces/server/eos_cn.md#control_cn) · [视觉](node/interface/egress_cn.md#interface_egress_cn--vision)

`aether-server` 是面向客户端控制和设置目录的 Rust 进程。它使用两个回环监听器：`--listen` 提供运行时控制，`--catalog-listen` 提供只读设置路由，`--profile` 选择图快照。目录监听器完成校验和绑定后才报告运行时就绪。仅设置模式只传入 `--profile` 与 `--catalog-listen`，不需要运行时 socket。保存的图仍声明 `bridge` 与 `config` 进程节点；IsOS 在启动时把两者的设置合并为一个 `bridge` 子进程。它绑定回环地址，校验 Host 与 Origin，限制请求体和私有 IPC，并只将已准入的公开操作转发到选定的 `aether-runtime` Unix 套接字。它不直接释放执行器。IsOS 在已应用部署中选择并监督 bridge。

<a id="external_http_cn--控制路由"></a>
## 控制路由

`POST /api/eos/v2/control` 接收有界 JSON，用于已准入的 `primitive_typed`、授权结果查询及只读状态、观察与健康。[控制契约](../../isos/interfaces/server/eos_cn.md#control_cn)通过易读的 JSON 请求、成功和错误示例说明字段、授权、期限、错误与 Dispatcher 预留。浏览器调用方不能提供私有所有者授权，也不能调用原始调度或关闭操作。

<a id="external_http_cn--故障与-stop"></a>
## 故障与 Stop

无效 Host 或 Origin、畸形 JSON、过大载荷、缺少授权、过期修订、运行时不可用及所有者证明失败，都产生有界错误且不泄露私有路径或令牌。IsOS 负责部署 Stop。EOS 在终止其直接运行的选定服务子进程前关闭命令准入并停止已准入执行器。
