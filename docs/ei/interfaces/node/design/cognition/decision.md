# Decision node design

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/design/cognition</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Context node design](context.md)
  - [Cognition / intent](intent.md)
  - [Missions node design](missions.md)
  - [Cognition / state](state.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../../docs_cn/ei/interfaces/node/design/cognition/decision.md)

## Registered node behavior

**Inputs:** selected, snapshot. **Outputs:** proposal, plan_state. The [stage dataflow design](../../../../core/runtime/pipeline.md) describes EI runtime values. IsOS owns the graph port and setting contract, including wire priority and frequency.

**Algorithm and failure:** Bind the selected mission to a bounded context and model lane; validate structured output against the current plan. A model timeout, malformed reply, or canceled call yields failure or retry policy, never an admitted action.


## ei.cognition.decision.planner@v0.2

Combines the selected mission and context snapshot into a primitive proposal and plan state. [Node interface](../../cognition/decision.md#cognition_decision--planner)
