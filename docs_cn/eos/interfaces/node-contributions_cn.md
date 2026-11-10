# EOS 软件包节点贡献

> 历史契约：当前按钮到模拟机械臂图未实现此接口。当前数据及命令路径参见[机械臂控制](arm-control_cn.md)；设置、Run、Status 和 Stop 参见[所有者生命周期](owner-health_cn.md)。


[English](../../../docs/eos/interfaces/node-contributions.md) · [统一硬件 profile](hardware_cn.md) · [原语绑定](primitive-bindings_cn.md)

IsOS 安装或链接 Aether 软件包并管理已应用的图。软件包发布版本可以贡献已注册的节点类型和节点集；IsOS 验证发布版本，为 EOS 写出派生的只读节点类型侧文件。EOS 读取图与侧文件，不修改它们。本 JSON 接口是 EOS 专用的节点 IDL，描述身份、位置、设置和类型化端口；硬件采样与命令在 EOS 内仍使用 Rust 类型。下方示例使用当前已注册节点身份和类型化端口。

## 已验证的节点类型侧文件

IsOS 通过可重复的 `--node-types <absolute-path>` 参数把已验证的侧文件传给 `aether-profile`、`aether-runtime` 和 `aether-config-server`。每个文件最多 1 MiB。EOS 先载入安全声明，再载入对应类型；重复或格式错误的定义会被拒绝，并使用内置类型与贡献类型的合并注册表校验图。枚举值使用 `snake_case`，未知字段解析失败。

```json
{
  "package": {
    "id": "island/mujoco-sim",
    "version": "0.1.1"
  },
  "safety_adapters": [],
  "data_types": [],
  "adapter_families": [
    {
      "id": "mujoco-v2",
      "device_kinds": [
        "camera",
        "base",
        "manipulator"
      ],
      "transport": "unix_socket",
      "description": "EOS-owned MuJoCo simulator socket adapter for camera, base, and manipulator devices",
      "supported_primitives": [
        "base.velocity@1.0.0",
        "arm.movej@1.0.0"
      ]
    }
  ],
  "node_types": [
    {
      "id": "isos.eos_process.mujoco@v0.2",
      "core": "isos",
      "layer": "eos_process",
      "group": "mujoco",
      "kind": "software",
      "lifecycle": "process",
      "capabilities": [
        "process"
      ],
      "settings": [
        {
          "name": "scene",
          "data_type": "string",
          "required": true,
          "description": "Stable verified scene ID",
          "selector": {
            "provider": "package",
            "resource": "scene",
            "package_from": "node.package"
          }
        },
        {
          "name": "socket",
          "data_type": "string",
          "required": true,
          "description": "Owner-only simulator Unix socket path"
        },
        {
          "name": "env",
          "data_type": "string_table",
          "required": false,
          "description": "Simulator child environment references"
        },
        {
          "name": "start_timeout_ms",
          "data_type": "positive_integer",
          "required": false,
          "description": "Simulator readiness deadline in milliseconds"
        }
      ],
      "inputs": [],
      "outputs": [
        {
          "name": "status",
          "data_type": "status",
          "required": false,
          "description": "Simulator process lifecycle",
          "transport": "in_process",
          "data_class": "state"
        },
        {
          "name": "health",
          "data_type": "health",
          "required": false,
          "description": "Simulator process health",
          "transport": "in_process",
          "data_class": "state"
        }
      ],
      "safety_adapter_type": null,
      "description": "Starts one verified MuJoCo scene as an EOS-owned simulator child and supplies only the graph-declared simulated devices."
    }
  ]
}
```

`package` 是精确的 `{id, version}` 引用。ID 由 `/` 分隔的两个有效地址段组成；版本包含三个非空数字部分。贡献类型的实例必须将相同值写入 `node.package`；内置节点省略该字段。注册检查声明的停止能力，但 EOS 只有在类型和软件包对应的适配器及停止实现已编译进运行时时才准入物理执行器。

