<a id="cognition_missions_cn"></a>
<a id="cognition_missions_cn--cognition--missions"></a>
# Cognition / missions

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/cognition</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [Cognition / context](context_cn.md)
  - [Cognition / decision](decision_cn.md)
  - [Cognition / intent](intent_cn.md)
  - [Cognition / state](state_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/ei/interfaces/node/cognition/missions.md#cognition_missions) · [层目录](../../../interfaces_cn.md#readme_cn--cognition) · [EI 节点边界](../../../../isos/interfaces/server/ei_cn.md#node_cn)

[共享节点契约](../../../../node/interfaces/README_cn.md)定义类型身份、设置、端口、路由和注册规则；EI 负责本组节点的行为。本目录说明该组节点的 EI 行为，运行职责见[组设计](../../../core/cognition/missions.md)。

<a id="cognition_missions_cn--queue"></a>
<a id="queue"></a>

<a id="cognition_missions_cn--eicognitionmissionsqueuev02"></a>
## ei.cognition.missions.queue@v0.2

直接调用 `QueueStage` 时，它选择待处理意图中优先级最高的任务。意图要求稳定的 `mission_id`（1–128 字符）、非空 `text`（最多 4096 字符），以及可选的 −100 到 100 整数 `priority`。直接调用方可设置 1–256 的容量，默认八。重复 ID 和容量耗尽均被拒绝。结果必须指向队列中的任务，状态只能为 `completed`、`failed` 或 `cancelled`；处理后输出生命周期记录并选择下一任务。相同优先级保持插入顺序。Stop 清空本地队列并关闭输入。此直接处理器不新增图路由或持久任务存储。
