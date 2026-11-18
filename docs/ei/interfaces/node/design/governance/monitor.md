# Governance / monitor

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/design/governance</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Safety node design](safety.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../../docs_cn/ei/interfaces/node/design/governance/monitor.md) · [Governance layer](../../../../core/README.md#governance)

Monitor nodes carry read-only health and arm-status observations through the selected EI projection. They do not submit actuator commands or change EOS admission. The [node interfaces](../../governance/monitor.md#governance_monitor) identify the registered monitor stages; the [EI–EOS interface](../../../../core/runtime/io.md#southbound-operation-handling) owns the owner exchange.