`node_types` 是[硬件契约](hardware_cn.md)中的 `NodeType` 定义数组；`safety_adapters` 是 `SafetyAdapterType` 数组。Rust 与 JSON 字段相同。`core`、`layer` 和 `group` 是必填的位置段；图不能从 group 推断硬件种类。`kind` 值为 `camera`、`base`、`manipulator` 和 `software`；生命周期值为 `internal`、`process` 和 `adapter`。IsOS 软件包节点集使用这些类型 ID 和符号图引用；已应用图只包含具体路径。

可选的 `data_types` 是 `DataTypeDescriptor` 数组。每项包含 `id`（字符串，唯一且带版本，例如 `vendor.camera.depth@v0.2`）、`version`（字符串，`0.2`，与 ID 后缀一致）、`purpose`（非空用途说明字符串）、`media_type`（非空 MIME 字符串）、`codec`（非空运行时编解码器 ID）、`schema`（描述逻辑字段的 JSON 对象，使用 `description`、`x-unit` 和 JSON Schema 校验关键字）、`max_payload_bytes`（正整数，字节）。内置 ID 继续使用 snake case，例如 `camera_frame` 和 `vision_frame`。贡献端口引用的数据类型必须拥有已编译进 EOS 的编解码器。内置编解码器 ID 为 `json_v1` 和 `vision_frame_jpeg_v1`。`aether-profile catalog` 和配置服务器的 `GET /api/eos/v2/data-types` 暴露合并目录。

连线的来源与输入必须使用完全相同的已注册 `data_type` ID。schema 的 `x-unit` 注释描述该类型中载荷字段的单位，用于显示和类型内部的校验；不同类型 ID 即使单位相同也不能互换。图连线不做隐式单位转换。贡献描述符不能以不同单位重定义已注册 ID；重复 ID 在注册时拒绝。IsOS 拥有指向具体连线的 Apply 诊断；EOS 拥有启动后的实时健康。[硬件连线契约](hardware_cn.md)规定来源 QoS、路由分类、传输和安全限制。软件包贡献端口不能仅通过设置 `data_class: "command"` 或匹配内置运动类型取得命令路径：硬件命令输入只接受已注册的 Dispatcher 原语绑定来源。

可选的 `adapter_families` 数组声明适配器族。每项必填 `id`（有效族名段）、`device_kinds`（非空的 `camera`、`base` 或 `manipulator` 列表）、`transport`（`in_process`、`unix_socket`、`websocket` 或 `ros2`）和 `description`（非空说明）。可选 `settings_schema` 为 JSON 对象；可选 `supported_primitives` 是已注册 `<名称>@<版本>` 字符串数组。侧文件外层的 `package` 提供精确来源。软件包作者不能设置 `compiled`；EOS 根据实际已编译的工厂计算该字段，并在目录中暴露 `compiled` 与 `reason`。通用硬件实例把软件包引用写在 `node.adapter.package`，与内置节点类型独立。

每个输入或输出端口必填 `name`（图内名称字符串）、`data_type`（已注册 ID 字符串）和 `required`（布尔值）。可选 `description` 说明用途；`expected_rate_hz` 是正且有限的赫兹数；`transport` 可为 `in_process`、`unix_socket`、`websocket` 或 `ros2`；`data_class` 可为 `image`、`observation`、`command`、`event`、`state` 或 `opaque`；`cardinality` 可为 `one` 或 `many`（默认为 `one`）。连线校验使用基数、传输和数据类别；`expected_rate_hz` 仍是显示提示，不是连线限速。每个 `@v0.2` 类型的端口都声明传输与数据类别。运行时数据身份由精确的已注册 `data_type` ID 决定。JPEG 编解码和只读传输见 [Vision 接口](vision_cn.md)。

## MuJoCo 贡献

