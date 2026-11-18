# context 节点设计

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/design/cognition</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [decision 节点设计](decision.md)
  - [cognition / intent 节点组](intent.md)
  - [missions 节点设计](missions.md)
  - [cognition / state 节点组](state.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/ei/interfaces/node/design/cognition/context.md)

## 注册节点行为

**输入：** observations、可选 body 与 memory。**输出：** snapshot。[阶段数据流设计](../../../../core/runtime/pipeline.md)描述 EI 运行值。图端口、设置及连线优先级和频率的契约归 IsOS 所有。

**算法与故障：** 读取不可变快照，确定性排序区段，执行单一 token 总预算并说明遗漏。可选来源缺失时省略区段。停止时丢弃待渲染工作，不改变来源证据。


## ei.cognition.context.builder@v0.2

将已准入的观测整理成供规划器使用的上下文快照。 [节点接口](../../cognition/context_cn.md#cognition_context_cn--builder)
