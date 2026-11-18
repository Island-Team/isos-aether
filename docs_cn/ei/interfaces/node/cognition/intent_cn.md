<a id="cognition_intent_cn"></a>
<a id="cognition_intent_cn--cognition--intent"></a>
# Cognition / intent

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/cognition</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [Cognition / context](context_cn.md)
  - [Cognition / decision](decision_cn.md)
  - [Cognition / missions](missions_cn.md)
  - [Cognition / state](state_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/ei/interfaces/node/cognition/intent.md#cognition_intent) · [层目录](../../../interfaces_cn.md#readme_cn--cognition) · [EI 节点边界](../../../../isos/interfaces/server/ei_cn.md#node_cn)

[共享节点契约](../../../../node/interfaces/README_cn.md)定义类型身份、设置、端口、路由和注册规则；EI 负责本组节点的行为。本目录说明该组节点的 EI 行为，运行职责见[组设计](../design/cognition/intent.md)。

<a id="cognition_intent_cn--context"></a>
<a id="context"></a>

<a id="cognition_intent_cn--eicognitionintentcontextv02"></a>
## ei.cognition.intent.context@v0.2

在演示控制路径中提供激活上下文。

<a id="cognition_intent_cn--forwarder"></a>
<a id="forwarder"></a>

<a id="cognition_intent_cn--eicognitionintentforwarderv02"></a>
## ei.cognition.intent.forwarder@v0.2

通过 EI 所选择的本地阶段转发激活事件。
