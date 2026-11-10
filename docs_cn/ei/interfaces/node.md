# EI 激活节点

[English](../../../docs/ei/interfaces/node.md) · [配置](setup.md) · [架构](../design/architecture.md)

图 `0.2.0` 注册三个激活数据传递节点和三个机械臂监测数据传递节点：`ei.io.input.trigger@v0.2` 位于 `io/input`，`ei.cognition.intent.forwarder@v0.2` 位于 `cognition/intent`，`ei.governance.safety.gate@v0.2` 位于 `governance/safety`。每个节点都有名为 `activation` 的输入和输出，传递 `button_activation@v0.2`，交付模式为 `event`，数据类别为 `event`。节点设置为空。旧健康状态节点和仅声明型节点不再接纳。

输入节点通过 `unix_socket` 接收 IsOS 所有者桥接，契约为 `{owner:isos,id:button-activation,version:1}`。其输出及认知节点的输入、输出使用 `in_process`，契约为 `{owner:ei,id:activation-local,version:1}`。安全节点接收此本地契约，并通过 `unix_socket` 和 `{owner:ei,id:activation-to-eos,version:1}` 向外发送。图输入引用必须构成 IsOS 按钮 → EI 输入 → 认知 → 安全的有序路径。IsOS 验证穿过 EOS 的完整路径。

EI 原样传递 `{event_id,kind:"activate"}`，不选择运动。MVP 中所有者执行这三个数据传递阶段，而受管 EI 工作进程提供就绪状态和生命周期。安全阶段在转发前检查选定运行、修订、工作进程存活、图路径和 EOS 所有者端点；它不建立执行器通道。只有 EOS 接纳机械臂命令。

## 只读机械臂监测回传

回传链注册 `ei.io.input.arm_status@v0.2`（`io/input`）、`ei.cognition.state.arm_status@v0.2`（`cognition/state`）及 `ei.governance.monitor.arm_status@v0.2`（`governance/monitor`）。每个节点的 `observation` 输入和输出传递 `arm_monitor_state@v0.2`，交付模式为 `event`，数据类别为 `observation`。监测节点可在高级设置中指定 `forward_rate_hz`（默认 20，范围 1–20）；输入和状态节点设置为空。EOS egress → EI 输入的 Unix 套接字契约为 `{owner:eos,id:arm-monitor-to-ei,version:1}`。EI 输入 → 状态 → 监测的进程内契约为 `{owner:ei,id:arm-monitor-local,version:1}`。EI 监测 → IsOS UI 的 Unix 套接字契约为 `{owner:ei,id:arm-monitor-to-isos,version:1}`。

监测设置描述符声明 `data_type: "number"`，与 IsOS 节点目录的设置模式一致。

EOS 采样源在独立观察适配器中为 `eos/hardware/mock/arm_observer`，在现有已应用图中为 `eos/hardware/mock/arm`。EI 只接收自己的图投影，因此接纳这两个精确的 EOS 来源路径；运行、修订和有序关节名称约束仍然生效。

EI 所有者在运行所属的后台任务中每秒最多按 `forward_rate_hz` 读取 EOS 私有 `POST /v1/monitor` 端点，与 IsOS 读取者无关。它要求选定运行和修订、存活的工作进程、完整的 EOS egress → EI 输入 → 状态 → 监测声明路径，以及不倒退的 EOS 序号。Run 从已应用 EOS 机械臂节点取得 `endpoints.arm_monitor.joint_names`；EI 拒绝关节名称和顺序不同的观测。验证 EOS 响应后，所有者原样传递其中的 `observation` 对象。`ready` 携带六个有限的弧度关节位置；`degraded`、`stale` 和 `stopped` 携带空位置及明确原因。已停止或过期运行不能读取新数据。IsOS UI 最多以 5 Hz 获取最终 EI 观测；回传链绝不调用运动准入。
