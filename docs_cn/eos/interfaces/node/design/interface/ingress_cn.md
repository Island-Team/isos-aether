# interface/ingress 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/design/interface</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [interface/egress 分组](egress_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/eos/interfaces/node/design/interface/ingress.md)

以下概述当前 EOS 节点类型及其运行时职责。准确载荷与拒绝规则见所链接的所有者接口。

## eos.interface.ingress.trigger@v0.2

接收带运行围栏的 EI 激活桥接。 输入：`activation`：`button_activation@v0.2`，EI Unix socket。 输出：`activation`：`button_activation@v0.2`，进程内。 EOS 所有者检查当前运行身份后，才把事件转发至本地演示链。 接口：[arm-control_cn](../../../../interfaces_cn.md#arm_control_cn)。
