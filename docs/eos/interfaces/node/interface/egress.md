<a id="interface_egress"></a>
<a id="interface_egress--interfaceegress-group"></a>
# interface/egress group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/interface</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [interface/ingress group](ingress.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/interface/egress_cn.md#interface_egress_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [arm_status](egress.md#interface_egress--arm-status)
- [vision](egress.md#interface_egress--vision)
- [vision_flow](egress.md#interface_egress--vision-flow)

<a id="interface_egress--arm-status"></a>
<a id="arm-status"></a>

<a id="interface_egress--eosinterfaceegressarmstatusv02"></a>
## eos.interface.egress.arm_status@v0.2

Built-in node type in the current IsOS node library.

<a id="interface_egress--node-behavior"></a>
### Node behavior

Publishes checked arm status to EI.

- Input: `observation`: `arm_monitor_state@v0.2`, in process.
- Output: `observation`: `arm_monitor_state@v0.2`, EI Unix socket.

<a id="interface_egress--owner-and-lifecycle"></a>
### Owner and lifecycle

The bridge publishes bounded, run-fenced observations; Stop ends the live publication.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [arm-monitor](../../../interfaces.md#arm_monitor).

<a id="interface_egress--vision"></a>
<a id="vision"></a>

<a id="interface_egress--eos-v2-vision-interface"></a>
## EOS v2 Vision interface

<a id="interface_egress--graph-vision-transfer-graph-021"></a>
### Graph Vision transfer (graph 0.2.1)

`eos.interface.egress.vision_flow@v0.2` is an EOS-owned standard interface node. Its required `frame` input consumes an `image_frame@v0.2` in-process event with data class `image` from a declared camera device. Its `frame` output exposes the same type to an IsOS UI Vision monitor over a run-fenced Unix socket. The camera node selects its service and device source. Apply and Run validate the camera-to-transfer edge.

The EOS owner admits `vision_subscribe` only for the active run, revision, and exact transfer-to-camera route. It returns the run-scoped worker socket and a separate 64-hex admission token. IsOS subscribes on that socket with `{op:"vision_subscribe",run_id,revision,node,vision_token}`. The native worker pushes `[u32 big-endian JSON header length][JSON header][u32 big-endian JPEG length][JPEG bytes]` for each new capture. The header carries run, revision, source path, output sequence, sample, dimensions, and `encoding:"jpeg"`. Header and JPEG bounds are 4096 bytes and 20 MiB; the worker allows at most four subscribers and disconnects a slow partial writer after 150 ms. Stop closes the worker and its subscriptions. The control socket remains available while frames are pushed.

The source output port declares its expected frequency. A destination input port can declare a desired range; a processing node declares its own output frequency. The graph Vision transfer does not impose a separate default capture or rendition limit. Payload bounds and run fencing remain enforced.

The pushed data lane currently accepts JPEG frames. The demo's MuJoCo camera emits JPEG at its native 1280×720 setting. Other `image_frame@v0.2` encodings remain part of the data type but are not delivered through this binary Vision subscription.

<a id="interface_egress--vision-flow"></a>
<a id="vision-flow"></a>

<a id="interface_egress--eosinterfaceegressvisionflowv02"></a>
## eos.interface.egress.vision_flow@v0.2

Built-in node type in the current IsOS node library.

<a id="interface_egress--node-behavior-1"></a>
### Node behavior

Transfers camera frames to IsOS Vision.

- Input: `frame`: `image_frame@v0.2`, in-process event.
- Output: `frame`: `image_frame@v0.2`, Unix-socket stream.

<a id="interface_egress--owner-and-lifecycle-1"></a>
### Owner and lifecycle

Only the graph-declared camera source may feed this bridge. EOS validates each frame and binds transfer to the active run; Stop ends the stream.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [vision](egress.md#interface_egress--eosinterfaceegressvisionflowv02).
