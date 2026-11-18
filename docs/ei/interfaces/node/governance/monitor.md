<a id="governance_monitor"></a>
<a id="governance_monitor--governance--monitor"></a>
# Governance / monitor

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/governance</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Governance / safety](safety.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/ei/interfaces/node/governance/monitor_cn.md#governance_monitor_cn) · [Layer](../../../interfaces.md#readme--governance) · [EI node boundary](../../../../isos/interfaces/server/ei.md#node)

The [Shared node contracts](../../../../node/interfaces/README.md) define type identity, settings, ports, routes, and registry rules; EI owns this group’s node behavior. This catalog describes the EI behavior for the nodes in this group; the [group design](../../../core/governance/monitor.md) explains their runtime roles.

<a id="governance_monitor--arm-status"></a>
<a id="arm-status"></a>

<a id="governance_monitor--eigovernancemonitorarmstatusv02"></a>
## ei.governance.monitor.arm_status@v0.2

Forwards bounded arm monitor observations to the IsOS UI route.

<a id="governance_monitor--health"></a>
<a id="health"></a>

<a id="governance_monitor--eigovernancemonitorhealthv02"></a>
## ei.governance.monitor.health@v0.2

Turns health updates into status and latch state.
