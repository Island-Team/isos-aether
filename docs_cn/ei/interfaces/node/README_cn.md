# EI 节点接口

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node</summary>

浏览本目录的文档及下级目录。

- [上级目录](../)
- 下级目录：
  - [cognition/](cognition/)
  - [evidence/](evidence/)
  - [governance/](governance/)
  - [io/](io/)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/ei/interfaces/node/README.md) · [EI 接口](../../interfaces_cn.md) · [共享节点契约](../../../node/interfaces/README_cn.md)

EI 发布以下 `<layer>/<group>` 节点契约。每个分组固定类型身份、设置、类型化端口和所有者行为。[共享节点契约](../../../node/interfaces/README_cn.md)定义配置位置、数据描述符、连线及注册表准入。IsOS 只向 EI 提供选定的图投影和精确的已验证软件包引用；EI 在激活事件到达 EOS 前验证路线并强制执行安全门。

| 层 | 分组 |
| --- | --- |
| `io` | [input](io/input_cn.md)、[link](io/link_cn.md) |
| `evidence` | [perception](evidence/perception_cn.md) |
| `cognition` | [context](cognition/context_cn.md)、[decision](cognition/decision_cn.md)、[intent](cognition/intent_cn.md)、[missions](cognition/missions_cn.md)、[state](cognition/state_cn.md) |
| `governance` | [monitor](governance/monitor_cn.md)、[safety](governance/safety_cn.md) |

[隔离认知库](../../interfaces_cn.md#cognition_isolated_library_cn)是没有图路线的直接调用接口。[IsOS–EI 服务端接口](../../../isos/interfaces/server/ei_cn.md)定义设置与受运行围栏约束的所有者交换。
