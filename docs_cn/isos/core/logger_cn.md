# 本地核心日志

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS 核心](README_cn.md)
  - [IsOS 架构](architecture_cn.md)
  - [IsOS 命令行](cli_cn.md)
  - [IsOS 配置](config_cn.md)
  - [IsOS 软件包](pkg_cn.md)
  - [IsOS 路线图](roadmap_cn.md)
  - [IsOS 服务端](server_cn.md)
  - [IslandOS 服务](service_cn.md)
- 下级目录：
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/isos/core/logger.md) · [核心日志契约](../interfaces/core_cn.md#core_events) · [快速开始](../start/quick-start_cn.md)

先执行 `./isos.sh status --json` 确认部署和运行 ID，再用 `./isos.sh logs all --lines 80` 或工作台日志面板查找第一条就绪或失败记录。IsOS 将自身记录与持久所有者日志池中的 EOS、EI 诊断合并。EOS 和 EI 通过私有所有者套接字提供这些诊断，因此后来打开的 UI 也可读取所有者启动和运行历史。

## 每核心一份 JSONL 和一份易读日志

部署 `default` 的生产者文件为 `data/isos/log/deployments/default/isos.jsonl`、`data/eos/log/default/eos.jsonl` 和 `data/ei/log/default/ei.jsonl`。IsOS 还将选定的所有者记录存入 `data/isos/log/deployments/default/owner-pool/eos.jsonl` 和 `ei.jsonl`，并为每个核心保存来源游标。独立运行的 owner 用 `local` 代替部署名。开发模式启动器写入 `data/isos/log/launch/dev/launcher.jsonl`，产品模式写入 `launch/<session-id>/launcher.jsonl`。安装记录和捕获的构建输出共用 `data/isos/log/install/<session-id>/install.jsonl`。

每个核心还会在 JSONL 旁写入 `isos.log`、`eos.log` 或 `ei.log`。启动器、安装器和受监督会话的 JSONL 文件也有同名 `.log` 文件。文本行采用 `IsOS-v0.2.3@<source>: [LEVEL][UTC 时间戳]<消息>`；消息中的换行会转义。来源是逻辑组件身份，不是机器绝对路径。浏览器控制台缓冲区保存在内存中，可在日志面板查看；浏览器不能直接写入本地文件。

JSONL 文件仍具有必填字段 `timestamp`、`level`、`source` 和 `message`。节点来源采用 `<core>.<layer>.<group>.<node>@aether-v0.2.3`；系统组件可使用 `isos.runtime.manager.diagnostic@aether-v0.2.3` 等身份。可选的 `event_code`、`run_id`、`correlation_id`、`node_path` 和有界 `fields` 保留供游标和筛选使用的结构化上下文。

每个核心只写自己的生产者文件。IsOS 每 250 毫秒请求有界的所有者日志页，将 `warning` 和 `critical` 规范为 `warn` 和 `error`，验证选定记录，并将概况信息及全部警告和错误追加到分核心日志池。没有 `run_id` 的所有者记录绑定到当前 IsOS 管理器或运行范围。日志池采用配置的 IsOS 日志轮换上限（默认每文件 8 MiB、五份备份），并跨 UI 重连保留。IsOS 从自身 JSONL 和日志池返回合并页面。生产者文件在磁盘上保持分开，因此即使 IsOS 不可用，仍可检查故障 owner。Run 和 Stop 的健康状态以部署状态为准。

开发管理器开始新会话时，只有前一运行不存在或已确认停止，才会执行合格的历史重置，仅替换 IsOS 会话及启动器诊断。EOS、EI 生产者文件、IsOS 所有者日志池及其来源字节游标继续保留。因此重启的 IsOS 可继续摄入所有者日志，不会抹掉重新附着的 UI 所需历史。前一次 Stop 结果不确定时，不执行重置。

空闲工作台读取 `manager_session` 范围：IsOS 日志行属于当前管理器会话，而较早会话中保留的 EOS、EI 所有者日志池记录仍可见。Run 后，日志视图切换到 `deployment_run`，三个来源都按所选运行 ID 约束。这样既能显示启动与重连历史，也不会把之前运行的诊断混入当前运行。

Run 期间，管理器串行处理生命周期命令。HTTP 进程与监控读取会等待命令预算，再判断管理器不可用；这样 owner 启动时不会产生错误的 503 记录。浏览器关闭待完成的状态读取后，命令完成时管理器可能收到断管；这种只读请求的对端关闭不是管理器故障，不产生 `manager_client_failed` 警告，生命周期命令断开仍保留警告。状态与监控客户端各自最多保留一个待完成的轮询；EI 和 EOS 工作台的相同连接错误在恢复前只生成一条浏览器消息。真实的管理器或 owner 故障仍保留诊断及请求记录。

2026-11-13 的隔离混合图重放使用短私有主目录及 16130/16131 端口。约 30 秒的启动后，八个并发状态轮询及八个机械臂监控轮询全部返回 200，没有 503。按钮激活与机械臂观测、RoboCasa Vision 帧、日志读取及已确认的 Stop 均通过。IsOS 没有错误，EOS 和 EI 没有警告或错误。IsOS 唯一一组警告是 Stop 在读取期间退役运行身份时的监控 409；该过期围栏拒绝符合预期。完整命令、响应、核心日志及 Stop 证明位于忽略目录 `data/test/20261113-status-monitor-availability/`。该目录记录的可复现命令通过 Rust 格式检查、严格 Clippy、IsOS 测试、UI 类型检查／测试／构建及双语文档审计。

## 诊断与清理

用 `./isos.sh logs all --level warn --lines 100` 查看可操作故障，`--format jsonl` 查看原始记录，`--current` 使用当前运行游标。HTTP 错误的关联 ID 可对应 IsOS JSONL 记录。读取器拒绝畸形或过大的记录并标记缺口，不将其当成可信事件。

旧的 `events.jsonl`、`events/<source>.jsonl` 和构建输出文本文件属于已退役布局。日志面板继续读取 JSONL；直接检查可使用新的 `.log` 配套文件。应在写入器和读取器运行后清理旧文件，避免删除仍被进程打开的文件。

验证命令：`bash -n isos.sh install.sh`、`cargo test -p isos`、`cargo fmt --all -- --check`、`cargo clippy -p isos --bin isos -- -D warnings`、UI 构建与测试，以及 setup → Apply → Run → logs → Stop 重放。完整输出保存在忽略的 `data/test/<run-id>/` 下。

2026-11-09 的重放使用 `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace`，使目录测试解析到重新构建的 EOS profile。随后对 `config/deployments/default.nodes.json` 执行 Apply，启动 EOS 和 EI，通过当前运行的 API 读取三个核心文件中的记录，确认 Stop 回执，并启动新的一轮运行。输出和旧文件清单保存在 `data/test/core-local-jsonl-20261109/` 下。

2026-11-15 的易读日志改动已通过 `cargo fmt --all -- --check`、针对 `isos`、`aether-runtime`、`aether-common` 的严格 Clippy、构建 `aether-profile` 后的 `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos`、`cargo test -p aether-common -p aether-runtime`、针对性的 EI pytest、Ruff、Pyright、两项文档审计、Shell 语法检查以及包含换行的启动器日志样本。完整命令输出保存在忽略的 `data/test/20261115-unified-readable-logs/` 下。
