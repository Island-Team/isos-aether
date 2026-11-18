# Perception node design

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/design/evidence</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../../docs_cn/ei/interfaces/node/design/evidence/perception.md)

## Registered node behavior

**Inputs:** resource, result, optional hearing. **Outputs:** observations. The [stage dataflow design](../../../../core/runtime/pipeline.md) describes EI runtime values. IsOS owns the graph port and setting contract, including wire priority and frequency.

**Algorithm and failure:** Validate source identity and event shape; preserve event order, coalesce replaceable state by key, and count bounded omissions. Malformed evidence is rejected and logged, never converted into an empty success.


## ei.evidence.perception.admission@v0.2

Admits a verified resource bundle as observations for cognition. [Node interface](../../evidence/perception.md#evidence_perception--admission)
