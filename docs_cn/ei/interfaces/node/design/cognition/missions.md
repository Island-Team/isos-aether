# missions 节点设计

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/design/cognition</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [context 节点设计](context.md)
  - [decision 节点设计](decision.md)
  - [cognition / intent 节点组](intent.md)
  - [cognition / state 节点组](state.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/ei/interfaces/node/design/cognition/missions.md)

## 注册节点行为

**输入：** intent、可选 result。**输出：** selected、lifecycle。[阶段数据流设计](../../../../core/runtime/pipeline.md)描述 EI 运行值。图端口、设置及连线优先级和频率的契约归 IsOS 所有。

**算法与故障：** 验证身份、修订与队列容量；持久化已接受状态变化后按任务策略选取工作。空队列等待；过期修订失败；关闭时保存终结或暂停状态。


## ei.cognition.missions.queue@v0.2

从所有者注入的意图选择任务，并提供生命周期状态。 [节点接口](../../cognition/missions_cn.md#cognition_missions_cn--queue)
