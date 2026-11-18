# 安全门与紧急停止

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/core/governance</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README.md)
- 同级文档：
  - [Governance / monitor](monitor.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/ei/core/governance/safety.md) · [架构](../architecture.md) · [阶段数据流](../runtime/pipeline.md)

规划器与操作员动作均经过 `SafetyGate`。远程调用按 `(robot alias, device_id, primitive name)` 对当前发现目录做精确解析；目标缺失、歧义、未知或过期时，在传输前拒绝。MVP 没有 Agent 本地执行路径。EOS 仍拥有最终执行准入。

## 实现与输出

`gate.py` 拥有普通准入，返回决定、原因和可选 EOS 确认；`audit.py` 消费故障观测，合并已恢复的 episode，并通过监控拥有的锁存升级未恢复状态；`action_list.py` 只给出冲突建议，不执行或预留设备。门控在 `SubmitOperation` 前建立仅所有者可见的操作 ID 检查点，只根据终态 EOS 凭据结算。启动时通过 `GetOperation` 查询待结算 ID，绝不重新提交。

`primitive.ack.accepted` 只表示 EOS 接收了放行请求，并非物理完成。后端拒绝是正常结果；传输失败或畸形确认记为 `egress_failed`，不能合成成功确认。

## 可见性与门控顺序

EOS 发现的标准原语可供规划器使用。操作员原语同时要求 EOS 暴露及 `primitives.expose_operator = true`；特权 `motion.stop` 和 `safety.estop` 从不向规划器开放。操作员 WebSocket 调用同一安全门。

门控依次检查紧急锁存、调用者可见性、精确原语与目标、规范参数 schema、设备限制、配置策略、运行状态和速率/冷却窗口。首个失败返回稳定拒绝原因并写审计观测，后续检查和传输不再运行。注册项缺失、重复、过期或歧义时默认拒绝。布尔值不能充当数值，非有限值无效，设备缺少必需限制时调用不可用。

## 全机群紧急序列

紧急介入先建立屏障，阻止新负载与停止操作竞争。然后对每个已发现 `base`、`arm` 或 `gimbal` 设备发送 `motion.stop`，再以空设备目标发送一次进程范围的 `safety.estop`，最后在机群锁存中记录当前 EOS UUID。所有端点都尽力尝试；一个失败不会阻止其余设备。再次介入已锁存状态是幂等的。目标消失、发现不完整、后端拒绝、超时与传输失败分别记录，均不能清除屏障。

监控拥有紧急升级，可直接要求链路执行停止；普通工作始终遵循 `decision/operator → SafetyGate → EosClient → CapabilityService.SubmitOperation`。`action_list` 不提供第二条执行路径。

## 注册节点行为

输入为 `proposal`、`registry`、`latch`，输出为 `admission`、`rejection`。只有已准入请求走固定的内部链路路径；EOS 执行最终物理准入。[阶段数据流](../runtime/pipeline.md)描述 EI 内部值；图端口和设置由 IsOS 定义。


## 健康、边界与紧急监控

本次调整仅改变 EI Python 模块所有权；注册图节点 `ei.governance.monitor.arm_status@v0.2` 的身份和 IsOS 契约保持不变。



## 独立监控任务

`ei.core.governance.safety.monitor_entry` 仍由 supervisor 独立托管，避免健康探测阻塞普通动作准入。

### Heartbeat 循环

`monitor_entry` 将单客户端或 backend 映射规范化，为各 backend 建独立失败计数与升级 episode，设置 ready 后按 `heartbeat_interval_s` tick。每轮串行探测所有 backend， 因此慢探测会延迟后续探测。

任何失败后的成功 health check 会强制 rediscovery，再刷新 typed status；两步任一失败 都算失败 tick。成功 rediscovery 更新 tracker，estop 已 engaged 时还尝试 restart reconciliation。完全成功会记录 tracker success，若先前升级则只发一条 recovered 观测，并清零计数。失败使计数递增；首次失败记录日志，达到 `failure_threshold` 每个 episode 只发一个 fault，持续失败合并。每轮后 `availability_changed` 收到是否所有计数 均为零。

## Mission stall 流程

Heartbeat 后读取 active mission、最新 `result` 和 mission start。无活动任务会重置 episode。只有结果年龄与启动年龄都达到 `stall_timeout_s` 才算 stall，以保护较慢的首轮 推理；缺失时间视为已超时。首次 stall 发 fault、注释 plan 并请求 decision interrupt， 后续 tick 合并，直到新活动或任务结束重新武装。

## 边界与速率状态

`PhysicalBounds.check_velocity` 先以 `hypot(x,y)` 检查平面速度，再检查绝对 yaw rate；默认上限为 2 m/s 与 3.14159 rad/s，同时越界时先报告速度。`BoundWatchdog` 忽略非 base 快照， 每个越界帧都发 fault；默认连续五帧后只 engage estop 一次，健康帧清零并重新武装。 streak 是 watchdog 全局状态，不按 backend/device 分组。非有限数不在此显式拒绝，依赖 上游 typed contract。

`RateWindow.admit` 使用 monotonic time，删除至少 60 秒前的时间戳，只为已接纳尝试 占槽；之后传输失败仍耗槽，拒绝不追加。它只在内存中，进程重启即重置。

## 紧急生命周期

Engage 幂等，并在传输前建立本地屏障。对每个 backend，latch 发现当前 EOS UUID 并调用 进程级 `safety.estop`；异常或 EOS 拒绝按 backend 记录且不终止 fleet 循环。未知 UUID 存为空，不能证明后续重启。`motion_stop()` 独立遍历所有 backend 与 motion device， best-effort 调 `motion.stop`，任一失败不阻止其余调用。

## 监控节点行为

**输入：** health、progress、可选 bounds。**输出：** status、latch。[阶段数据流设计](../runtime/pipeline.md)描述 EI 运行值。图端口、设置及连线优先级和频率的契约归 IsOS 所有。

**算法与故障：** 以单调时钟期限观察心跳、进度和测量边界。把确认的故障升级为持久停止锁存；只有专用紧急路径可绕过普通动作准入。未经授权复位，重启不清除锁存。

## ei.governance.safety.gate@v0.2

在 EOS 准入前对激活路径应用 EI 安全门。 [节点接口](../../interfaces/node/governance/safety_cn.md#governance_safety_cn--gate)
