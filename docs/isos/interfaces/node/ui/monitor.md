# IsOS UI monitor group

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/node/ui</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS UI button group](button.md)
  - [IsOS UI Vision group](vision.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../docs_cn/isos/interfaces/node/ui/monitor_cn.md) · [Node index](../README.md) · [Client monitor](../../api/client.md#monitor)

`isos.ui.monitor.arm@v0.2` is an IsOS-owned read-only node at `isos/ui/monitor/<id>`. Its required `observation` input carries `arm_monitor_state@v0.2` from the declared EOS observation and EI governance path. It does not produce an actuator command or infer physical success from a submitted button event.

A read requires the active run, saved revision, and complete admitted return chain. A ready observation has six finite joint positions in radians; degraded, stale, and stopped observations have no positions and include a reason. The [monitor client contract](../../api/client.md#monitor) defines the read route, freshness, and failure states.
