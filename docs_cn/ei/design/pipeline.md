# EI 阶段数据流设计

[English](../../../docs/ei/design/pipeline.md) · [架构](architecture.md) · [当前 IsOS 适配器](../interfaces/setup.md)

## 边界与状态

本页是 EI 的内部阶段数据流与验证设计，不定义 IsOS 图端口或设置的公共契约。注册节点的类型、位置、端口和设置以 [IsOS 节点契约](../../isos/interfaces/node/README_cn.md)为准；[EI 节点边界](../interfaces/node.md)说明跨所有者路径。下面的值名帮助追踪 EI 运行状态。

## 公共阶段值

```json
{
  "schema_version": "0.1.0",
  "event_id": "01JAZQ3D5Q4N8PK1Y3V6W0M2BS",
  "run_id": "run-20261101-01",
  "backend": "robot-a",
  "observed_at": "2026-11-01T08:00:00Z",
  "kind": "observation",
  "payload": {
    "resource_ref": "camera/front",
    "status": "available"
  }
}
```

| 字段 | 类型 / 单位 | 用途 | 所有者 |
|---|---|---|---|
| `event_id` / `run_id` | 字符串 | 关联证据与运行 | EI 事件生产者 |
| `backend` | 字符串 | EOS 来源身份 | 链路验证 |
| `observed_at` | UTC 时间 | 来源观测时间 | 证据来源 |
| `kind` / `payload` | 枚举 / 类型化值 | 观测分类与内容 | 感知准入 |

所有图可配置阶段设置使用 IsOS setting descriptor，包含名称、类型、标签、描述、默认值、必填标记、验证规则和可选 selector 元数据。新注册类型的 `description` 必须非空；旧 sidecar 在读取时可得到仅用于展示的兼容描述。图边 `priority` 与 `frequency_hz` 是 IsOS 的传递控制，不改变 Mission 优先级或安全准入。

## 阶段目录与端口

下表对应 `ei/src/ei/stages.py` 的七个已注册阶段。端口名称用于定位内部数据流；可编辑图契约仍由 IsOS 定义。

| 注册类型 | 层 / 组 | 输入 | 输出 |
| --- | --- | --- | --- |
| `isos.ei_io.gateway@v0.2` | io / link | `gateway`, `admitted`, `emergency_stop` | `discovery`, `resource`, `result`, `health` |
| `isos.ei_evidence.admission@v0.2` | evidence / perception | `resource`, `hearing`, `result` | `observations` |
| `isos.ei_cognition.builder@v0.2` | cognition / context | `observations`, `body`, `memory` | `snapshot` |
| `isos.ei_cognition.queue@v0.2` | cognition / missions | `intent`, `result` | `selected`, `lifecycle` |
| `isos.ei_cognition.planner@v0.2` | cognition / decision | `selected`, `snapshot` | `proposal`, `plan_state` |
| `isos.ei_governance.gate@v0.2` | governance / safety | `proposal`, `registry`, `latch` | `admission`, `rejection` |
| `isos.ei_governance.health@v0.2` | governance / monitor | `health`, `progress`, `bounds` | `status`, `latch` |

A01 本地运行时拒绝其他 run 的事件、未来或无效时间戳，以及接收时超过 300 秒的事件。Resource 准入要求 `sample.text` 和 `sample.eos_robot_id` 均为非空字符串。拒绝诊断带稳定代码、源节点和事件 ID；校验失败后，修正事件可以复用该 ID。

### 运行值

