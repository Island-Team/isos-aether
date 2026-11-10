# EOS v2 原语绑定与类型化命令信封

> 历史契约：当前按钮到模拟机械臂图未实现此接口。当前数据及命令路径参见[机械臂控制](arm-control_cn.md)；设置、Run、Status 和 Stop 参见[所有者生命周期](owner-health_cn.md)。


[English](../../../docs/eos/interfaces/primitive-bindings.md) · [硬件 profile](hardware_cn.md) · [图控制](control_cn.md)

EOS 原语绑定命名一个带版本的运动操作，并把它连接到恰好一个硬件设备。IsOS 图拥有绑定边；EOS 读取该边，校验类型化命令头，并通过共享 Dispatcher 传递解码后的 Rust 运动值。绑定本身不拥有驱动，也不授权直接释放执行器命令。

## 图契约

内置类型 `isos.eos_dispatcher.primitive_base@v0.2` 和 `isos.eos_dispatcher.primitive_manipulator@v0.2` 分别位于 `isos/eos_dispatcher/base/<id>` 与 `isos/eos_dispatcher/manipulator/<id>`。其 `command` 输出类型分别为 `base_movement` 与 `manipulator_movement`。通用硬件类型 `isos.eos_hardware.base@v0.2` 和 `isos.eos_hardware.manipulator@v0.2` 的 `inputs.command` 基数为 `many`，即一个或多个 `{node,output}` 的显式数组。每个绑定必须恰好连接到一个兼容设备。绑定没有目标设置；图边就是目标。此路径隐式使用共享 Dispatcher 服务，因此图不必包含 `isos.eos_dispatcher.control@v0.2`。

```json
{
  "id": "velocity",
  "node_type": "isos.eos_dispatcher.primitive_base@v0.2",
  "label": "Base velocity",
  "description": "Bounded velocity binding",
  "settings": {
    "primitive_name": "base.velocity",
    "primitive_version": "1.0.0",
    "movement_type": "body_velocity"
  }
}
```

```json
{
  "command": [
    {
      "node": "isos/eos_dispatcher/base/velocity",
      "output": "command"
    }
  ]
}
```

| 字段 | 类型 | 单位 | 用途与校验 |
| --- | --- | --- | --- |
| `settings.primitive_name` | 字符串 | 原语 ID | 必填；当前底盘为 `base.velocity`，机械臂为 `arm.movej`。 |
| `settings.primitive_version` | 字符串 | 语义版本 | 必填；当前两个已准入原语均为 `1.0.0`。 |
| `settings.movement_type` | 字符串 | 模式 ID | 必填；`base.velocity` 对应 `body_velocity`，`arm.movej` 对应 `joint_positions`；设备必须声明此模式。 |
| `inputs.command` | `PortSource` 数组 | 类型化图边 | 执行器必填；来源各不相同、类型兼容，且为绑定输出。一个绑定恰好对应一个设备。 |
| `PortSource.node` | 字符串 | 规范路径 | 来源实例 `<core>/<layer>/<group>/<node>`；必须存在。 |
| `PortSource.output` | 字符串 | 输出端口 | 必须是来源已注册的 `command` 输出。 |
| `node.adapter.type` | 字符串 | 适配器族 ID | 选择一个设备所有的已编译工厂，如 `mujoco-v2` 或 `mock-cxx`。 |
| `node.adapter.package` | `{id:string,version:string}` 或缺省 | 软件包引用 | 软件包适配器族必填；精确匹配已验证软件包，与内置节点类型的 `node.package` 独立。 |
| `node.adapter.settings` | 对象 | 适配器族专用 | 经校验的设备端点设置；MuJoCo 使用 `backend`、`socket_path`、`simulator_node`、`device_id`。 |

注册目录的 `primitive_specs` 包含 `name`、`version`、`device_class`、`movement_type`、`description`。`adapter_families` 包含 `id`、`device_kinds`、`transport`、`description`、`supported_primitives`、可选 `settings_schema`、可选精确 `package`，以及由 EOS 计算的 `compiled` 和 `reason`。软件包侧文件可以声明适配器族字段，但不能声明代码已编译。`GET /api/eos/v2/primitive-specs`、`GET /api/eos/v2/adapter-families` 与 `aether-profile catalog --node-types <已验证侧文件>` 提供这些目录。已编译的 `mujoco-v2` 适配器族支持 `island/mujoco-sim@0.1.1` 的相机、底盘及机械臂设备。一个适配器拥有一个物理设备及其类型化方法和停止方法；原语不创建第二个适配器。

## 有界运行时命令

