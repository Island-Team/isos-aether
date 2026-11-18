<a id="cognition_decision_cn"></a>
<a id="cognition_decision_cn--cognition--decision"></a>
# Cognition / decision

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/cognition</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [Cognition / context](context_cn.md)
  - [Cognition / intent](intent_cn.md)
  - [Cognition / missions](missions_cn.md)
  - [Cognition / state](state_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/ei/interfaces/node/cognition/decision.md#cognition_decision) · [层目录](../../../interfaces_cn.md#readme_cn--cognition) · [EI 节点边界](../../../../isos/interfaces/server/ei_cn.md#node_cn)

[共享节点契约](../../../../node/interfaces/README_cn.md)定义类型身份、设置、端口、路由和注册规则；EI 负责本组节点的行为。本目录说明该组节点的 EI 行为，运行职责见[组设计](../../../core/cognition/decision.md)。

<a id="cognition_decision_cn--planner"></a>
<a id="planner"></a>

<a id="cognition_decision_cn--eicognitiondecisionplannerv02"></a>
## ei.cognition.decision.planner@v0.2

直接 `PlannerStage` 处理器接受带非空 `mission_id` 的选中任务，以及 `text` 不超过 65,536 字节的上下文快照。缺少已验证模型提供者时，它输出 `state:"abstained"`、`reason:"missing_model"` 的 `plan_state`，绝不伪造运动。注入提供者必须附带精确模型文件和 SHA-256 摘要；构造及每次调用前均检查。提供者必须返回非空 `plan_id`、1 到 `max_steps` 条短步骤字符串，以及含非空 `backend`、`device`、`primitive`、`version`、`target_id`、对象 `arguments` 和未来带时区 `expires_at` 的提议。`max_steps` 默认 8（范围 1–64），`deadline_ms` 默认 1000（范围 1–30000）。无效或超时输出在发出前被拒绝。`ei-config` 将此目录类型标为需要已验证模型，并以 `missing_model_package` 拒绝图选择；此独立阶段不声称包到图的规划器接线。Stop 关闭输入。
