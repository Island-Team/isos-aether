# 共享节点图接口

<!-- doc-navigation:start -->
<details>
<summary>目录：node/interfaces</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [共享节点接口](README_cn.md)
  - [共享节点注册表接口](registry_cn.md)
  - [共享节点类型接口](type_cn.md)

</details>
<!-- doc-navigation:end -->

<a id="profile"></a>
<a id="profile--保存图中的节点配置"></a>
## 保存图中的节点配置

[English](../../../docs/node/interfaces/graph.md#profile) · [节点接口](../../isos/interfaces/node/README_cn.md) · [实例](graph_cn.md#instance)

节点配置是保存的实例，不是第二份类型声明，也不是实时状态记录。图格式 `0.2.1` 包含 `isos`、`ei`、`eos` 三个核心。注册类型固定 `<core>.<layer>.<group>.<type>@v0.2`；配置位于匹配的 `cores.<core>.layers.<layer>.groups.<group>.nodes.<id>` 路径。映射键与 `id` 一致。

```text
cores.isos.layers.ui.groups.button.nodes.trigger
cores.ei.layers.io.groups.input.nodes.trigger
cores.ei.layers.cognition.groups.intent.nodes.forwarder
cores.ei.layers.governance.groups.safety.nodes.gate
cores.eos.layers.interface.groups.ingress.nodes.trigger
cores.eos.layers.skill.groups.demo.nodes.joint_delta
cores.eos.layers.dispatcher.groups.arm.nodes.delta
cores.eos.layers.hardware.groups.mock.nodes.arm
```

每个配置选择类型，并可提供标签、描述、已声明设置，以及注册类型具有端口时的输入引用。起始图的七条连接具有类型化输入和输出。[通用图模板](../../../crates/isos/templates/generic-graph.toml)包含完整的可部署示例。

IsOS 保存并验证图，然后渲染归属方投影。EI 和 EOS 分别验证自己的投影并监督进程。归属方状态独立报告就绪和故障。模拟硬件适配器在内存中保存六个关节位置，不连接实体硬件。

<a id="instance"></a>
<a id="instance--保存的节点实例"></a>
## 保存的节点实例

[English](../../../docs/node/interfaces/graph.md#instance) · [配置形态](graph_cn.md#profile) · [类型](type_cn.md#type)

图格式 `0.2.1` 恰有三个核心键：`isos`、`ei`、`eos`。实例位于 `cores.<core>.layers.<layer>.groups.<group>.nodes.<id>`。映射键等于 `id`；`node_type` 选择在该准确位置注册的 `<core>.<layer>.<group>.<type>@v0.2`。规范引用路径为 `<core>/<layer>/<group>/<id>`。

| 字段 | 含义 |
| --- | --- |
| `id`、`node_type` | 实例身份和注册的包装类型。 |
| `label`、`description` | 展示文本。`label` 须为非空白字符串；`description` 可省略或为空字符串。 |
| `settings` | 仅包含所选类型声明的设置。 |
| `inputs` | 指向已声明来源端口的引用。 |
| `package` | 软件包类型要求时使用的准确验证版本。 |

类型身份由完整的 `node_type` ID 与精确的 `package` `{id, version}` 锁定共同组成。两个已安装发行版可以声明相同的类型 ID 和位置；它们的设置与端口不会合并。没有匹配实例锁定的贡献类型会被拒绝。内建类型没有顶层软件包锁定。对于软件包硬件，`package` 选择已验证的类型，`adapter.package` 选择已验证的适配器和服务；EOS 核对两个锁定。只有模拟硬件类型仍为内建。

通信角色属于注册类型的可选 `interaction_roles`，不由每个保存实例重复声明。图配置不能覆盖这些角色。同时具有实时输入和输出的节点可以同时呈现为 subscriber 和 publisher；路由准入仍取决于其精确的类型化端口和归属方支持的连接。

起始图有 17 个节点和 17 条声明的输入连接，包括来自激活与机械臂观测的 EI 上下文输入。保存图不包含实时进程状态、归属方套接字路径、凭据、事件值或硬件命令。IsOS 管理修订与 Apply；EI 和 EOS 接收各自核心投影。EOS 模拟机械臂实例声明六个关节、适配器和运动限制。维护验证器的贡献者可在 [`contract/node/`](../../../contract/node/README.md) 找到机器可读的实例定义。

下面是混合图中的 IsOS 机械臂监视器实例。它位于 `cores.isos.layers.ui.groups.monitor.nodes.arm`；`inputs.observation` 指定来源节点和输出端口。`refresh_rate_hz` 是以赫兹计的显示上限，不是来源采样频率。

```json
{
  "id": "arm",
  "node_type": "isos.ui.monitor.arm@v0.2",
  "label": "Return: arm state",
  "description": "Read the latest arm state independently of movement completion",
  "settings": {
    "refresh_rate_hz": 5
  },
  "inputs": {
    "observation": {
      "node": "ei/governance/monitor/arm_status",
      "output": "observation"
    }
  }
}
```

<a id="edges"></a>
<a id="edges--图连接"></a>
## 图连接

[English](../../../docs/node/interfaces/graph.md#edges) · [端口](type_cn.md#ports)

输入引用格式为 `{"node":"core/layer/group/node","output":"out"}`，目标节点把它保存到已声明的输入名下。IsOS 检查两个实例是否存在于注册位置、端口是否匹配，以及图是否无环。如果两端都声明 `transport: "none"` 和 `delivery_mode: "declaration"`，则允许跨核心 `topology_link`；由于不跨边界传输载荷，不需要运行时桥接。

实时连接还要求两端的传输通道、投递模式、准确注册的负载 ID 和协议 `contract` 一致。两端都声明 `data_class` 时，类别不同会被 IsOS 以 `edge_data_class` 拒绝。可信归属方目录中的 `port_routes` 条目必须准确列出源节点类型与输出端口、目标节点类型与输入端口、传输通道、投递模式、负载 ID 和协议契约。跨核心 Unix 套接字桥接还要求两个端点配置均声明桥接端口，并且两个归属方均提供桥接适配器。缺少路由证明时，IsOS 拒绝连接。验证响应提供准确的 `source`、`output`、`target`、`input`，供客户端识别已准入连接。静态 `topology_link` 连接仍只是声明，不承载负载。

起始图有 17 条输入连接。按钮激活事件经过 EI 上下文与安全节点后进入 EOS；仅 EOS 将其转换为六关节增量，并准入至模拟机械臂。

内置目录仅发布演示图使用的准确实时 `port_routes`。其他实时连接需要归属方实现及匹配的路由注册。
