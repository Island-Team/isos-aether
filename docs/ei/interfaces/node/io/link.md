<a id="io_link"></a>
<a id="io_link--io--link"></a>
# Io / link

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/io</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Io / input](input.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/ei/interfaces/node/io/link_cn.md#io_link_cn) · [Layer](../../../interfaces.md#readme--io) · [EI node boundary](../../../../isos/interfaces/server/ei.md#node)

The [Shared node contracts](../../../../node/interfaces/README.md) define type identity, settings, ports, routes, and registry rules; EI owns this group’s node behavior. This catalog describes the EI behavior for the nodes in this group; the [group design](../../../core/io/link.md) explains their runtime roles.

<a id="io_link--gateway"></a>
<a id="gateway"></a>

<a id="io_link--eiiolinkgatewayv02"></a>
## ei.io.link.gateway@v0.2

Receives owner-injected resource, health, and operation-result records. Direct `GatewayStage` input accepts only an object with `owner:"eos"`, an offset-aware `source_time` within five minutes of the current time (and no more than one second in the future). When `GatewayStage` is constructed with a selected `run_id`, the input must match it; without one, run matching is disabled. The complete JSON record is limited to 65,536 bytes. A valid record is emitted unchanged on its typed port. Invalid owner, run, time, size, or stopped state rejects; reads do not cause an EOS operation. The graph has no new bridge route for these isolated records.
