<a id="cognition_missions"></a>
<a id="cognition_missions--cognition--missions"></a>
# Cognition / missions

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/cognition</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Cognition / context](context.md)
  - [Cognition / decision](decision.md)
  - [Cognition / intent](intent.md)
  - [Cognition / state](state.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/ei/interfaces/node/cognition/missions_cn.md#cognition_missions_cn) · [Layer](../../../interfaces.md#readme--cognition) · [EI node boundary](../../../../isos/interfaces/server/ei.md#node)

The [Shared node contracts](../../../../node/interfaces/README.md) define type identity, settings, ports, routes, and registry rules; EI owns this group’s node behavior. This catalog describes the EI behavior for the nodes in this group; the [group design](../../../core/cognition/missions.md) explains their runtime roles.

<a id="cognition_missions--queue"></a>
<a id="queue"></a>

<a id="cognition_missions--eicognitionmissionsqueuev02"></a>
## ei.cognition.missions.queue@v0.2

Direct `QueueStage` invocation selects the highest-priority pending intent. An intent requires a stable `mission_id` (1–128 characters), nonempty `text` (at most 4096 characters), and optional integer `priority` from −100 to 100. Capacity defaults to eight and may be set from 1 to 256 by its direct caller. Duplicate IDs and exhausted capacity reject. A result must name a queued mission and have `completed`, `failed`, or `cancelled` state; it emits a lifecycle record and selects the next mission. Equal priorities retain insertion order. Stop clears the local queue and closes input. This direct handler does not add a graph route or durable mission store.
