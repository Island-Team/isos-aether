# interface/egress group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/design/interface</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [interface/ingress group](ingress.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../../docs_cn/eos/interfaces/node/design/interface/egress_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.interface.egress.arm_status@v0.2

Publishes checked arm status to EI. Input: `observation`: `arm_monitor_state@v0.2`, in process. Output: `observation`: `arm_monitor_state@v0.2`, EI Unix socket. The bridge publishes bounded, run-fenced observations; Stop ends the live publication. Interface: [arm-monitor](../../../../interfaces.md#arm_monitor).

## eos.interface.egress.vision_flow@v0.2

Transfers camera frames to IsOS Vision. Input: `frame`: `image_frame@v0.2`, in-process event. Output: `frame`: `image_frame@v0.2`, Unix-socket stream. Only the graph-declared camera source may feed this bridge. EOS validates each frame and binds transfer to the active run; Stop ends the stream. Interface: [vision](../../interface/egress.md#interface_egress--vision).