已合并的 EOS R03 实现中仅所有者可访问的 Unix 控制 socket 接收最多 64 KiB 的单行 JSON。新绑定使用 `primitive_typed` 操作；`payload` 是[硬件 v2](hardware_cn.md)定义的类型化运动对象。**私有** `payload_length_bytes` 统计此对象经解析后通过 `serde_json` 紧凑 UTF-8 序列化的字节数。回环 HTTP bridge 接收有界的公开紧凑 UTF-8 长度，在解析后规范化私有请求头，因为 JavaScript 和 Rust 对相同有限数字的序列化字节数可能不同；见[bridge 契约](control_cn.md#回环-api)。解码器在请求 Dispatcher 准入前拒绝未知字段、超过 16 KiB 的载荷、不支持的编解码器或版本、伪造的图目标，以及原语与载荷类型不匹配。Dispatcher 继续校验健康、截止时间、限值、日志和停止。

已合并的 EOS R03 实现中，浏览器调用经现有回环 `/api/eos/v2/control` bridge：它允许 `primitive_typed`，检查 Origin 和大小后转发到此私有 runtime 操作。调用方可以发送 `deadline_after_ms`，由 EOS 根据自身单调时钟换算。继续使用绝对截止时间的调用方通过 bridge 获取 EOS 单调时钟时间。EOS 再次验证类型化头、精确活动绑定及目标。响应中的 `command_id`、精确的 `Completed`/`Cancelled`/`Rejected`/`InProgress`/`OutcomeUnknown` 状态和原因驱动面板结果显示；浏览器本地的发送状态仅表示等待。浏览器 bridge 将有界公开 `result` 改写为受保护的私有 `primitive_result`；[R07 授权结果契约](control_cn.md#r07-浏览器授权结果后续查询)定义 UI 后续查询 `InProgress` 所需的有界且按权限校验的读取。不确定结果绝不显示为已完成。这是现有契约的 UI 入口，不是新图连线或 gRPC 服务名。

```json
{
  "op": "primitive_typed",
  "header": {
    "data_type_id": "base_movement",
    "data_type_version": "1.0",
    "primitive_name": "base.velocity",
    "primitive_version": "1.0.0",
    "source_path": "isos/eos_dispatcher/base/velocity",
    "target_path": "isos/eos_hardware/base/drive",
    "command_id": "move-1",
    "sequence": 1,
    "deadline_monotonic_ns": 5000000000,
    "payload_length_bytes": 83,
    "codec": "json_v1"
  },
  "payload": {
    "type": "body_velocity",
    "x_m_s": 0.1,
    "y_m_s": 0.0,
    "yaw_rad_s": 0.0,
    "valid_for_ms": 100
  }
}
```


| 命令头字段 | 类型 | 单位 | 用途与校验 |
| --- | --- | --- | --- |
| `data_type_id` | 字符串 | 注册 ID | `base_movement` 或 `manipulator_movement`，须与解码后的载荷及绑定端口匹配。 |
| `data_type_version` | 字符串 | IDL 版本 | 必须为 `1.0`。 |
| `primitive_name` | 字符串 | 原语 ID | 须与绑定的 `primitive_name` 相同。 |
| `primitive_version` | 字符串 | 语义版本 | 须与绑定的 `primitive_version` 相同。 |
| `source_path` | 字符串 | 规范图路径 | 须以短路径精确指向绑定节点。 |
| `target_path` | 字符串 | 规范图路径 | 须与图边选定的唯一硬件设备相同。 |
| `command_id` | 字符串 | 命令身份 | 非空、至多 128 个 UTF-8 字节；Dispatcher 还校验 ID 格式及重复策略。 |
| `sequence` | 无符号整数 | 命令序号 | 必须为正数；调用方提供，用于排序及追踪。 |
| `deadline_monotonic_ns` | 无符号整数 | ns | EOS 单调时钟截止时间；Dispatcher 拒绝过期命令。 |
| `deadline_after_ms` | 无符号整数 | ms | HTTP bridge 可使用的相对截止时间；EOS 根据自身单调时钟换算，范围为 1–5,000 ms。不能与绝对截止时间同时填写。 |
| `delivery_mode` | 字符串 | — | 默认 `one_shot`；`stream` 通过已声明的 gateway 连线发布有界运动租约。 |
| `run_id` | 字符串 | — | stream 必填；必须等于当前受管理运行的 ID，即使重启后的修订不变。 |
| `gateway_path` | 字符串 | 图路径 | stream 必填；指向连接到 primitive binding 输入的 EOS HTTP gateway 节点。 |
| `payload_length_bytes` | 无符号整数 | 字节 | 私有 IPC：精确的紧凑 `serde_json` 载荷长度，范围 1 至 16,384；公开 HTTP：有界提示，由 bridge 在私有 IPC 前规范化。 |
| `codec` | 字符串 | 编解码器 ID | 必须为 `json_v1`；EOS 在 CXX 方法之前解码成 `Movement`。 |

### 流式运动

IsOS HTTP bridge 为每条 `base_movement` 命令发送 `delivery_mode:"stream"`、当前 `run_id`、已声明的 `gateway_path` 和 `deadline_after_ms`。EOS 仅允许 `body_velocity` 使用 stream，并在发布前检查活动修订、运行 ID 和图连线。后续 `primitive_result` 可能返回 `Completed`，原因是 `stream lease published`：这表示命令已交给硬件适配器，不表示实体已经静止。每条命令替换前一条有界租约；按键释放时，零速度命令会及时替换持续运动。该路径的 gateway 和 primitive binding 均声明 `max_rate_hz:20`。
