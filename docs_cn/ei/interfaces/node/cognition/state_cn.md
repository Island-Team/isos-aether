<a id="cognition_state_cn"></a>
<a id="cognition_state_cn--cognition--state"></a>
# Cognition / state

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/cognition</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [Cognition / context](context_cn.md)
  - [Cognition / decision](decision_cn.md)
  - [Cognition / intent](intent_cn.md)
  - [Cognition / missions](missions_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/ei/interfaces/node/cognition/state.md#cognition_state) · [层目录](../../../interfaces_cn.md#readme_cn--cognition) · [EI 节点边界](../../../../isos/interfaces/server/ei_cn.md#node_cn)

[共享节点契约](../../../../node/interfaces/README_cn.md)定义类型身份、设置、端口、路由和注册规则；EI 负责本组节点的行为。本目录说明该组节点的 EI 行为，运行职责见[组设计](../design/cognition/state.md)。

<a id="cognition_state_cn--arm-status"></a>
<a id="arm-status"></a>

<a id="cognition_state_cn--eicognitionstatearmstatusv02"></a>
## ei.cognition.state.arm_status@v0.2

通过 EI 状态阶段传递机械臂监控观测。
