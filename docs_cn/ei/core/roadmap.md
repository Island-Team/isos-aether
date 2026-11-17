# EI 路线图

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EI 核心](README.md)
  - [EI 架构](architecture.md)
- 下级目录：
  - [cognition/](cognition/)
  - [evidence/](evidence/)
  - [governance/](governance/)
  - [io/](io/)
  - [optional/](optional/)
  - [runtime/](runtime/)

</details>
<!-- doc-navigation:end -->

[当前架构](architecture.md) · [English](../../../docs/ei/core/roadmap.md)

## 目标架构（尚未实现）

**目标设计——尚未实现。** EI 应将异构证据变为带时间信息的表示，在明确资源和风险约束下选择可复用技能，并使用世界模型（WM）预测、检查和修订短时动作。这是研究及架构方法，不表示已集成某个 Physical AI、WAM 或 VLA 模型。模型输出不能绕过 EI 安全门或 EOS Dispatcher。

### 1. 多模态统一张量空间

为图像、深度、语言、本体状态、力／触觉和 EOS 语义事件设置模态适配器。每个适配器输出类型化 token 或张量，以及模态 ID、来源及校准版本、坐标系、单位、采集时间、到达时间、序号、有效性掩码和不确定度。围绕决策时间戳对齐有界时间窗口；通过掩码保留异步频率和缺失模态，不伪造同步数值。原始传感证据及来源可供审计，融合表示则是派生、带版本的模型输入。统一空间意味着共同的时间及语义索引方式，并非把每种传感器强制变为相同数值形状。

首版用显式特征适配器与后期融合。先测量对齐误差、传感器缺失行为和下游任务价值，再训练更大的联合编码器。EOS 提供已验证的物理测量和语义目录；EI 负责任务意义及认知记忆。仿真专属真值可标注离线数据集，但不得进入推理张量。校准或编码器版本变化会使受影响证据的缓存嵌入失效。

### 2. VLA 技能学习与路由

将慢速 VLA 规划器、技能路由器及短时技能策略分开。VLA 提出经定位的子目标和预期结果。路由器依据观测状态、进入条件、实体能力、资源占用、模型置信度、持续时间及风险预算比较候选技能；可弃权或请求新观测。选中技能接收类型化、会过期的目标，其中包含坐标系、目标不确定度、允许的关节或资源、速度与力限制及完成判据。技能输出有界运动提案或 EOS 支持的原语请求，绝不直接写入未检查的执行器。EOS 在物理写入边界再次检查设备状态、新鲜度、限制和 Stop。

先建立小型固定技能库：单关节运动、协调到达、保持和柔顺接触。先在多样配置与扰动上训练通用运动和接触响应，再用保留对象评估。以相同任务、传感、时间和风险预算，将学习式关节空间策略与调优的 IK、阻抗控制比较。单技能迁移成功后再加入并发技能集合；同时执行需要资源仲裁、共享执行器组合、驻留规则和抢占，紧急 Stop 始终优先。学习式路由器可改善调度，但不能定义物理安全限制。

### 3. 决策环中的 WM

将 WM 视为可替换的预测服务，可由物理感知模型、WAM 或更简单的动力学基线实现。EI 以有界决策频率提供当前融合证据、候选技能、实体配置和预测时域；WM 返回预测观测、接触或约束事件、不确定度及有效时域。执行后 EI 将预测与新鲜 EOS 观测比较；差异事件触发重新观测、调整技能、更换接触假设或 Stop。快速接触反射及 e-stop 留在 EOS，因为模型推理延迟和不确定性无法保护即时物理环。

先让候选 WM 以影子模式运行：记录预测，不改变命令。仅在校准不确定度、延迟、保留集结果预测及恢复测试优于无 WM 和简单动力学基线后提升。遇到超时、输入过期、分布外状态或模型分歧时，路由器弃权或使用已明确验证的保守行为。模型不得把自身预测状态当成观测事实。EI 在认知记忆中保存决策和预测来源；EOS 保留权威物理证据，IsOS 保留部署追踪身份。

