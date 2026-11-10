# EOS 所有者生命周期

[English](../../../docs/eos/interfaces/owner-health.md) · [EOS 接口](README_cn.md) · [机械臂控制](arm-control_cn.md)

持久运行的 `aether-profile owner --socket <absolute-path>` 进程负责 EOS 设置及其直接子进程 `aether-runtime`。IsOS 通过私有 Unix socket 连接。IsOS 进程退出后所有者可继续运行。所有者与工作进程都在给定的绝对路径 `EOS_LOG_DIR` 下写入本地 JSONL 记录；所有者发现工作进程丢失时写入一条错误记录。

## 设置和图投影

所有者传输使用模式版本 `1`。设置请求通过 `POST /v1/identity`、`/v1/catalog`、`/v1/options`、`/v1/validate` 和 `/v1/render` 发送，包含 `core:"eos"` 及匹配的 `op`；设置响应使用模式版本 `2`。图投影格式为 `0.2.0`，只含一个 `cores.eos` 条目。目录包含[机械臂控制](arm-control_cn.md)所述的八个实时演示节点和三个载荷类型。已移除的健康封装节点与 `topology_link` 因类型未知而被拒绝。

## Run、Status 和 Stop

`POST /v1/run` 要求 `{schema_version:1,core:"eos",op:"run",run_id,revision,graph,launch}`。`run_id` 是有界 ASCII 标识符，`revision` 是 64 字符十六进制摘要。启动 SHA-256 必须与同目录的 `aether-runtime` 可执行文件一致；参数和环境对象为空。所有者启动 `aether-runtime --serve <private-socket> --run-id <id> --revision <digest> --joint-limit <rad> --source-node <path> --joint-names <csv> --sample-interval-ms <ms> --freshness-timeout-ms <ms>` 并等待就绪。运行身份冲突时拒绝。

`POST /v1/status`、`POST /v1/monitor` 和 `POST /v1/stop` 需要相同的身份字段。Monitor 不驱动执行器，仅返回最新的运行隔离观测；见[机械臂监控](arm-monitor_cn.md)。Status 和 GET health 保留工作进程实际的 `ready`、`degraded` 或 `stale` 状态；子进程丢失后 Status 报告 `mock_hardware:"failed"`。降级或过期的工作进程仍可供只读监控，而运动准入保持关闭。Stop 关闭准入、请求子进程退出、等待并回收。若无法确认退出或 socket 清理，Stop 返回 `ok:false` 并保留运行身份以供重试。所有者崩溃时，子进程收到父进程死亡信号。`GET /v1/health` 报告所有者为 `idle` 或 `ready`。超过 1 MiB 的请求、无效 HTTP 和不支持的操作均被拒绝。

## 验证

运行 `cargo test -p aether-node-profile` 和 `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`。集成测试通过真实所有者和工作进程 socket 覆盖 Run → 激活 → 重复事件 → 过时请求拒绝 → Stop。
