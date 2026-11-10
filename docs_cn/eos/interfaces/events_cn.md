# EOS 本地 JSONL 日志契约

[English](../../../docs/eos/interfaces/events.md) · [EOS 接口](README_cn.md) · [所有者生命周期](owner-health_cn.md)

EOS 为每个部署写入一个本地 JSONL 文件：`data/eos/log/<deployment>/eos.jsonl`。独立运行时使用 `data/eos/log/local/eos.jsonl`。IsOS 向持久所有者提供绝对路径 `EOS_LOG_DIR`，并读取该文件更新日志面板。日志不通过所有者的 Run、Status、激活或 Stop 回复传递。当前图启动所有者和一个机械臂工作进程；工作进程将已准入的增量应用到模拟机械臂。

每行 UTF-8 文本是一个 JSON 对象，包含以下必需字段：

| 字段 | 值 |
| --- | --- |
| `timestamp` | UTC RFC 3339 时间戳 |
| `level` | 小写的 `debug`、`info`、`warn` 或 `error` |
| `source` | 节点使用 `eos.<层>.<组>.<节点>@aether-v0.2.1`；系统进程使用 `eos.<源文件>@aether-v0.2.1` |
| `message` | 人可读的诊断文本；换行必须经过 JSON 转义 |

属于一次运行的记录包含 `run_id`。模拟来源是 `eos.hardware.mock.arm@aether-v0.2.1`。所有者和工作进程的系统来源分别是 `eos.aether_profile.owner@aether-v0.2.1` 与 `eos.aether_runtime.worker@aether-v0.2.1`。Run 或 Stop 失败时，`message` 包含所有者错误码。健康或状态首次观察到工作进程丢失时，所有者写入一条错误。日志用于诊断；就绪、命令结果和 Stop 确认由所有者生命周期与激活回复决定。

保留的 bridge/config 程序通过 `LocalCoreLog` 将有界诊断写入同一个文件。它们不写第二个 `events.jsonl` 文件。当前图不启动这些程序。