### 拟议的认知管线演示架构


这是拟议的图架构，不是当前的起始图或仓库混合示例。当前图未声明完整的认知侧链。未来的图可以注册十三个使用演示风格 ID（`ei.<layer>.<group>.<name>@v0.2`）的 EI 节点类型，而不更改生产 `isos.ei_*` 阶段契约。

| 演示类型 | 位置 | 图输入 | 输出 |
|---|---|---|---|
| `ei.io.link.gateway@v0.2` | io / link | 无；归属方注入 `resource` | `resource`（`resource_bundle@v0.2`）、`health`（`health_update@v0.2`） |
| `ei.evidence.perception.admission@v0.2` | evidence / perception | `resource` | `observations`（`observation_set@v0.2`） |
| `ei.cognition.context.builder@v0.2` | cognition / context | `observations` | `snapshot`（`context_snapshot@v0.2`） |
| `ei.cognition.missions.queue@v0.2` | cognition / missions | 无；归属方注入 `intent` | `selected`（`selected_mission@v0.2`）、`lifecycle` |
| `ei.cognition.decision.planner@v0.2` | cognition / decision | `selected`、`snapshot` | `proposal`（`primitive_proposal@v0.2`）、`plan_state` |
| `ei.governance.monitor.health@v0.2` | governance / monitor | `health` | `status`、`latch` |

其余七个拟议演示类型将保留激活与机械臂监测路径：`ei.io.input.trigger`、`ei.cognition.intent.context`、`ei.cognition.intent.forwarder`、`ei.governance.safety.gate`、`ei.io.input.arm_status`、`ei.cognition.state.arm_status` 和 `ei.governance.monitor.arm_status`。

在拟议设计中，激活路径将从私有 `/v1/activate` 路由进入。归属方将按顺序求值已声明的输入、意图上下文、转发和安全节点，并用同一事件播种认知侧链：网关收到一条资源事件，任务队列收到一条以按钮事件 ID 为键的演示任务。意图上下文节点将把最新 `arm_monitor_state@v0.2` 观测准入共享感知队列，并用产品 `ContextBuilder` 渲染上下文快照，同时按原样输出激活事件。转发节点将在 `snapshot` 输入上缓存该快照，并原样转发激活事件。

安全门将在可选输入上接收 `proposal`，但从不输出它。因此规划器提议将无法改变到达 EOS 的激活值，归属方将只在 EI 日志中记录观察到的提议。这样将使 EOS 入口契约与激活 MVP 保持一致，同时仍完整演练感知 → 上下文 → 任务 → 决策 → 治理序列。演示 EI 路由将声明为进程内连接，IsOS 将为这些数据类型准入 EI→EI 路由边。

### 方法与验收门禁

1. 在 EI 所属契约中定义带版本的张量和技能接口；拒绝缺失坐标系、过期采样、单位不符、无效掩码和不支持的实体。
2. 从 EOS 证据建立可重放固定数据集，调参前冻结训练、验证及保留对象族；分别统计仿真回合、教师时间、实体修正、VLA 调用与计算成本。
3. 建立有界单关节、协调运动和保持检查，再于保留接触任务上比较学习技能与调优 IK／阻抗控制；同时报告成功率、力／碰撞越界、耗时、延迟和不确定性区间。
4. 在相同尝试预算下比较盲目重试与证据驱动修订，WM 先以影子模式运行；仅当更改假设改善结果且未增加物理风险时提升。
5. 通过真实 EI–EOS 边界重放过期证据、模型故障、分歧、Stop 和运行替换；模型故障或模态缺失不能产生无界命令。

独立的 Q03 视觉入口资格验证目标需要重复的 stub stream；它不取代一次性 Q01 重复代际字节和身份回放。两项目标检查仍需实现并分别保存证据。


## 设计验证

2026-11-11 的仅文档变更运行了双语链接检查、IsOS 文档检查、`git diff --check HEAD` 和架构标题层级一致性检查。命令及完整输出见 [IsOS 验证](../../isos/core/server_cn.md)；本设计不声称 EI 运行时行为已经实现。
