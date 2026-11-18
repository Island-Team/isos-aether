# IsOS 节点接口

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/node</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 下级目录：
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/isos/interfaces/node/README.md) · [共享节点契约](../../../node/interfaces/README_cn.md) · [IsOS 接口](../README_cn.md)

本目录定义 IsOS 自身的节点分组。每个实例都遵循共享的[类型、端口、实例与连线规则](../../../node/interfaces/README_cn.md)。IsOS 管理保存图准入和运行身份；这些节点提供 UI 输入或只读输出，不建立执行器路线。

| 层 / 分组 | 节点接口 |
| --- | --- |
| `ui/button` | [按钮激活](ui/button_cn.md) |
| `ui/monitor` | [机械臂观测](ui/monitor_cn.md) |
| `ui/vision` | [视觉画面](ui/vision_cn.md) |

起始图包含 `isos.ui.button.trigger@v0.2` 与 `isos.ui.monitor.arm@v0.2`。使用 `isos.ui.external.v1` 的贡献 IsOS 节点可位于任意已声明的 IsOS 层与分组，并使用私有、带运行围栏的工作进程。输入或读取前，IsOS 检查所选类型、精确软件包固定版本、当前运行及保存的修订。未注册类型化交付桥接之前，Apply 拒绝连接贡献的外部 UI 节点的图边。[客户端 API](../api/client_cn.md)定义浏览器请求与呈现。

## 贡献的外部 IsOS 节点

贡献节点声明 `implementation:{"kind":"external","interface":"isos.ui.external.v1","version":"1","lifecycle_adapter":"unix-socket-v1"}`，并在精确的已验证软件包中声明进程入口。IsOS 以直属子进程启动它，传入 `ISOS_NODE_SOCKET`、`ISOS_NODE_PATH`、`ISOS_RUN_ID` 和 `ISOS_GRAPH_REVISION`。子进程绑定私有 Unix socket。每个连接交换一条换行终止的 JSON 请求和应答；每条最多 64 KiB，socket 操作超时为两秒。

请求包含 `schema_version:1`、`op:"health"|"input"|"read"|"stop"`、精确的 `run_id`、64 位十六进制 `revision`、`node_path` 和 `payload`。健康与 Stop 使用 `payload:null`；输入为已注册输入携带 `{port,value}`；读取为已注册输出携带 `{port,value:null}`。成功应答回显身份字段、设置 `ok:true`，并在读取时把类型化值放入 `payload`。IsOS 根据所选数据类型验证输入与读取值。

例如，已注册的 `count` 输出可使用以下带围栏的请求和成功应答读取。输出值必须符合该端口选定的数据类型。

```json
{
  "schema_version": 1,
  "op": "read",
  "run_id": "run-1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "node_path": "isos/ui/community/panel",
  "payload": {"port": "count", "value": null}
}
```

```json
{
  "schema_version": 1,
  "ok": true,
  "run_id": "run-1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "node_path": "isos/ui/community/panel",
  "payload": {"count": 1}
}
```

输入要求部署正在运行；读取也允许降级状态。Stop 仅在收到带围栏的应答且子进程退出后完成。UI 端口不传输图连线；不支持的原生或外部 IsOS 实现返回 `owner_reader_unavailable`。[软件包进程契约](../package/README_cn.md#process)定义入口声明。
