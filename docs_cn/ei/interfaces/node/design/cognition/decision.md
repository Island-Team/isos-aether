# decision 节点设计

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/design/cognition</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [context 节点设计](context.md)
  - [cognition / intent 节点组](intent.md)
  - [missions 节点设计](missions.md)
  - [cognition / state 节点组](state.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/ei/interfaces/node/design/cognition/decision.md)

## 注册节点行为

**输入：** selected、snapshot。**输出：** proposal、plan_state。[阶段数据流设计](../../../../core/runtime/pipeline.md)描述 EI 运行值。图端口、设置及连线优先级和频率的契约归 IsOS 所有。

**算法与故障：** 把选中任务与有界上下文和模型通道绑定；按当前计划验证结构化输出。模型超时、畸形回复或取消只产生失败或重试，不产生已接纳动作。


## ei.cognition.decision.planner@v0.2

结合选中的任务和上下文快照，产生原语提案与规划状态。 [节点接口](../../cognition/decision_cn.md#cognition_decision_cn--planner)
