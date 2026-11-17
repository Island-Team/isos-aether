<a id="interface_ingress_cn"></a>
<a id="interface_ingress_cn--interfaceingress-分组"></a>
# interface/ingress 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/interface</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [interface/egress 分组](egress_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/interface/ingress.md#interface_ingress)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [trigger](ingress_cn.md#interface_ingress_cn--trigger)

<a id="interface_ingress_cn--trigger"></a>
<a id="trigger"></a>

<a id="interface_ingress_cn--eosinterfaceingresstriggerv02"></a>
## eos.interface.ingress.trigger@v0.2

当前 IsOS 节点库中的内置节点类型。

<a id="interface_ingress_cn--节点作用"></a>
### 节点作用

接收带运行围栏的 EI 激活桥接。

- 输入：`activation`：`button_activation@v0.2`，EI Unix socket。
- 输出：`activation`：`button_activation@v0.2`，进程内。

<a id="interface_ingress_cn--所有者与生命周期"></a>
### 所有者与生命周期

EOS 所有者检查当前运行身份后，才把事件转发至本地演示链。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [arm-control_cn](../../../interfaces_cn.md#arm_control_cn)。
