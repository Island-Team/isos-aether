# safety 节点设计

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/design/governance</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [Governance / monitor](monitor.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/ei/interfaces/node/design/governance/safety.md)

## 注册节点行为

输入为 `proposal`、`registry`、`latch`，输出为 `admission`、`rejection`。只有已准入请求走固定的内部链路路径；EOS 执行最终物理准入。[阶段数据流](../../../../core/runtime/pipeline.md)描述 EI 内部值；图端口和设置由 IsOS 定义。


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

**输入：** health、progress、可选 bounds。**输出：** status、latch。[阶段数据流设计](../../../../core/runtime/pipeline.md)描述 EI 运行值。图端口、设置及连线优先级和频率的契约归 IsOS 所有。

**算法与故障：** 以单调时钟期限观察心跳、进度和测量边界。把确认的故障升级为持久停止锁存；只有专用紧急路径可绕过普通动作准入。未经授权复位，重启不清除锁存。

## ei.governance.safety.gate@v0.2

在 EOS 准入前对激活路径应用 EI 安全门。 [节点接口](../../governance/safety_cn.md#governance_safety_cn--gate)
