<a id="manager"></a>
<a id="manager--归属方实现说明"></a>
# 归属方实现说明

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [节点、软件包、核心与 IsosUI 接口](README_cn.md)
  - [共享契约目录](contract-catalog_cn.md)
  - [核心归属方契约](core_cn.md)
- 下级目录：
  - [api/](api/README_cn.md)
  - [node/](node/README_cn.md)
  - [package/](package/README_cn.md)
  - [server/](server/README_cn.md)

</details>
<!-- doc-navigation:end -->

本页面向 IsOS 本地管理器及所配置服务的实现者。这些是内部控制面；软件包和核心发布者应使用[外部核心契约](core_cn.md)进行交接和停止。

- [管理器](manager_cn.md#manager_protocol)定义本地操作 IPC 与检查修订的变更封套。
- [服务](../core/service_cn.md)定义选择本地归属方与启动子进程的管理器 TOML 条目。
- [日志游标](manager_cn.md#manager_log_cursors)定义用于诊断的运行范围日志分页及排序。

[English](../../../docs/isos/interfaces/manager.md)

<a id="manager_log_cursors"></a>
<a id="manager_log_cursors--当前运行日志游标"></a>
## 当前运行日志游标

[English](../../../docs/isos/interfaces/manager.md#manager_log_cursors) · [本地核心日志](../core/logger_cn.md)

日志 API 读取 IsOS 的管理器 JSONL 及有界的 EOS、EI 所有者日志池，并按时间合并被接受的记录。EOS 和 EI 通过私有套接字提供筛选后的所有者日志页；IsOS 保存选定记录后供 UI 读取。后续游标对客户端不透明，并由 IsOS 签名。它绑定来源过滤、运行身份、文件身份和偏移；伪造令牌会被拒绝。运行变化或文件替换会产生缺口诊断，并从安全边界继续。日志只是观察结果，不是工作进程健康的证明；健康状态应读取部署状态。

| 字段 | 类型 / 单位 | 规则 |
| --- | --- | --- |
| `cursor` | 不透明字符串 / 后续位置 | 首次读取可省略；绑定过滤条件与当前运行。 |
| `limit` | 整数 / 条记录 | 每页 1–200 条。 |
| `source` | 字符串 / 核心 | `all`、`isos`、`eos` 或 `ei`。 |
| `level` | 字符串 / 严重程度 | `trace`、`debug`、`info`、`warn` 或 `error`。 |
| `node_path` | 字符串 / 图路径 | 可选精确匹配。 |
| `correlation_id` | 字符串 / 操作 | 可选精确匹配。 |

IsOS 读取 `data/isos/log/deployments/<deployment>/isos.jsonl`，以及同一部署目录下的 `owner-pool/eos.jsonl` 和 `owner-pool/ei.jsonl`。所有者日志池独立于浏览器会话持久保存，让新连接的 UI 读取较早的启动、Run、警告和错误记录。IsOS 只读取有界完整行，并拒绝符号链接、畸形 JSON、过大记录，以及来源核心与日志池不符的记录。EOS 和 EI 保留各自的本地 JSONL 文件，供所有者重新附着后重放。启动器和安装器在自己的 IsOS 文件中使用相同 JSONL 信封；参阅[日志指南](../core/logger_cn.md)。

开发管理器可在确认 Stop 后替换自己的上一会话诊断，但保留两个所有者日志池、`owner-pool/{eos,ei}.cursor` 来源偏移和所有者本地 JSONL，使重连继续读取而不丢失 EOS 或 EI 历史。对于被替换的管理器文件，UI 仍将旧 IsOS 游标视为已轮转并请求新页面。

在空闲的 `manager_session` 日志视图中，即使保留的 EOS、EI 日志池记录的 `run_id` 与当前管理器会话 ID 不同，IsOS 仍将它们返回。IsOS 自己的记录继续受当前管理器会话约束。选定部署运行后，`deployment_run` 视图只包含三个来源中精确匹配该运行 ID 的记录。来源或级别筛选均在各自范围内生效；旧所有者记录或可见日志行都不授予运行控制权。

<a id="manager_protocol"></a>
<a id="manager_protocol--管理器-ipc-v1"></a>
## 管理器 IPC v1

[English](../../../docs/isos/interfaces/manager.md#manager_protocol) · [边界索引](manager_cn.md) · [运行时 HTTP](api/http_cn.md#http_runtime)

<a id="manager_protocol--概览"></a>
### 概览

IsOS CLI 和回环 HTTP 服务器通过 owner 专用 Unix socket 调用 IsOS 管理器。此 wire 为 `isos.manager@v1`，独立于 HTTP schema、图版本和 owner setup。管理器串行化部署写操作。

启动和重启先验证已保存的 `0.2.1` 图及其精确字节版本，再准备启动计划。部署计划始终使用受管理的准入流程，包括控制授权和 owner 就绪检查。

<a id="manager_protocol--身份验证与传输"></a>
### 身份验证与传输

Socket 检查同 UID 对端身份。一个请求是一行有大小限制和截止时间的 JSON 对象；过大、畸形、超时或未知操作在分派前失败。私有 socket 不得投射为浏览器端点。

<a id="manager_protocol--接口"></a>
### 接口

请求包含 `schema_version:"0.1.0"`、snake-case `op` 和该操作的专属 `payload`。类型化载荷中的额外字段会被拒绝。停止请求示例：

```json
{
  "schema_version": "0.1.0",
  "op": "deployment_stop",
  "payload": {
    "deployment": "default",
    "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  }
}
```

响应包含 `schema_version:"0.1.0"`、`ok`、操作特定 `data`，以及可选 `error`（稳定 `code`、安全 `message` 和 `correlation_id`）。错误示例：

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "revision_conflict",
    "message": "request conflicts with current state",
    "correlation_id": "req-42"
  }
}
```

| 字段 | 类型 / 单位 | 用途与验证 |
| --- | --- | --- |
| `schema_version` | 字符串 / 版本 | 必须为 `0.1.0`。 |
| `op` | 闭合 snake-case 字符串 / 无 | 选择一个操作和载荷类型；未知值拒绝。 |
| `payload.deployment` | slug 字符串 / 无 | 选择管理器拥有的部署；部署和追加日志操作必需。 |
| `payload.revision` | 可选字符串 / SHA-256 | 部署写操作的精确已保存字节栅栏；省略表示接受当前版本。 |
| `payload.service_id` | 字符串 / 无 | 仅服务目标和服务日志操作必需。 |
| `payload.cursor` | 可选不透明字符串 / 无 | 服务日志续读；客户端不解码。 |
| `payload.limit` | 可选无符号整数 / 条记录 | 限制服务日志读取。 |


<a id="manager_protocol--操作"></a>
### 操作

无载荷操作：`manager_identity`、`manager_shutdown`、`services_list`、`services_validate`、`services_status`。部署操作：`deployment_start`、`deployment_restart`、`deployment_stop` 使用 `deployment` 和可选 `revision`；`deployment_status`、`deployment_active_packages` 仅使用 `deployment`。服务写操作 `services_start`、`services_stop`、`services_restart` 使用 `service_id`；`services_logs` 增加可选 `cursor` 和 `limit`。在 `deployment_stop` 中添加 `service_id` 等跨操作字段无效。停止只在 owner 证明后确认；见[依赖停止](core_cn.md#core_dependent_stop)。
