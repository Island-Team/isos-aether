# 当前运行日志游标

[English](../../../../docs/isos/interfaces/manager/log-cursors.md) · [本地核心日志](../../design/runtime/logging_cn.md)

日志 API 读取三个本地核心 JSONL 文件，并按时间合并被接受的记录。后续游标对客户端不透明，并由 IsOS 签名。它绑定来源过滤、运行身份、文件身份和偏移；伪造令牌会被拒绝。运行变化或文件替换会产生缺口诊断，并从安全边界继续。日志只是观察结果，不是工作进程健康的证明；健康状态应读取部署状态。

| 字段 | 类型 / 单位 | 规则 |
| --- | --- | --- |
| `cursor` | 不透明字符串 / 后续位置 | 首次读取可省略；绑定过滤条件与当前运行。 |
| `limit` | 整数 / 条记录 | 每页 1–200 条。 |
| `source` | 字符串 / 核心 | `all`、`isos`、`eos` 或 `ei`。 |
| `level` | 字符串 / 严重程度 | `trace`、`debug`、`info`、`warn` 或 `error`。 |
| `node_path` | 字符串 / 图路径 | 可选精确匹配。 |
| `correlation_id` | 字符串 / 操作 | 可选精确匹配。 |

IsOS 读取 `data/isos/log/deployments/<deployment>/isos.jsonl`、`data/eos/log/<deployment>/eos.jsonl` 和 `data/ei/log/<deployment>/ei.jsonl`。它只读取有界完整行，并拒绝符号链接、畸形 JSON、过大记录，以及来源核心与文件不符的记录。EOS 和 EI 从不通过 owner IPC 传送日志。启动器和安装器在自己的 IsOS 文件中使用相同 JSONL 信封；参阅[日志指南](../../design/runtime/logging_cn.md)。
