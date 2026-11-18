<a id="cognition_context"></a>
<a id="cognition_context--cognition--context"></a>
# Cognition / context

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/cognition</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Cognition / decision](decision.md)
  - [Cognition / intent](intent.md)
  - [Cognition / missions](missions.md)
  - [Cognition / state](state.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/ei/interfaces/node/cognition/context_cn.md#cognition_context_cn) · [Layer](../../../interfaces.md#readme--cognition) · [EI node boundary](../../../../isos/interfaces/server/ei.md#node)

The [Shared node contracts](../../../../node/interfaces/README.md) define type identity, settings, ports, routes, and registry rules; EI owns this group’s node behavior. This catalog describes the EI behavior for the nodes in this group; the [group design](../../../core/cognition/context.md) explains their runtime roles.

<a id="cognition_context--builder"></a>
<a id="builder"></a>

<a id="cognition_context--eicognitioncontextbuilderv02"></a>
## ei.cognition.context.builder@v0.2

Builds a context snapshot from admitted observations for the planner.
