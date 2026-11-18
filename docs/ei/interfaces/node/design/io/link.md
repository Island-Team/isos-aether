# Link node design

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/design/io</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Io / input](input.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../../docs_cn/ei/interfaces/node/design/io/link.md)

## Registered node behavior

**Inputs:** gateway; admitted and emergency_stop are governed internal routes. **Outputs:** discovery, resource, result, health. The [stage dataflow design](../../../../core/runtime/pipeline.md) describes EI runtime values. IsOS owns the graph port and setting contract, including wire priority and frequency.

**Algorithm and failure:** Resolve the IsOS-provided endpoint, discover resources and capabilities, verify exact identities, then atomically publish a generation. On disconnect mark the generation stale and block new actions; close channels after dependents stop.


## ei.io.link.gateway@v0.2

Receives the owner-injected resource bundle and health updates. [Node interface](../../io/link.md#io_link--gateway)
