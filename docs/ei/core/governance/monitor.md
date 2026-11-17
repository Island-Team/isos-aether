# Governance / monitor

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/core/governance</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [safety](safety.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/ei/core/governance/monitor.md) · [Governance layer](../README.md#governance)

Monitor nodes carry read-only health and arm-status observations through the selected EI projection. They do not submit actuator commands or change EOS admission. The [node interfaces](../../interfaces/node/governance/monitor.md#governance_monitor) identify the registered monitor stages; the [EI–EOS interface](../runtime/io.md#southbound-operation-handling) owns the owner exchange.
