# 共享节点接口

<!-- doc-navigation:start -->
<details>
<summary>目录：node/interfaces</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [共享节点图接口](graph_cn.md)
  - [共享节点注册表接口](registry_cn.md)
  - [共享节点类型接口](type_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/node/interfaces/README.md) · [IsOS 节点分组](../../isos/interfaces/node/README_cn.md)

注册节点类型固定位置、设置、类型化端口、实现和生命周期。保存的实例提供身份、精确软件包固定版本、设置及输入引用。IsOS 组合所选所有者目录并在 Apply 前验证整张图。图格式 `0.2.1`、节点类型身份、软件包版本和所有者协议分别演进。

- [图声明](graph_cn.md)定义配置形态、实例和连线。
- [类型与载荷](type_cn.md)定义注册类型、端口、数据描述符和选择器。
- [注册表与准入](registry_cn.md)定义软件包声明、所有者能力、实现形态和路线证明。

`contract/node/` 中的已编写模式定义注册类型与保存实例的形态。IsOS 在 `contract/` 统一维护共享载荷和服务契约；EI 与 EOS 只读消费选定定义。声明的服务绑定携带精确身份；IsOS 仅为选定运行解析端点。可增加可选字段而不改变已有图语义；改变既有必填字段或线协议身份需要显式版本更新。