| 值或设置 | 类型 / 单位 | 用途与默认值 | 验证、所有者与兼容性 |
|---|---|---|---|
| `resource` | EOS 有类型资源引用与样本 / 源单位 | 证据；无默认值。 | EOS 拥有 payload；link 验证来源和 schema；未知引用不能变成能力。 |
| `discovery` | 完整的 EOS 资源/能力发现代际 / 无 | Link 就绪及精确绑定；完整代际完成前不存在。 | Link 原子发布；部分或冲突的发现默认拒绝；底层 protobuf 字段由 EOS 拥有。 |
| `body` | 已验证的 EOS body 快照 / 源单位 | 可选硬件上下文；发现前不存在。 | EOS 拥有物理描述；link 验证 robot 和资源身份后才交给 context。 |
| `observations` | 有界序列/快照 / 源单位 | 提供给 context 的证据；有效但尚无证据时为空。 | Perception 管理准入、排序、合并、省略计数；新观测变体需要准入策略。 |
| `snapshot` | 有界上下文文档 / token | 供模型读取的状态；允许空的可选区块。 | Context 管理总 token 预算和确定性排序；不含原始秘密或无限媒体。 |
| `intent` / `selected` | Mission 文档与修订 / 无 | 操作员请求与已选工作；队列空时不存在。 | Missions 管理身份、来源、优先级、版本和 checkpoint；拒绝过期修订。 |
| `lifecycle` | Mission ID、修订和状态转移 / 无 | 传递 Mission 变化；发生转移前不存在。 | Missions 管理单调修订和持久 checkpoint；同 ID 重复转移幂等，冲突失败。 |
| `proposal` | 有类型的规划 primitive 引用 / 无 | 候选动作；无默认值。 | Decision 解析模型输出；必须引用 backend、device、primitive、version 和参数；没有执行权。 |
| `plan_state` | Plan ID、修订、阶段和结果 / 无 | 报告规划进度；选中 Mission 前不存在。 | Decision 管理；无效转移和过期修订失败，模型超时不算成功。 |
| `registry` | 完整、已验证的能力代际 / 无 | 安全门查询；发现成功前不存在。 | Link/registry 管理精确 backend/device/primitive/version 绑定；safety 拒绝缺失或过期代际。 |
| `admission` / `rejection` | 安全门决定及原因 / 无 | 放行或解释拒绝；每个提案只有一个终结决定。 | Safety 管理；放行数据必须精确绑定当前 registry；未知、过期、无效数据默认拒绝。 |
| `result` | EOS 操作 ID 与终结状态 / 无 | 反馈；无默认值。 | EOS 管理终结操作语义，link 验证身份；超时和取消不是成功。 |
| `health` / `latch` | 状态枚举和单调 episode / 无 | 报告降级及紧急状态；只有完成初始化验证后才能视为健康且未锁存。 | Monitor 管理；锁存状态只能经授权重置，不能靠重启清除。 |
| `progress` | Mission/操作 ID 和单调已用时间 / 秒 | Monitor 输入；没有运行工作时不存在。 | Decision/link 管理测量值；monitor 限制过期，不能从无更新推断完成。 |
| `bounds` | 有限测量值和配置界限 / 源单位 | 可选 monitor 安全输入；未配置界限来源时不存在。 | Monitor 检查单位和来源身份；缺少必需界限时默认拒绝。 |
| `status` | 健康状态、episode、原因 / 无 | 面向操作员的监控投影；初始化为不可用。 | Monitor 管理状态转移；过期或部分 backend 状态不能映射为健康。 |
| `admitted` / `emergency_stop` | 有类型的命令引用 / EOS 单位 | 固定运行时出口，不是用户可编辑图线；无默认值。 | Safety 管理工作负载准入，只有 monitor 管理紧急停止；link 验证调用者，EOS 负责最终准入。 |

### 阶段设置

| 阶段 / 设置 | 类型 / 单位 | 用途与安装默认值 | 验证与兼容性 |
|---|---|---|---|
| `link.backend_alias` | 字符串 / 无 | 稳定 EOS 路由；每个 link 必填。 | 非空，link 实例之间唯一；旧单 backend 模式规范化为 `default`。 |
| `link.gateway` | 端点引用 / 无 | IsOS 解析的 EOS gRPC 端点；managed 模式必填。 | TCP host/port 或绝对 UDS 路径；拒绝 HTTP control/catalog 端点；旧 TCP profile 没有 `transport` 时仍解释为 TCP。 |
| `perception.capacity` | 整数 / 记录数 | 准入 FIFO 上限；64。 | `1..10000`；溢出计数并报告省略。 |
| `perception.history_per_kind` | 整数 / 每类记录数 | 提示历史深度；8。 | `1..10000`；修改后不重新解释已存事件。 |
| `missions.mission_capacity` | 整数 / 待处理 Mission 数 | 队列准入上限；8。 | `1..1024`；队列满时拒绝新 Mission。 |
| `decision.controller.model` | selector ID / 无 | 快速模型 binding；`gemma4-fast`。 | 非空、存在于 `models.toml`、不同于 reasoner；不可用的已保存 ID 仍显示但阻止 Apply。 |
| `decision.reasoner.model` | selector ID / 无 | 慢速模型 binding；`gemma4-reasoning`。 | 与 controller 相同的校验，且必须与之不同。 |
| `decision.max_invalid_replies` | 整数 / 模型回复数 | 连续错误模型输出导致 Mission 失败的上限；3。 | 至少 1；传输重试不计入。 |
| `decision.step_timeout_s` | 有限数 / 秒 | Primitive 步骤出口超时；30。 | 大于 0；超时不等于 EOS 完成。 |
| `monitor.heartbeat_interval_s` | 有限数 / 秒 | Backend 健康轮询间隔；5。 | 大于 0；图边频率不能代替此安全监控节奏。 |
| `monitor.failure_threshold` | 整数 / 轮询次数 | 连续心跳失败后升级的门限；3。 | 至少 1；只有验证恢复后才重置。 |
| `monitor.stall_timeout_s` | 有限数 / 秒 | 运行中 Mission 没有新 primitive 结果的偏离门限；安装默认 150。 | 大于 0；进行中的模型推理不算 primitive 停滞。 |

## 准入、错误与兼容行为

