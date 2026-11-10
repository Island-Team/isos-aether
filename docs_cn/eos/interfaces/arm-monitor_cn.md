# EOS 模拟机械臂观测

[English](../../../docs/eos/interfaces/arm-monitor.md) · [EOS 接口](README_cn.md) · [机械臂控制](arm-control_cn.md)

已应用的 EOS 投影包含独立返回路径：零图输入观测源 `eos.hardware.mock.arm_observer@v0.2` → `eos.dispatcher.arm.monitor@v0.2` → `eos.skill.demo.arm_status@v0.2` → `eos.interface.egress.arm_status@v0.2`。每个端口名为 `observation`，采用事件交付，传递观测数据 `arm_monitor_state@v0.2`。三条内部边使用进程内契约 `{owner:eos,id:arm-monitor-local,version:1}`。出口使用 Unix socket 契约 `{owner:eos,id:arm-monitor-to-ei,version:1}`。EOS 在 Run 前要求这些节点及连接齐全。

观测源是 EOS 硬件层中的只读软件节点，没有图输入或执行器绑定；它读取唯一模拟机械臂适配器的周期样本，采样序号独立于激活事件递增。模拟机械臂仍是唯一执行器。旧的有输入 `eos.hardware.mock.arm_output@v0.2` 类型仍适用于使用该路径的已保存图；其监控载荷中的 `source_node` 仍为模拟机械臂路径。

默认情况下，模拟机械臂每 10 ms（100 Hz）采样并验证六个关节。其可选高级节点设置 `sample_interval_ms`（1–1000 ms，默认 10）和 `freshness_timeout_ms`（2–5000 ms，默认 50）要求新鲜度阈值至少为采样间隔的两倍。初始样本为六个零；每次准入的运动更新关节后立即采样。样本超过配置的新鲜度阈值时进入 `stale`；关节值非有限数或越界进入 `degraded`。两种状态都阻止运动，直到新的有效样本恢复 `ready`。Stop 关闭运动准入并报告 `stopped`。仅 `ready` 观测包含位置数组。

## 载荷

`arm_monitor_state@v0.2` JSON 对象恰好包含以下字段：

| 字段 | 类型及规则 |
| --- | --- |
| `run_id`、`revision` | 当前运行 ID 和 64 位小写修订摘要。 |
| `source_node` | 图绑定的观测源路径，例如 `eos/hardware/mock/arm_observer`。 |
| `sequence` | 此工作进程运行内的正整数样本序号。 |
| `sampled_at_monotonic_ns` | 非负单调采样时间，单位纳秒。 |
| `joint_names` | 按图顺序排列的六个唯一名称。 |
| `joint_positions_rad` | `ready` 时为六个有限弧度值；其他状态为 `null`。 |
| `state` | `ready`、`degraded`、`stale` 或 `stopped`。 |
| `reason_code` | 就绪时为 `null`；否则为 `invalid_joint_sample`、`sample_timeout` 或 `stopped` 等原因。 |

## 私有所有者读取

EI 在权限为 `0600` 的 EOS 所有者 Unix socket 上调用 `POST /v1/monitor`。精确请求为 `{ "schema_version":1, "core":"eos", "op":"monitor", "run_id":"run_1", "revision":"<64 位小写十六进制>" }`。EOS 要求匹配当前运行和修订，读取最新工作进程样本而不移动机械臂，返回 `{ "schema_version":1, "core":"eos", "ok":true, "run_id":"run_1", "revision":"<revision>", "observation":{...} }`。EI 可最多以 20 Hz 轮询；EOS 安全采样独立保持 100 Hz。工作进程不可用时返回 `worker_unavailable` 和 HTTP 503。运行隔离过期时返回 `stale_or_missing_run` 和 HTTP 422。Stop 使读取隔离失效。
