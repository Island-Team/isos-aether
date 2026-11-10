# 三核心按钮运行时

[English](../../../../docs/isos/design/runtime/owner-health.md) · [节点契约](../../interfaces/node/README_cn.md)

图格式 `0.2.0` 有三个平级核心。IsOS 管理保存修订、Apply、启动以及带运行栅栏的按钮入口。EI 和 EOS 通过私有 Unix 套接字提供归属方服务并监管自己的工作进程。EI 将类型化激活事件传过输入、认知和安全阶段。EOS 经接口接收事件，由演示技能映射为固定六关节增量，经调度器准入后更新模拟机械臂。同一运行中的重复事件 ID 保持幂等；Stop 阻止后续运动。

IsOS 退出后归属方服务仍可运行。IsOS 在私有运行目录持久化运行 ID 和图修订，管理器重启后重新连接两个服务并分别报告工作进程状态。显式部署 Stop 向两个归属方发出停止请求，确认后才删除运行身份。

若后续归属方进程不再识别该运行，保存的收据会使 IsOS 保持降级并阻止监控读取。[依赖停止契约](../../interfaces/core/dependent-stop_cn.md)定义了带证明的 Stop 和本地紧急关闭。

## 紧急关闭

CLI `isos stop` 命令在 IsOS 服务未运行、保存的运行 ID 不匹配时仍可工作。它向属于此工作区的进程发送 SIGTERM，必要时升级为 SIGKILL。确认没有匹配进程后，以紧急审计记录退役过期的归属方运行收据。进程成功退出并不证明执行器处于物理安全状态。命令警告此操作可能中断运行中的硬件，要求操作员核实硬件状态。

本地操作系统权限控制可发送信号的进程。`config/isos/emergency-stop.toml` 或其 `ISOS_STOP_CODE_SHA256` 环境变量覆盖值可增加口令校验，通过 `--code-stdin` 从标准输入提供；口令不会进入进程参数。在源码工作区中，`isos stop` 调用 `kill.sh` 发送进程信号；脚本还支持 dry-run 预览。[契约](../../interfaces/core/dependent-stop_cn.md#本地紧急-stop)定义了命令范围和警告。

## 回放证据

2026-11-09 的回放保存在被忽略的 `data/test/20261109-arm-button-integration/`。它执行 `isos config init`、验证、Apply、Run、两次不同激活、重复重试、旧修订拒绝、统一日志以及已确认 Stop。模拟机械臂位置依次为 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]` 和 `[0.1,-0.08,0.06,-0.04,0.02,-0.02]`。重复重试保留第一次位置；旧修订返回 409。Rust 工作区测试、UI 检查与构建、格式检查、Clippy 和双语链接审计均通过。复现时使用全新的 `ISOS_HOME`、已构建的 `target/debug` 归属方二进制、`ISOS_AETHER_BIN_DIR=$PWD/target/debug`、`ISOS_EI_BIN_DIR=$PWD/ei/bin` 和 `EI_PYTHON=$PWD/ei/.venv/bin/python`。

2026-11-10 的紧急 Stop 回放保存在被忽略的 `data/test/emergency-stop-20261110/`。`cargo fmt --all -- --check`、`cargo test -p aether-runtime --bin aether-profile`、IsOS 恢复聚焦测试、`ei/.venv/bin/python -m pytest ei/tests/ei/test_owner_service.py -q`、Ruff 和 `python3 scripts/check_doc_links.py --all` 均通过。`target/debug/isos stop` 在没有管理器时清除旧归属方；重新 Apply 和 Run 后 EOS 就绪、EI 运行，`/api/isos/v1/nodes/monitor` 返回包含观察源的 HTTP 200；紧急 Stop 也清除了正在运行的模拟部署。所选测试环境中的 IsOS 广泛测试有两个目录队列／准入用例失败；对 IsOS 测试目标使用 `-D warnings` 的 Clippy 报告未使用代码警告。完整输出保存在证据目录。
