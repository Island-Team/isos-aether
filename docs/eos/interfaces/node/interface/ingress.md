<a id="interface_ingress"></a>
<a id="interface_ingress--interfaceingress-group"></a>
# interface/ingress group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/interface</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [interface/egress group](egress.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/interface/ingress_cn.md#interface_ingress_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [trigger](ingress.md#interface_ingress--trigger)

<a id="interface_ingress--trigger"></a>
<a id="trigger"></a>

<a id="interface_ingress--eosinterfaceingresstriggerv02"></a>
## eos.interface.ingress.trigger@v0.2

Built-in node type in the current IsOS node library.

<a id="interface_ingress--node-behavior"></a>
### Node behavior

Receives the run-fenced EI activation bridge.

- Input: `activation`: `button_activation@v0.2`, EI Unix socket.
- Output: `activation`: `button_activation@v0.2`, in process.

<a id="interface_ingress--owner-and-lifecycle"></a>
### Owner and lifecycle

The EOS owner checks active run identity before forwarding the event into the local demo chain.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [arm-control](../../../interfaces.md#arm_control).
