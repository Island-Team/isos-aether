<a id="governance_monitor_cn"></a>
<a id="governance_monitor_cn--governance--monitor"></a>
# Governance / monitor

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/governance</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [Governance / safety](safety_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/ei/interfaces/node/governance/monitor.md#governance_monitor) · [层目录](../../../interfaces_cn.md#readme_cn--governance) · [EI 节点边界](../../../../isos/interfaces/server/ei_cn.md#node_cn)

[共享节点契约](../../../../node/interfaces/README_cn.md)定义类型身份、设置、端口、路由和注册规则；EI 负责本组节点的行为。本目录说明该组节点的 EI 行为，运行职责见[组设计](../design/governance/monitor.md)。

<a id="governance_monitor_cn--arm-status"></a>
<a id="arm-status"></a>

<a id="governance_monitor_cn--eigovernancemonitorarmstatusv02"></a>
## ei.governance.monitor.arm_status@v0.2

按限定速率将机械臂监控观测转发至 IsOS UI 路由。

<a id="governance_monitor_cn--health"></a>
<a id="health"></a>

<a id="governance_monitor_cn--eigovernancemonitorhealthv02"></a>
## ei.governance.monitor.health@v0.2

将健康更新转换为状态与锁存状态。
