# Missions node design

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/design/cognition</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Context node design](context.md)
  - [Decision node design](decision.md)
  - [Cognition / intent](intent.md)
  - [Cognition / state](state.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../../docs_cn/ei/interfaces/node/design/cognition/missions.md)

## Registered node behavior

**Inputs:** intent, optional result. **Outputs:** selected, lifecycle. The [stage dataflow design](../../../../core/runtime/pipeline.md) describes EI runtime values. IsOS owns the graph port and setting contract, including wire priority and frequency.

**Algorithm and failure:** Validate identity, revision and queue capacity; checkpoint accepted transitions, then select eligible work by mission policy. Empty queue waits. Stale revisions fail; shutdown persists terminal or suspended state.


## ei.cognition.missions.queue@v0.2

Selects the mission from owner-injected intent and exposes lifecycle state. [Node interface](../../cognition/missions.md#cognition_missions--queue)
