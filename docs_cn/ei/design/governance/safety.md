# 安全门与紧急停止

[English](../../../../docs/ei/design/governance/safety.md) · [架构](../architecture.md) · [阶段数据流](../pipeline.md)

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

输入为 `proposal`、`registry`、`latch`，输出为 `admission`、`rejection`。只有已准入请求走固定的内部链路路径；EOS 执行最终物理准入。[阶段数据流](../pipeline.md)描述 EI 内部值；图端口和设置由 IsOS 定义。
