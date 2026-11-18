# Context node design

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/design/cognition</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Decision node design](decision.md)
  - [Cognition / intent](intent.md)
  - [Missions node design](missions.md)
  - [Cognition / state](state.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../../docs_cn/ei/interfaces/node/design/cognition/context.md)

## Registered node behavior

**Inputs:** observations, optional body and memory. **Outputs:** snapshot. The [stage dataflow design](../../../../core/runtime/pipeline.md) describes EI runtime values. IsOS owns the graph port and setting contract, including wire priority and frequency.

**Algorithm and failure:** Read immutable snapshots, order sections deterministically, enforce one total token budget, and state omissions. Missing optional sources produce absent sections. Stop discards pending rendering without changing source evidence.


## ei.cognition.context.builder@v0.2

Builds a context snapshot from admitted observations for the planner. [Node interface](../../cognition/context.md#cognition_context--builder)
