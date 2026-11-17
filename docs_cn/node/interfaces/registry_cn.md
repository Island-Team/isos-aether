# 共享节点注册表接口

<!-- doc-navigation:start -->
<details>
<summary>目录：node/interfaces</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [共享节点接口](README_cn.md)
  - [共享节点图接口](graph_cn.md)
  - [共享节点类型接口](type_cn.md)

</details>
<!-- doc-navigation:end -->

<a id="authoring"></a>
<a id="authoring--节点声明规则"></a>
## 节点声明规则

[English](../../../docs/node/interfaces/registry.md#authoring) · [配置形态](graph_cn.md#profile) · [类型](type_cn.md#type)

图格式 `0.2.1` 恰好要求 `isos`、`ei`、`eos` 三个核心。每个注册 ID 遵循 `<core>.<layer>.<group>.<type>@v0.2`；其实例必须位于相同核心、层和组。[起始图](../../../crates/isos/templates/generic-graph.toml)展示了代表性的声明。

编写图之前，先选择当前可执行的路径：

| 核心 | 支持的贡献实现 | 当前图限制 |
| --- | --- | --- |
| IsOS | `isos.ui.external.v1`、版本 `1`、`unix-socket-v1` 直属子进程；[请求/应答契约](../../isos/interfaces/node/README_cn.md) | 声明的 UI 端口支持独立输入和读取；连接此节点的图边会被拒绝。 |
| EI | Linux `isos_node_v1` `.so` 或 `ei.owner.stage.v1` 外部工作进程；[EI 节点契约](../../isos/interfaces/server/ei_cn.md#node_cn) | 执行器交付仍必须经过 EI 安全门。 |
| EOS | 软件节点使用 Linux `isos_node_v1` `.so`，服务绑定适配器使用 `isos_node_v2`；[EOS 贡献契约](../../isos/interfaces/server/eos_cn.md#node_contributions_cn) | 完整的模拟机械臂演示链可与贡献的观测及软件节点沿已注册路线同时运行。跨软件包执行器命令会被拒绝；服务绑定的 v2 执行器需要声明的 Dispatcher 路由。v2 ABI 没有类型化软件输入回调。 |

贡献的类型按[类型接口](type_cn.md#type)声明带版本的数据类型和精确端口。已验证的[软件包发行版](../../isos/interfaces/package/README_cn.md#release)列出该类型、原生库或外部进程，以及所有者确能交付的路线。实例在 `<core>/<layer>/<group>/<id>` 路径下使用一致的映射键和 `id`、精确的 `{id,version}` 软件包固定版本、允许的设置和必需输入引用。IsOS 拒绝缺少已验证实现、位置或端口不符、以及未声明的路线。

[图 API](../../isos/interfaces/api/http_cn.md#http_graph)定义修订准入；[注册表](registry_cn.md#registry)定义软件包和所有者声明。创建、验证及运行命令见[节点首次使用指南](../../isos/start/quick-start_cn.md)。

起始图声明类型化控制与观测连接。激活事件经 EI 到达 EOS；EOS 将其转换为有界的六关节增量并传给模拟适配器。同一适配器的观测状态经 EOS 和 EI 异步返回 UI 监控节点。混合示例的控制路径缺少激活源，还将软件包相机帧经 EOS 视觉流送到 UI 视觉监控。未知类型、错误位置、不兼容端口会导致准入失败；缺少必需输入会产生警告。模拟适配器没有实体执行器路由。

混合演示图的 Vision 链为 `front_camera` → `front_vision_flow` → `front_monitor`。MuJoCo 相机保持 1280×720、30 Hz 设置并输出 JPEG。两个下游期望速率上限均为 32 Hz，以便在毫秒级交付间隔下通过 30 Hz 采集。这些设置并非浏览器实测速率的保证。

断开的必需输入保留在所编辑的图中。Apply 可以保存该图；重新连接输入并再次 Apply 之前，受影响的路径不能交付数据。

输入端口可以将来源包装为 `{"sources":{"node":"<source-path>","output":"<port>"},"desired_rate_hz":{"min":10,"max":30}}`。两个速率边界均为可选的有限正数，单位为 Hz；省略包装表示端口无上限。IsOS 从 `max` 推导交付上限；如果注册的来源输出可选的官方 `frequency_hz` 小于 `min`，则发出警告。最小值只是期望，不会生成帧。边的 `frequency_hz` 不再由图作者设置。来源可以省略软件包输出端口的 `frequency_hz`；数据继续传递，直到后续输出端口声明官方频率时才进行频率比较。若来源采集频率可配置，应在图实例中将 `node.adapter.settings.frequency_hz` 设为 `(0, 1000]` Hz 范围内的数值。v1 或 v2 原生 Vision 来源必须设置此值，才能采集画面并写入来源池；软件包的官方频率本身不会调度采集，缺少实例设置时不会生成池中画面。此自定义适配器设置控制来源，但不会成为软件包的官方频率。旧软件包中的 `expected_rate_hz` 声明须迁移为 `frequency_hz`。

<a id="registry"></a>
<a id="registry--自定义节点注册表v023"></a>
## 自定义节点注册表（v0.2.3）

[English](../../../docs/node/interfaces/registry.md#registry) · [节点接口](../../isos/interfaces/node/README_cn.md) · [软件包发布](../../isos/interfaces/package/README_cn.md#release)

IsOS 根据自身的 UI 声明及所选 EI、EOS 所有者目录，组合针对图的注册表。软件包通过 `node_contributions` 提交声明，并由精确的 `{id,version}` 固定版本选择已验证字节。注册本身不授予执行权限。图准入前，跨核心路由必须获得相关所有者的目录声明。

注册类型可选的 `interaction_roles` 描述其通用通信行为：`publisher`（发布者）、`subscriber`（订阅者）、`service`（服务）或 `action`（动作）。一个类型可以声明多个角色。这些角色便于阅读图，也供 Web UI 区分节点卡片；它们不能替代精确端口声明、所有者路由声明或 Dispatcher 准入。未声明角色的目录仍有效；UI 可以从实时输出或输入端口推断发布者或订阅者。服务和动作必须显式声明。

<a id="registry--软件包与核心注册边界"></a>
### 软件包与核心注册边界

软件包管理器是已安装发行包、已验证字节、贡献节点类型、数据类型、资源、进程和精确固定版本的唯一来源。IsOS 仅向核心的设置接口传入该核心的图投影和已验证软件包引用。核心通过目录与验证操作返回受支持的类型、路由、适配器和诊断。IsOS 按完整类型 ID 和精确软件包固定版本合并声明，核对图中每条连接的两端所有者，并在所有选定所有者接受后发布图。软件包名、组名、可执行文件名和硬件实现均不选择特殊注册路径。未知软件包采用相同准入顺序；安装或链接并验证发行包之前，其内容保持未知。

图格式 `0.2.1` 保留 `isos`、`ei`、`eos` 三个核心和现有的 `@v0.2` 类型标识。软件包可以在其中一个核心下选择层和分组名称；类型 ID 必须与声明的位置一致。每个类型固定位置、设置、端口、载体和实现。实例在对应位置选择类型，并提供值及精确的软件包固定版本。非模拟 EOS 硬件类型由软件包声明，不再内建。重复的类型或数据标识、错误的位置、未声明的库、变化的字节、不受支持的 ABI 和不可用的所有者实现均拒绝准入。

<a id="registry--实现形式"></a>
### 实现形式

`implementation.kind: "native_plugin"` 选择 Linux `isos_node_v1` 或 `isos_node_v2`，并在 `node_contributions.libraries` 中以 SHA-256 声明一次的软件包内安全相对 `.so` 路径。所属进程加载该库，并负责其内存、线程、生命周期和 Stop 证明。[v1 C 头文件](../../../contract/node/isos_node_v1.h)和 [v2 C 头文件](../../../contract/node/isos_node.h)定义 ABI。所有调用返回有界状态；主机在回调返回前复制输出字节。非零 Stop 结果不能证明执行器已静止。

`implementation.kind: "external"` 指定版本 `1` 的所有者接口和生命周期适配器。所有者验证接口，启动或连接所声明的工作进程，将其绑定到所选运行和修订，并在报告完成前证明 Stop。接口仅供所有者使用；节点声明不能提供任意可执行命令或执行器路由。


<a id="registry--数据和载体"></a>
### 数据和载体

[类型接口](type_cn.md#type)定义供读者理解的节点元数据；注册的[数据类型](type_cn.md#data_types) 包含 JSON Schema、编解码器、媒体类型和字节上限。实时端口精确选择一种 `in_process` 或 `unix_socket` 载体、交付模式、数据类型和 `{owner,id,version}` 协议契约。`none` 仅用于静态声明。所有者通告精确的 `port_routes` 和 `bridge_adapters`；端口名称匹配本身不授权交付。Unix socket 信封包含运行 ID、图修订、源节点、输出端口、数据类型和有界载荷。IsOS 及两端所有者在跨界时检查这些绑定。
