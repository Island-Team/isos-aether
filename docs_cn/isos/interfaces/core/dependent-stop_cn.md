# 依赖停止契约

[English](../../../../docs/isos/interfaces/core/dependent-stop.md) · [设计](../../design/README_cn.md)

当前 IsOS 从选中的所有者管道读取严格且以换行结束的终止停止收据。收据使用模式 `1`。IsOS 将收据绑定到选中 `run_id`、所有者 PID、活动修订、已准入的包进程节点及预期部分。所有者证明其后代和物理停止；IsOS 独立证明自己的直接进程组退出。证明缺失、格式错误、过期、延迟或矛盾时产生 `unknown`。

| 收据字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `schema_version`, `type` | 整数 `1`；字符串 `eos_terminal_stop` / 身份 | 当前精确接受的所有者收据格式。 | 必填。 |
| `run_id`, `active_revision` | 字符串 / 运行与 SHA-256 | 必须匹配选中运行和图修订。 | 必填。 |
| `eos_pid`, `backend_node` | 整数；图路径 / 所有者与节点 | 必须匹配已准入所有者进程及后端节点。 | 必填。 |
| `backend_pid`, `backend_pgid` | 整数或 null / 进程 ID | 绑定已准入后代进程组。 | 字段必填；值可为 null。 |
| `generation_id`, `stop_epoch` | 字符串或 null / 所有者代次 | 所有者提供的停止关联。 | 字段必填；值可为 null。 |
| `admission_closed`, `backend_group_drained`, `backend_reaped`, `sockets_removed` | 布尔值 / 停止事实 | 报告 `stopped` 前检查所有必需事实。 | 必填。 |
| `parts` | 数组 / 选中的物理结果 | 必须精确覆盖预期已准入部分。 | 必填。 |
| `outcome`, `reason_code` | 字符串及可选字符串 / 终止结果 | `unknown` 保持保守。 | 结果必填。 |

物理停止仍不确定时，Restart 不能替换该运行。

## 中断的三核心归属方运行

三核心演示在私有的 `default.owner-run.json` 收据中保存 `run_id`、`revision`、`pending`、`attempted` 和 `stopped`。普通部署 Stop 只有在 EI 和 EOS 都确认该运行停止后才删除收据。若归属方已失去该运行，IsOS 会保留收据并报告归属方不可用；旧运行已无效，因此监控读取返回 409。

## 本地紧急 Stop

即使 IsOS 服务未运行或归属方拒绝保存的运行 ID，CLI 命令 `isos stop` 仍可紧急关闭。它向确认属于所选工作区的本地进程发送 SIGTERM；进程仍存在时升级为 SIGKILL。保存的运行 ID 不是准入条件。确认没有匹配进程后，IsOS 退役过期的归属方运行收据，并记录紧急审计条目。审计仅表示进程已退出，**不证明执行器处于物理安全状态**，也不取代终止停止收据。命令显著警告此操作可能中断运行中的硬件。再次 Run 前，操作员必须独立检查硬件。

默认由操作系统的同一用户进程权限控制此命令。可选文件 `config/isos/emergency-stop.toml` 可包含 `code_sha256 = "<64 位小写十六进制字符>"`；`ISOS_STOP_CODE_SHA256` 环境变量会覆盖该值。配置后，命令要求 `--code-stdin` 并从标准输入读取口令。不匹配时，在发送信号前拒绝执行。不得把口令作为命令行参数传入，否则可能通过 shell 历史和进程列表泄露。可选口令是本地操作系统权限的补充，不会赋予关闭其他用户进程的权限。

在源码工作区运行 `isos stop` 时，工作区中的 `kill.sh` 脚本负责向进程发送信号，也支持 dry-run 预览。它将候选进程限制在此工作区，并报告被发送信号的 PID。脚本确认没有匹配进程后，IsOS 才退役收据。私有的紧急停止标记使离线状态显示为 `stopped`，终止结果为 `emergency_signal_unproven`；新的 Run 会清除该标记。重新启动前核实物理安全状态。

独立确认物理安全的操作员可在停止 IsOS 管理器后运行 `isos physical-stop recover --run-id <准确的 ID> --attest-safe`。CLI 要求准确、常规且私有的归属方运行收据，并要求本地扫描没有可能的进程候选。它先写入包含运行 ID、修订、收据 SHA-256 和 `operator_attestation` 的私有审计记录，然后才退役收据。缺少声明、管理器仍运行、收据变化、运行 ID 错误或存在进程候选都会拒绝恢复。此命令不向归属方发送信号，也不证明物理安全；操作员声明承担该判断。后续 Run 仍须通过正常的图、归属方与生成配置准入。
