# 本地核心日志契约

[English](../../../../docs/isos/interfaces/core/events.md) · [运行时日志](../../design/runtime/logging_cn.md)

每个核心写入一个私有 UTF-8 JSONL 文件。IsOS 在本地读取这些文件供日志 API 和工作台使用；EI 和 EOS 不通过 setup、Run、Status 或 Stop 接口传送日志事件。IsOS 不把日志记录当作进程健康或执行器状态的证明；健康状态仍以部署状态的 owner 回复为准。

| 字段 | 类型 | 规则 |
| --- | --- | --- |
| `timestamp` | RFC3339 UTC 字符串 | 必填；生产者时间，以 `Z` 结尾。 |
| `level` | 字符串 | 必填：`trace`、`debug`、`info`、`warn` 或 `error`。 |
| `source` | 字符串 | 必填逻辑身份：图节点采用 `<core>.<layer>.<group>.<node>@aether-v0.2.1`；系统组件采用带核心前缀的名称，如 `isos.runtime.manager.diagnostic@aether-v0.2.1`。 |
| `message` | 字符串 | 必填的有界诊断文本；不得包含秘密或原始硬件载荷。 |
| `event_code` | 字符串 | 可选的稳定机器代码。 |
| `run_id`、`correlation_id` | 字符串 | 可选操作身份，可用于关联状态及 HTTP 错误。 |
| `node_path` | 字符串 | 事件属于节点时可选的规范图路径。 |
| `fields` | 对象 | 可选的有界结构化细节。 |

示例记录：

```json
{
  "timestamp": "2026-11-09T09:24:22.340Z",
  "level": "info",
  "source": "eos.hardware.mock.adapter@aether-v0.2.1",
  "message": "Mock hardware is ready",
  "event_code": "mock_hardware_ready",
  "run_id": "r-example"
}
```

当前路径为 `data/isos/log/deployments/<deployment>/isos.jsonl`、`data/eos/log/<deployment>/eos.jsonl` 和 `data/ei/log/<deployment>/ei.jsonl`。IsOS 读取有界的完整行，核对来源核心与文件，并按时间合并被接受的记录。畸形、过大或文件被替换的记录会形成缺口诊断，不会变成可信事件。本地文件仅归属方可读；回环日志 API 继续执行 Host、Origin 检查和过滤。
