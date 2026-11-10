# 本地核心日志

[English](../../../../docs/isos/design/runtime/logging.md) · [核心日志契约](../../interfaces/core/events_cn.md) · [快速开始](../../start/quick-start_cn.md)

先执行 `./isos.sh status --json` 确认部署和运行 ID，再用 `./isos.sh logs all --lines 80` 或工作台日志面板查找第一条就绪或失败记录。IsOS 读取本地核心文件，按时间合并记录供显示。EOS 和 EI 不通过 owner 接口传送日志；这些接口只承载 setup 和进程健康状态。

## 每核心一个文件

部署 `default` 的当前文件为 `data/isos/log/deployments/default/isos.jsonl`、`data/eos/log/default/eos.jsonl` 和 `data/ei/log/default/ei.jsonl`。独立运行的 owner 用 `local` 代替部署名。开发模式启动器写入 `data/isos/log/launch/dev/launcher.jsonl`，产品模式写入 `launch/<session-id>/launcher.jsonl`。安装记录和捕获的构建输出共用 `data/isos/log/install/<session-id>/install.jsonl`。

所有文件均为 UTF-8 JSON Lines，并具有相同的必填字段 `timestamp`、`level`、`source` 和 `message`。节点来源采用 `<core>.<layer>.<group>.<node>@aether-v0.2.1`；系统组件可使用 `isos.runtime.manager.diagnostic@aether-v0.2.1` 等身份。可选的 `event_code`、`run_id`、`correlation_id`、`node_path` 和有界 `fields` 保留结构化上下文。来源是逻辑身份，不是机器上的绝对路径。需要时才生成易读视图；磁盘上不再并行写入 `.log` 文本或合并事件文件。

每个核心只写自己的文件。IsOS 从三个本地文件读取有界记录，检查记录形状和来源前缀，并向 UI 返回合并页面。磁盘文件保持分开，因此即使 IsOS 不可用，仍可检查故障 owner。Run 和 Stop 的健康状态以部署状态为准。

## 诊断与清理

用 `./isos.sh logs all --level warn --lines 100` 查看可操作故障，`--format jsonl` 查看原始记录，`--current` 使用当前运行游标。HTTP 错误的关联 ID 可对应 IsOS JSONL 记录。读取器拒绝畸形或过大的记录并标记缺口，不将其当成可信事件。

旧的 `.log`、`events.jsonl`、`events/<source>.jsonl` 和构建输出文本文件属于已退役布局；当前日志面板不读取它们。应在新写入器和读取器运行后清理，避免删除仍被进程打开的文件。

验证命令：`bash -n isos.sh install.sh`、`cargo test -p isos`、`cargo fmt --all -- --check`、`cargo clippy -p isos --bin isos -- -D warnings`、UI 构建与测试，以及 setup → Apply → Run → logs → Stop 重放。完整输出保存在忽略的 `data/test/<run-id>/` 下。

2026-11-09 的重放使用 `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace`，使目录测试解析到重新构建的 EOS profile。随后对 `config/deployments/default.nodes.json` 执行 Apply，启动 EOS 和 EI，通过当前运行的 API 读取三个核心文件中的记录，确认 Stop 回执，并启动新的一轮运行。输出和旧文件清单保存在 `data/test/core-local-jsonl-20261109/` 下。
