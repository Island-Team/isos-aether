# IsOS UI Vision group

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/node/ui</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS UI button group](button.md)
  - [IsOS UI monitor group](monitor.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../docs_cn/isos/interfaces/node/ui/vision_cn.md) · [Node index](../README.md) · [Vision stream](../../api/client.md#vision)

`isos.ui.vision.monitor@v0.2` is an IsOS-owned read-only node at `isos/ui/vision/<id>`. Its required `frame` input has type `image_frame@v0.2` and connects through `eos.interface.egress.vision_flow@v0.2` to a compatible EOS camera. The camera owns source selection; the EOS interface node transfers the frame. Apply and Run reject an incomplete or unrouted path.

A subscription is bound to one active monitor path, run ID, saved revision, transfer path, and source path. Stop or any changed binding invalidates the stream. The [Vision client contract](../../api/client.md#vision) defines the text subscription and binary JPEG frame envelope.
