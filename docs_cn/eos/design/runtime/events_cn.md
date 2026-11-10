# EOS 本地日志

[English](../../../../docs/eos/design/runtime/events.md) · [日志契约](../../interfaces/events_cn.md) · [故障排查](troubleshooting_cn.md)

EOS 所有者和机械臂工作进程追加到同一个本地 JSONL 文件：受管部署使用 `data/eos/log/<deployment>/eos.jsonl`，独立运行使用 `data/eos/log/local/eos.jsonl`。IsOS 读取文件更新日志面板。所有者 API 返回生命周期和激活结果；日志记录留在本地。

每条记录包含 UTC RFC 3339 时间戳、小写级别、来源及消息。属于一次运行的记录还包含运行 ID。模拟机械臂来源是 `eos.hardware.mock.arm@aether-v0.2.1`；工作进程系统事件使用 `eos.aether_runtime.worker@aether-v0.2.1`。所有者观察到工作进程丢失时只记录一次，并记录失败的 Run 或 Stop 结果。命令的终态由激活回复证明，不能只依赖日志。

为调用方保留的旧 bridge 和 config-server 程序通过 `LocalCoreLog` 写入同一个文件中的有界记录。当前演示图不启动它们。运行 `cargo test -p aether-runtime --all-targets` 验证本地日志及所有者和工作进程生命周期；完整输出保存在忽略的 `data/test/<run-id>/`。
