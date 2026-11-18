# interface/ingress group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/design/interface</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [interface/egress group](egress.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../../docs_cn/eos/interfaces/node/design/interface/ingress_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.interface.ingress.trigger@v0.2

Receives the run-fenced EI activation bridge. Input: `activation`: `button_activation@v0.2`, EI Unix socket. Output: `activation`: `button_activation@v0.2`, in process. The EOS owner checks active run identity before forwarding the event into the local demo chain. Interface: [arm-control](../../../../interfaces.md#arm_control).