**正常：** 完整发现代际使 link ready；证据经过 perception/context；已选 Mission 产生提案；gate 放行一个精确 primitive 或输出拒绝；EOS 终结结果回到证据链。**空输入：** 没有 Mission 或可选观测时阶段等待，不编造数据。**错误、重复、过期：** 所属阶段用稳定事件码、节点 ID 和关联 ID 拒绝。**不可用、部分结果：** 必需的发现阻止 ready，可选模型/记忆可降级。**超时、取消：** 在 EOS 终结证据到来前保留 pending/failed，不能推断物理完成。**重启：** 读取 checkpoint，创建新的 run ID，重新发现 EOS；紧急锁存必须经授权重置。

图边调度器可以优先和限速传递证据，但 gate 始终依据当前已验证状态准入。有界 observation/context 队列溢出时要计数并记录日志；安全和终结结果事件不能被静默丢弃。图不能把模型提案直连 link，也不能配置任意紧急发送方。唯一的紧急旁路是 monitor 的专用停止出口。

## 接口验证

Q03 所有者资格验证设计规定现有两条 A04 证据路径的测量。负载与指标是测试证据，不增加阶段端口、QoS 设置或安全路径。所选安装 wheel 与跨所有者回放仍是发布门槛。

[Q01 Vision 证据入口](../interfaces/vision-evidence.md) 是独立的进程所属只读检查路径。它不增加第八阶段、图端口、A04 `resource` 图像解释、提示输入或执行器路由。Q01 中 gate 帧不进入此阶段调度器。

为每个端口和设置 descriptor、未知字段/增量字段兼容、未来类型拒绝、不可用 selector 值、错误/过期/重复事件建立 contract fixture。EOS stub 集成测试须证明图线无法绕过 gate，断线不会生成成功结果。实现后把完整输出保存在 `data/test/<run-id>/`。

## 激活数据传递 MVP

IsOS 触发的演示路径从私有所有者 `/v1/activate` 路由进入 EI。所有者按选定图和运行依次执行输入 → 意图 → 安全节点。这些阶段原样传递激活封套；EI 不选择关节数值。现有受管工作进程继续提供进程就绪状态和生命周期。只有在该进程存活且本次运行的 EOS 所有者套接字可用时，安全阶段才转发事件。随后由 EOS 负责命令接纳及模拟机械臂运动。所有者套接字串行处理 Activate 和 Stop，因此 Stop 在结束工作进程前关闭新事件接纳。EOS 传输结果不确定时，EI 向 IsOS 报告且不重试。

2026-11-09 的 EI 激活边界检查使用 `uv sync --project ei --extra dev --extra vec --frozen`、`ei/.venv/bin/pytest ei/tests -q`（736 通过、6 跳过）、`ei/.venv/bin/ruff check ei/src/ei ei/tests`（通过）、在 `ei/` 中运行 `.venv/bin/pyright src/ei`（0 错误），以及在 `ei/` 中运行 `PYTHONPATH=src:libexec .venv/bin/lint-imports`（6 项导入契约保持）。完整输出保存在忽略的 `data/test/ei-arm-activation-20261109/`。

本次双语文档链接检查运行 `python3 scripts/check_doc_links.py --all`，检查了 370 个 Markdown 文件，未发现损坏的相对链接。完整输出保存在 `data/test/ei-arm-activation-20261109/links.log`。

## 机械臂监测回传 MVP

EI 所有者将 `POST /v1/monitor` 作为经过三个注册节点的只读传递处理。路径检查把 EOS egress 的 `observation` 绑定到 EI 输入、状态和监测节点。所有者按监测节点的 `forward_rate_hz`（默认 20，范围 1–20）轮询 EOS，并在对应间隔内复用最近验证的观测；不会合成新采样或关节数值。运行或修订不符、工作进程丢失、图路径缺失、EOS 数据无效或与配置的关节名称不符、序号倒退均拒绝处理。Stop 清空缓存。最终观测保留 EOS 身份、序号、采样时间、六个关节名称、就绪或故障状态，并且仅在就绪时包含位置。

2026-11-09 的 EI 监测检查使用 `uv sync --project ei --extra dev --extra vec --frozen`、`ei/.venv/bin/pytest ei/tests -q`（743 通过）、`ei/.venv/bin/ruff check ei/src/ei ei/tests`（通过）、在 `ei/` 中运行 `.venv/bin/pyright src/ei`（0 错误）、在 `ei/` 中运行 `PYTHONPATH=src:libexec .venv/bin/lint-imports`（6 项导入契约保持），以及 `python3 scripts/check_doc_links.py --all`（372 个文件，链接无错误）。完整输出保存在忽略的 `data/test/ei-arm-monitor-20261109/`。

配置化监测检查（2026-11-09）：自定义六个关节名称的机械臂观测，只有在名称及顺序匹配 Run 的 `endpoints.arm_monitor.joint_names` 时才通过 EI；`forward_rate_hz` 设置改变缓存间隔，超过 20 的值被拒绝。`ei/.venv/bin/pytest ei/tests -q` 的 745 项测试通过，Ruff 与 Pyright 通过。完整输出保存在忽略的 `data/test/ei-arm-monitor-20261109/configured-monitor-*.log`。
