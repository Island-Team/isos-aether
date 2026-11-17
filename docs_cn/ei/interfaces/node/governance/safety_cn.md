<a id="governance_safety_cn"></a>
<a id="governance_safety_cn--governance--safety"></a>
# Governance / safety

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/governance</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [Governance / monitor](monitor_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/ei/interfaces/node/governance/safety.md#governance_safety) · [层目录](../../../interfaces_cn.md#readme_cn--governance) · [EI 节点边界](../../../../isos/interfaces/server/ei_cn.md#node_cn)

[共享节点契约](../../../../node/interfaces/README_cn.md)定义类型身份、设置、端口、路由和注册规则；EI 负责本组节点的行为。本目录说明该组节点的 EI 行为，运行职责见[组设计](../../../core/governance/safety.md)。

<a id="governance_safety_cn--gate"></a>
<a id="gate"></a>

<a id="governance_safety_cn--eigovernancesafetygatev02"></a>
## ei.governance.safety.gate@v0.2

在 EOS 准入前对激活路径应用 EI 安全门。
