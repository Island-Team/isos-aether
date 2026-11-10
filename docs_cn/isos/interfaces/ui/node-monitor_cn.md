# 机械臂监控节点

[English](../../../../docs/isos/interfaces/ui/node-monitor.md) · [节点类型](../node/README_cn.md) · [按钮事件](node-events_cn.md)

起始图通过 EOS 硬件输出、调度器监控、技能状态和接口出口节点，EI IO 输入、认知状态和治理监控节点，最终到 `isos.ui.monitor.arm@v0.2`，返回模拟机械臂的六关节观测状态。硬件输出节点是没有图输入的源节点。EOS 在本地读取同一个模拟机械臂采样器，因此图中有独立的返回链，也不会创建第二条执行器路径。

每条返回边都通过 `observation` 端口承载 `arm_monitor_state@v0.2`。载荷包含当前 `run_id`、`revision`、图中声明的观测 `source_node`（双链演示图为 `eos/hardware/mock/arm_observer`，旧图为 `eos/hardware/mock/arm`）、递增的 `sequence`、`sampled_at_monotonic_ns`、六个有序的 `joint_names`、`state`（`ready`、`degraded`、`stale` 或 `stopped`）及可为空的 `reason_code`。仅当状态为 `ready` 时，`joint_positions_rad` 才包含六个有限的弧度值；其他状态下为 null。EOS 在命令准入前检查每个 100 Hz 本地采样，EI 最多接收 20 Hz，IsosUI 最多以 5 Hz 刷新。这些速率及新鲜度限制位于图节点的高级设置中。

`GET /api/isos/v1/nodes/monitor` 使用常规 IsOS API 信封，数据为 `{state, data_type, run_id, active_revision, observation}`。部署停止时返回 `state:"stopped"`，运行标识及观测均为 null。部署运行或降级但运行标识已知时，IsOS 检查保存图的修订和完整的返回路径，然后每 200 ms 最多读取一次经过验证的 EI 快照，供所有本地 UI 读取者共享。缓存键包含部署、运行和修订；运行切换时不会返回旧快照。IsOS 在响应前再次检查运行状态。路径不完整、图已过期、需要重新读取时 owner 不可用或响应无效时均拒绝返回状态。IsosUI 只调用此 IsOS 路由，并在 Stop 或运行切换时清除旧值。

当前运行使用旧图修订时，端点返回 `graph_fence_stale`（409）。IsosUI 将其说明为旧运行状态，而非编辑冲突；停止或恢复旧运行并运行已保存的节点图之前，监控保持为空。
