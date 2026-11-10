# EI 本地诊断

[English](../../../docs/ei/interfaces/events.md) · [日志设计](../design/runtime/lib.md) · [首次使用](../start/quick-start_cn.md)

EI 向 `data/ei/log/<deployment>/ei.jsonl` 逐行追加 JSON 对象（独立运行使用 `local`）。`EI_LOG_DIR` 可选择绝对路径下的私有目录。IsOS 在本地读取此文件来填充日志面板。EI 所有者接口仅传递健康与生命周期回复，不提供日志传输路由。

每条记录包含 `timestamp`（RFC 3339 UTC）、`level`（`debug`、`info`、`warn` 或 `error`）、`source` 与 `message`。节点来源格式为 `<core>.<layer>.<group>.<node>@aether-v0.2.1`；系统代码使用点分文件路径，例如 `ei.owner_service.py@aether-v0.2.1`。可选字段包括 `event_code`、`run_id`、`correlation_id` 及有界且已清理的 `fields`。JSONL 记录自身不含显示前缀。EI 在写入前清理秘密、URL、提示、媒体及原始负载。

所有者在同一文件中记录工作进程启动、意外退出及 Stop。日志记录只是诊断证据，不是命令，也不能证明物理执行成功。日志写入失败不会改变所有者生命周期回复。