MuJoCo 软件包目前在 `isos/eos_process/mujoco` 贡献 `isos.eos_process.mujoco@v0.2` 和 `mujoco-v2` 适配器族。新图的 sidecar 贡献带有非空类型描述及完整设置和端口元数据的 `isos.eos_process.mujoco@v0.2`，不贡献新的 MuJoCo 专用硬件类型。新节点集在 `isos/eos_hardware/<种类>/<id>` 使用内置通用相机、底盘和机械臂 `@v0.2` 类型，设备的 `node.adapter.package = {id = "island/mujoco-sim", version = "0.1.1"}`。进程类型来自软件包，所以进程节点携带 `node.package`。其设置包括 `scene`、`socket`，可选 `env` 和 `start_timeout_ms`；不在设置中重复软件包身份。每个设备适配器引用该进程节点。底盘速度与机械臂关节位置绑定连接到设备命令输入；所有执行器命令仍由共享 EOS Dispatcher 准入。


## 类型化数据身份

EOS 可用 `NodeDataEnvelope<T>` 包装进程内硬件值。信封包含 `source_node`（规范图路径）、`port`（已注册输出名）、`data_type`（`PortDataType`）、`contract_version`（`2.0.0`）和类型化的 `payload`。当输出端口声明的类型与 `T::DATA_TYPE` 不一致时，`NodeDataEnvelope::output` 拒绝该值。信封用于按 IsOS 图识别数据，不授予命令准入，也不建立第二条执行器路径。

`TypedDataHub` 将这类信封发布到已注册输出键。`subscribe<T>` 校验节点输出端口并返回类型化游标；`poll_latest` 只返回更新的值。每个输出仅保留一个值，慢速订阅者跳过中间值，不会累积队列。丢弃游标即断开该读取者。已编译的 `DataTransformer<Input, Output>` 声明编解码器 ID 并接收类型化选项；`CameraJpegTransformer` 使用 `vision_frame_jpeg_v1` 将 `camera_frame` 转成 `vision_frame`。

无效侧文件 JSON、重复类型 ID、缺少安全适配器、软件包不匹配、未知类型、位置不匹配和不兼容的端口边都在运行时构造前校验失败。缺少已编译的执行器实现在 EOS socket 绑定前导致适配器构造失败。

## 0.x 类型描述——已实现的一部分

新贡献的 `@v0.2` 节点类型必须包含非空字符串 `description`，解释节点功能、输入、输出及操作员可见效果；EOS 目录现在显示它，计划中的 IsosUI 添加节点选择器将显示它。内置类型同样拥有该字段。新类型描述为空、类型 ID 重复、软件包固定版本不匹配或编解码器不支持时，在 Apply 前拒绝注册。外层 `package` 和适配器族 `description` 保持既有含义。

## 目标设置选择器

`Setting.selector` 是新类型定义中的可选 JSON 对象，说明 IsOS 从何处取得选项，不包含可执行代码或任意文件路径。现有 `name`、`data_type`、`required`、`description`、`unit`、边界和静态选项规则见[硬件契约](hardware_cn.md)。

| 字段 | JSON 类型与单位 | 用途、默认值、校验与兼容性 |
| --- | --- | --- |
| `provider` | 字符串枚举，— | 有 `selector` 时必填：`package`、`registry` 或 `graph`；没有选择器时使用普通编辑器。 |
| `resource` | 非空字符串 ID，— | 必填选项族，例如 `scene`、`device`、`adapter_family` 或 `node`；只能由选定 provider 解析。未知族拒绝注册。 |
| `package_from` | 可选字符串枚举，— | `provider=package` 时必填：`node.package` 或 `node.adapter.package`；其他 provider 禁用。它选择图草稿中的精确已验证固定版本。 |

```json
{
  "name": "scene",
  "data_type": "string",
  "required": true,
  "description": "Verified MuJoCo scene ID",
  "selector": {
    "provider": "package",
    "resource": "scene",
    "package_from": "node.package"
  }
}
```

## 结构化设置元数据

已注册的 `object_array` 设置声明 `items_schema` 和条目数量边界。注册表校验结构与大小；IsOS 校验软件包固定版本和可执行文件路径。EOS 进程节点的 `settings.skill_executables` 选择经验证的 Rust `.skill` 文件。每项指定精确软件包和相对可执行文件。[Rust 技能契约](skills/rust_cn.md)规定选择交接与工作进程边界。
