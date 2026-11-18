<a id="cognition_intent"></a>
<a id="cognition_intent--cognition--intent"></a>
# Cognition / intent

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/cognition</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Cognition / context](context.md)
  - [Cognition / decision](decision.md)
  - [Cognition / missions](missions.md)
  - [Cognition / state](state.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/ei/interfaces/node/cognition/intent_cn.md#cognition_intent_cn) · [Layer](../../../interfaces.md#readme--cognition) · [EI node boundary](../../../../isos/interfaces/server/ei.md#node)

The [Shared node contracts](../../../../node/interfaces/README.md) define type identity, settings, ports, routes, and registry rules; EI owns this group’s node behavior. This catalog describes the EI behavior for the nodes in this group; the [group design](../../../core/cognition/intent.md) explains their runtime roles.

<a id="cognition_intent--context"></a>
<a id="context"></a>

<a id="cognition_intent--eicognitionintentcontextv02"></a>
## ei.cognition.intent.context@v0.2

Provides the activation context in the demo control path.

<a id="cognition_intent--forwarder"></a>
<a id="forwarder"></a>

<a id="cognition_intent--eicognitionintentforwarderv02"></a>
## ei.cognition.intent.forwarder@v0.2

Forwards the activation event through the EI owner’s selected native stage.
