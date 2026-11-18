# 共享节点类型接口

<!-- doc-navigation:start -->
<details>
<summary>目录：node/interfaces</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [共享节点接口](README_cn.md)
  - [共享节点图接口](graph_cn.md)
  - [共享节点注册表接口](registry_cn.md)

</details>
<!-- doc-navigation:end -->

<a id="type"></a>
<a id="type--注册节点类型"></a>
## 注册节点类型

[English](../../../docs/node/interfaces/type.md#type) · [节点接口](../../isos/interfaces/node/README_cn.md) · [实例](graph_cn.md#instance)

注册类型固定 `core`、`layer`、`group`、设置、端口、能力和生命周期。类型 ID 为 `<core>.<layer>.<group>.<type>@v0.2`，其中 `core` 为 `isos`、`ei` 或 `eos`。实例只提供身份与已声明的值，不能改变位置或行为。Apply 时 IsOS 合并归属方目录并验证所选类型。EI 和 EOS 实现自身接口行为；图节点只是这些接口的包装。

| 字段 | 当前含义 |
| --- | --- |
| `id`、`core`、`layer`、`group` | 准确的版本化身份和保存位置。 |
| `kind`、`description`、`lifecycle` | 展示角色和归属方生命周期。 |
| `interaction_roles` | 可选通信角色：`publisher`、`subscriber`、`service` 和 `action`；一个类型可同时具有多个角色。 |
| `capabilities` | 归属方声明的功能；允许空数组。 |
| `settings` | 实例可用值和默认值。 |
| `inputs`、`outputs` | 类型化连接声明。 |
| `deprecated` | 旧版类型仍可用于已保存图，但不出现在 IsosUI 的添加菜单中。 |
| `implementation`、`service_inputs`、`service_outputs` | 可选的归属方实现和服务声明；仅声明不会启用路由。 |

起始图有 17 个节点和 17 条声明的输入连接；控制路径在转发节点之前经过 EI 上下文节点。按钮发出 `button_activation@v0.2`；EOS 技能和调度器将 `arm_joint_delta@v0.2` 传给模拟机械臂。模拟机械臂是带六个关节名称和有界运动设置的 Manipulator/Adapter 实例。准确 ID 和字段见[起始图](../../../crates/isos/templates/generic-graph.toml)。

上表定义读者需要的类型声明字段；[端口](type_cn.md#ports)和[连接](graph_cn.md#edges)说明连接方式。维护验证器的贡献者可在 [`contract/node/`](../../../contract/node/README.md) 找到机器可读的定义。

`interaction_roles` 描述注册类型参与通信的方式，不是传输、图边或权限授予。publisher 发出类型化数据，subscriber 消费数据；service 处理有界请求与应答；action 表示具有准入、进度或结果以及 Stop 语义的命令。处理节点可同时声明 publisher 和 subscriber。这四种角色适用于 IsOS、EI 与 EOS。归属方仍须分别声明准确端口、投递模式、载体和路由证明。省略角色列表时，旧类型声明仍可读取；读取方可根据实时输入与输出端口推导 subscriber 与 publisher 的展示角色，但不得根据端口名称推断 service 或 action 权限。

内建按钮类型展示位置和输出声明。输出指定负载和投递契约；实际发出的事件见[数据类型](type_cn.md#data_types)。

```json
{
  "id": "isos.ui.button.trigger@v0.2",
  "core": "isos",
  "layer": "ui",
  "group": "button",
  "kind": "software",
  "description": "Run-fenced, general-purpose one-click activation",
  "lifecycle": "internal",
  "capabilities": ["bridge"],
  "settings": [],
  "inputs": [],
  "outputs": [
    {
      "name": "activation",
      "data_type": "button_activation@v0.2",
      "required": false,
      "transport": "unix_socket",
      "delivery_mode": "event",
      "cardinality": "one",
      "data_class": "event",
      "contract": {
        "owner": "isos",
        "id": "button-activation",
        "version": "1"
      }
    }
  ]
}
```

<a id="ports"></a>
<a id="ports--节点端口"></a>
## 节点端口

[English](../../../docs/node/interfaces/type.md#ports) · [类型](type_cn.md#type) · [连接](graph_cn.md#edges)

输入端口负责接收，输出端口负责发送。实时端口通过 `transport` 声明通信通道，并声明投递模式、准确注册的负载 `data_type`，以及包含 `owner`、`id`、`version` 的协议 `contract`。两端必须一致。若两端都声明 `data_class`，类别也必须一致。IsOS 仅在归属方支持实际路由时准入连接。仅负载名称相同不能保证投递。

起始图的 17 个类型具有类型化端口。IsOS → EI 和 EI → EOS 边界使用 `unix_socket`；归属方内部连接使用 `in_process`。激活连接承载 `button_activation@v0.2`；两条 EOS 命令连接承载 `arm_joint_delta@v0.2`。

| 字段 | 含义 |
| --- | --- |
| `name` | 在本方向内唯一的端口名。 |
| `data_type` | 准确的注册负载类型；静态声明使用 `topology_link`。 |
| `data_class` | 可选语义类别：软件包端口可使用 `image`、`observation`、`command`、`event`、`state` 或 `opaque`。两端都声明时值必须一致；它不能替代准确的 `data_type`。 |
| `frequency_hz` | 可选的正有限标称端口频率，单位为赫兹；不设置运行时采样节奏。软件包声明中已废弃的 `expected_rate_hz` 会被拒绝。 |
| `required` | 准入是否要求连接。 |
| `transport` | 必需的通道：静态声明使用 `none`；实时端口使用明确支持的载体。 |
| `delivery_mode` | 静态声明使用 `declaration`；否则为准确的实时投递行为。 |
| `contract` | 实时端口必须提供 `{owner,id,version}` 协议身份；静态端口禁止提供。 |
| `cardinality` | 连接数量，通常为 `one`。 |

`none` 与 `declaration` 必须成对出现。Apply 时 IsOS 检查来源和目标声明。EI 与 EOS 分别验证自己的归属方投影。运行时健康来自归属方进程状态，不来自端口。上表定义调用方可见的端口字段；维护验证器的贡献者可使用 [`contract/node/`](../../../contract/node/README.md) 下机器可读的定义。

<a id="data_types"></a>
<a id="data_types--节点数据类型"></a>
## 节点数据类型

本契约供生产者、消费者和 IsosUI 渲染器使用。发布贡献或核心目录只声明一次数据类型；注册[端口](type_cn.md#ports)引用其准确 `id`。IsOS 在 Apply 前拒绝重复 ID、缺失的负载元数据和不匹配的端口身份。不兼容的结构或含义必须使用不同的版本化 ID。生产者拥有负载字节与授权。

| 字段 | 类型／单位 | 含义与验证 | 缺失／默认值 |
| --- | --- | --- | --- |
| `id` | 非空字符串／类型 ID | 供端口和边使用的稳定身份。软件包贡献的 ID 以 `@v<version>` 结尾。 | 必填。 |
| `version` | 非空字符串／模式版本 | 本负载含义的版本，与软件包及 HTTP 版本独立。 | 必填。 |
| `purpose` | 非空字符串／展示文字 | 解释消费者可如何使用该值。 | 必填。 |
| `media_type` | 非空字符串／MIME 类型 | 描述表现形式；`json_v1` 要求 `application/json`。消费者还必须检查编解码器及模式。 | 必填。 |
| `codec` | 非空字符串／编码 ID | 标识准确编码，例如 JSON、UTF-8、图像、像素或音频编码。 | 必填。 |
| `schema` | 有效 JSON Schema 对象／负载结构 | 归属方定义结构化值或平面的形状及单位。若存在 `x-unit`，它必须是非空字符串。 | 必填。 |
| `max_payload_bytes` | 1–100,000,000 的整数／字节 | 在呈现或传输前限制单个编码负载。 | 必填。 |

同一组字段可描述记录、文本、时间序列、图像、像素平面、深度、检测和音频。渲染器只在检查注册端口、编解码器、媒体类型、模式、大小上限及自身能力后选择方法；[UI 选择](../../isos/interfaces/api/client_cn.md#selection)说明此过程。类型名称本身不授权解码。

IsOS 在软件包验证或 Apply 时检查描述符完整性、JSON Schema 语法、单位注解和声明的字节上限。归属方依据该上限、模式和语义规则检查每个编码值，包括来源身份、新鲜度、坐标系、单位、有限数值、向量和图像尺寸以及资源可用性。准确的数据 ID 不意味着转换、传输或执行器授权。大型媒体和数据集使用有界引用；新二进制编解码器须有解码器和字节上限后才能注册。

<a id="data_types--隔离节点库与场景描述符身份"></a>
### 隔离节点库与场景描述符身份

隔离节点库保留图格式 `0.2.1`、节点类型 `@v0.2` 和起始图既有的数据含义。新增的 JSON v1 描述符使用准确 ID；归属接口定义字段及可执行的验证。新增隔离处理器的输入端口是可选声明；归属方直接调用不会创建图边或路由。下列场景描述符没有处理器或端口。

| 归属方 | 既有注册 ID | 新增 ID |
| --- | --- | --- |
| EI | `resource_bundle@v0.2`、`observation_set@v0.2`、`context_snapshot@v0.2`、`selected_mission@v0.2`、`primitive_proposal@v0.2`、`health_update@v0.2` | `language_instruction@v0.2`、`vla_input@v0.2`、`grounded_goal@v0.2`、`motion_proposal@v0.2`、`prediction@v0.2`、`memory_reference@v0.2` |
| EOS | `image_frame@v0.2`、`joint_state@v0.2` 以及下文既有的硬件和机械臂类型 | `sample_request@v0.2`、`movement_binding_request@v0.2`、`binding_result@v0.2`、`physical_event@v0.2`、`device_status@v0.2`、`stop_request@v0.2`、`stop_latch@v0.2`、`evidence_record@v0.2`、`episode_reference@v0.2`、`gap_record@v0.2`、`manifest_request@v0.2`、`dataset_manifest_reference@v0.2` |
| 仅 EOS 场景数据 | — | `depth_frame_ref@v0.2`、`segmentation_mask_ref@v0.2`、`point_cloud_ref@v0.2`、`physical_detection@v0.2`、`imu_sample@v0.2`、`force_torque_sample@v0.2`、`tactile_sample@v0.2`、`contact_event@v0.2`、`audio_chunk_ref@v0.2` |

这些名称表示不同含义，不是别名。EI 拥有认知和提案语义；EI 运动提案绝不是 EOS 命令。EOS 拥有测量、物理状态、证据来源和 Dispatcher 准入。直接处理器的验证、不可用状态和 Stop 行为见 [EI 隔离库](../../ei/interfaces_cn.md#cognition_isolated_library_cn)及 [EOS 隔离库](../../eos/interfaces_cn.md#isolated_library_cn)。仅描述符类型及其限制见 [EOS 场景数据契约](../../eos/interfaces_cn.md#scene_data_cn)。

起始图将 `button_activation@v0.2` 和 `arm_joint_delta@v0.2` 注册为大小上限 1024 字节的 JSON v1 负载。激活事件包含 `event_id` 和 `kind: "activate"`，没有运动目标。EOS 增量包含恰好六个以弧度为单位的有限 `joint_deltas_rad` 值。IsOS 经 EI 传递前一种类型；只有 EOS 创建后一种类型。界限与准入见 [EOS 机械臂控制](../../eos/interfaces_cn.md#arm_control_cn)。

按钮通过 `activation` 输出发出如下完整激活值。不同点击使用新的事件 ID；该值本身不包含目标或运动参数。

```json
{
  "event_id": "click-1",
  "kind": "activate"
}
```

通用 EOS 硬件声明新增 `image_frame@v0.2`、`base_state@v0.2`、`joint_state@v0.2`、`base_twist_command@v0.2` 和 `joint_position_command@v0.2`。它们使用 JSON v1 描述符和准确的边类型身份。其语义及命令准入边界见 [EOS 通用硬件契约](../../eos/interfaces_cn.md#general_hardware_cn)。

[English](../../../docs/node/interfaces/type.md#data_types)

<a id="selectors"></a>
<a id="selectors--设置与选择器"></a>
## 设置与选择器

本页供类型发布者和 UI 作者使用。IsOS 当前依据注册类型验证实例设置，并从声明的来源投影选择器选项。选择器是设置声明，不是软件包专属表单或请求时的驱动参数。[资源](../../isos/interfaces/package/README_cn.md#resources)解释经过验证的软件包选项。

| 字段 | 类型／单位 | 含义与验证 | 缺失／默认值 |
| --- | --- | --- | --- |
| `name` | 非空字符串／设置 ID | 在一个注册类型内唯一。 | 必填。 |
| `data_type` | 枚举／JSON 值类别 | `string`、`number`、`integer`、`boolean`、`array`、`object`；当前别名还包括 `string_array`、`string_table`、`positive_integer`、`nonnegative_integer`。 | 必填。 |
| `required` | 布尔值／存在规则 | 必填实例值缺失时，需要类型声明的默认值。 | 必填。 |
| `description` | 非空字符串／展示文字 | 告诉作者选择什么及其单位。 | 必填。 |
| `unit` | 非空字符串／物理或展示单位 | 说明数值或文本如何度量。 | 可选。 |
| `min`、`max` | 有限数字／设置单位 | 限定数值实例；`min` 不得大于 `max`。 | 可选；无对应边界。 |
| `default` | 与 `data_type` 匹配的值 | 填补省略的实例值。 | 可选；无默认值。 |
| `choices` | 非空匹配值数组 | 限定合法选项。 | 可选；无有限列表。 |
| `selector.provider` | 非空字符串／选项提供者 | 指定 `package`、`graph`、`owner` 或本地 `ui` 选项来源。`owner` 从选中的 EOS 或 EI 设置服务取得上下文选项；`ui` 在浏览器本地提供选项。 | 存在 `selector` 时必填。 |
| `selector.resource` | 非空字符串／资源类别 | 指定请求的选项类型。 | 存在 `selector` 时必填。 |
| `selector.package_from` | 非空字符串／设置引用 | 选择提供选项的精确软件包锁定。 | 可选。 |
| `selector.filter` | 对象／提供者过滤条件 | 不解释归属方负载而限制选项。 | 可选；空过滤条件。 |

IsosUI 根据声明渲染提供者选项。图、软件包和归属方选项只是预览；Apply 会再次验证，避免过期选择获准。归属方未返回选项时，选择器被阻止，不允许输入未经验证的文本。按键或组合键说明固定节点设置：它属于实例配置，而发出的信号属于输出端口单独声明的负载契约。本次更改不启用浏览器按键选择器或键盘控制路由。

[English](../../../docs/node/interfaces/type.md#selectors)
