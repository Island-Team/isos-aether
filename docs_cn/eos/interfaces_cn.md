# EOS interfaces

<!-- doc-navigation:start -->
<details>
<summary>目录：eos</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EOS 文档](README_cn.md)
  - [EOS interfaces](models_cn.md)
- 下级目录：
  - [core/](core/README_cn.md)
  - [interfaces/](interfaces/)

</details>
<!-- doc-navigation:end -->

## 目录

- [EOS 接口](#readme_cn--eos-接口)
- [EOS 演示机械臂激活](#arm_control_cn--eos-演示机械臂激活)
- [EOS 模拟机械臂观测](#arm_monitor_cn--eos-模拟机械臂观测)
- [EOS 契约源码](#contracts_cn--eos-契约源码)
- [EOS 通用硬件声明](#general_hardware_cn--eos-通用硬件声明)
- [EOS 隔离节点库](#isolated_library_cn--eos-隔离节点库)
- [EOS v2 原语绑定与类型化命令信封](#primitive_bindings_cn--eos-v2-原语绑定与类型化命令信封)
- [EOS 物理场景数据](#scene_data_cn--eos-物理场景数据)
- [Rust 技能可执行程序契约](#skills_rust_cn--rust-技能可执行程序契约)


<a id="readme_cn"></a>
<a id="readme_cn--eos-接口"></a>
## EOS 接口

[English](../../docs/eos/interfaces.md#readme) · [EOS](README_cn.md) · [架构](core/architecture_cn.md)

EOS 节点实例可以省略 `description` 或将其设为空字符串；标签仍为必填项。

EOS 在图格式 `0.2.1` 中负责到模拟机械臂的控制路径及其观测返回路径。[机械臂控制契约](interfaces_cn.md#arm_control_cn)定义激活、增量和限值。[机械臂监控契约](interfaces_cn.md#arm_monitor_cn)定义类型化返回路径和私有读取路由。IsOS 负责保存图、软件包检查、Apply 和部署状态。[所有者生命周期](../isos/interfaces/server/eos_cn.md#owner_health_cn)定义 Run、Status、Stop 和工作进程故障。[设置交换](../isos/interfaces/server/eos_cn.md#setup_v1_cn)定义身份、目录、验证和渲染。[快速入门](../isos/start/quick-start_cn.md)按操作顺序说明。

[通用硬件声明](interfaces_cn.md#general_hardware_cn)定义摄像头、底盘和机械臂身份及通用负载含义。注册和可执行适配器支持分别检查。

[软件包观测运行时](core/hardware/hardware_cn.md)定义当前可执行的外部相机适配器、私有观测路由、ABI 探测和 Stop 边界。

[EOS 隔离节点库](interfaces_cn.md#isolated_library_cn)定义七个可直接执行的内建类型、类型化负载、验证和停止行为。这些节点没有图路由。

[物理场景数据契约](interfaces_cn.md#scene_data_cn)定义深度图、掩码、点云、物理检测、IMU、力、触觉、接触和音频来源的受限元数据与引用。这些仅是数据类型；注册不会提供解码器或图路由。

EOS 通过 [HTTP/SSE 适配器](interfaces/server_cn.md#external_http_cn)与外部进程通信，该适配器为 WebUI 请求调用 EOS gRPC 服务；EOS 还通过 HTTP、WebSocket 或配置的 gRPC 调用[模型提供方](models_cn.md#external_models_cn)。私有模型 schema 位于 `contract/schemas/models/`。这些传输不构成空间命名体系。技能 mailbox 属于原生技能编写和技能运行时。

<a id="readme_cn--节点库阅读路径"></a>
### 节点库阅读路径

当前 EOS 节点库遵循注册的 `<layer>/<group>` 路径。参见[接口层](interfaces_cn.md#readme_cn--eos-interface-contracts)、[技能层](interfaces_cn.md#readme_cn--eos-skill-contracts)、[调度层](interfaces_cn.md#readme_cn--eos-dispatcher-contracts)和[硬件层](interfaces_cn.md#readme_cn--eos-hardware-contracts)。这些索引链接至上面的归属契约。

<a id="readme_cn--契约源码目录"></a>
### 契约源码目录

`contract/proto/` 中的模式源码通过 `gateway.proto`、`skills.proto` 和 `workflow.proto` 定义 `island.eos.v1` gRPC 服务，通过 `capabilities.proto` 定义 `island.eos.v2` 面向智能体的能力及操作服务。`actions/`、`body/`、`world/`、`system/`、`robot/`、`perception/`、`workflow/` 和 `skills/` 文件夹保存领域消息；`options/` 保存共享标注，`external/` 保存 HTTP 路由标注和私有模型契约。项目导入路径相对于 `proto/`；各软件包声明自己的线协议版本。生成的公共 JSON Schema 位于 `contract/schemas/`，HTTP OpenAPI 位于 `contract/schemas/north-http.openapi.json`。[南向硬件边界](interfaces_cn.md#readme_cn--eos-hardware-contracts)由 Rust 运行时负责，没有 protobuf 或 JSON Schema 源码：EOS 先准入所选适配器，Dispatcher 随后才释放已验证的命令。

<a id="readme_cn--eos-hardware-contracts"></a>
<a id="eos-hardware-contracts"></a>

<a id="readme_cn--eos-hardware-层"></a>
### EOS hardware 层

硬件节点契约描述面向设备的类型化图端口和设置。内置 mock 分组用于当前演示；其他分组代表软件包贡献的候选节点，仅在精确软件包通过验证后可用。类型化硬件载荷与后端软件包元数据、节点设置保持分离。

通过 [mock](interfaces/node/hardware/mock_cn.md#hardware_mock_cn)、[camera](interfaces/node/hardware/camera_cn.md#hardware_camera_cn)、[base](interfaces/node/hardware/base_cn.md#hardware_base_cn)、[manipulator](interfaces/node/hardware/manipulator_cn.md#hardware_manipulator_cn)、[gripper](interfaces/node/hardware/gripper_cn.md#hardware_gripper_cn) 和 [torso](interfaces/node/hardware/torso_cn.md#hardware_torso_cn) 分组介绍查找节点契约。共用适配器义务见[通用硬件契约](interfaces_cn.md#general_hardware_cn)。

<a id="readme_cn--eos-dispatcher-contracts"></a>
<a id="eos-dispatcher-contracts"></a>

<a id="readme_cn--eos-dispatcher-层"></a>
### EOS dispatcher 层

Dispatcher 层公开类型化机械臂命令与监视节点。命令仅在属于当前运行且路线已声明时才被接受；拒绝不会改变执行器。机械臂控制契约定义载荷与准入规则，节点页面标识面向图的端口和设置。

[arm 分组](interfaces/node/dispatcher/arm_cn.md#dispatcher_arm_cn)列出两个节点契约。相关契约及运行时背景分别见[机械臂控制](interfaces_cn.md#arm_control_cn)、[机械臂监视](interfaces_cn.md#arm_monitor_cn)和 [Dispatcher 设计](core/README_cn.md#eos-dispatcher-layer)。

<a id="readme_cn--eos-interface-contracts"></a>
<a id="eos-interface-contracts"></a>

<a id="readme_cn--eos-interface-层"></a>
### EOS interface 层

interface 节点定义 EOS 边界上的图输入与输出。ingress trigger 进入已声明的图路线；机械臂状态和视觉流通过各自声明的 egress 路线输出。视觉传输绑定到实时摄像头来源及运行身份。这些页面标识节点端口与边界义务，不重定义所有者级传输契约。

请阅读 [ingress 分组](interfaces/node/interface/ingress_cn.md#interface_ingress_cn)和 [egress 分组](interfaces/node/interface/egress_cn.md#interface_egress_cn)，再参阅[interface 设计](core/README_cn.md#eos-interface-layer)了解执行路径。

<a id="readme_cn--eos-skill-contracts"></a>
<a id="eos-skill-contracts"></a>

<a id="readme_cn--eos-skill-层"></a>
### EOS skill 层

skill 节点在 EOS 图中公开类型化软件行为。demo 分组的 joint-delta 与 arm-status 节点连接到声明的控制及观测链。节点契约规定图身份、设置和端口；Rust 技能宿主契约规定选定实现的运行方式。

[demo 分组](interfaces/node/skill/demo_cn.md#skill_demo_cn)链接其节点契约。宿主行为见 [Rust 技能契约](interfaces_cn.md#skills_rust_cn)和[技能设计](core/README_cn.md#eos-skill-layer)。

<a id="arm_control_cn"></a>
<a id="arm_control_cn--eos-演示机械臂激活"></a>
## EOS 演示机械臂激活

[English](../../docs/eos/interfaces.md#arm_control) · [EOS 接口](interfaces_cn.md#readme_cn) · [硬件设计](core/hardware/hardware_cn.md)

<a id="arm_control_cn--概览"></a>
### 概览

EI 通过私有 Unix socket 调用 EOS 所有者的 `POST /v1/activate`。所有者检查当前运行，并把事件转发给其直接子进程。EOS 接口节点把激活传给内置演示技能；技能产生六关节增量，Dispatcher 准入后交给模拟机械臂适配器。浏览器和后端包不能直接调用适配器。

<a id="arm_control_cn--认证"></a>
### 认证

所有者 socket 仅本地可用，权限为 `0600`。IsOS 控制启动、运行 ID、修订版本及 EI 到 EOS 的连接。EOS 所有者要求与当前运行完全一致的运行 ID 和修订版本。工作进程 socket 同样只在本地使用，权限为 `0600`。Stop 在子进程退出之前关闭准入。

<a id="arm_control_cn--接口"></a>
### 接口

`button_activation@v0.2` 为 JSON 对象，必填 `event_id`（1–64 个 ASCII 字母、数字、`_`、`-` 或 `.`）及值为 `activate` 的 `kind`。`arm_joint_delta@v0.2` 为包含 `joint_deltas_rad` 的 JSON 对象，恰好六个有限数值，单位为弧度。内置技能产生 `[0.05, -0.04, 0.03, -0.02, 0.01, -0.01]`。每次增量的绝对值不超过 0.25 rad。EOS 在启动前拒绝图中大于 3.14 rad 的 `max_joint_position_rad`；Dispatcher 拒绝让模拟关节超出已准入图限值的结果。模拟机械臂从六个零值开始，在同一次工作进程运行期间保留位置。其[观测返回路径](interfaces_cn.md#arm_monitor_cn)持续采样，遇到过期或无效样本时关闭运动准入。

四个控制图节点为 `eos.interface.ingress.trigger@v0.2`、`eos.skill.demo.joint_delta@v0.2`、`eos.dispatcher.arm.delta@v0.2` 和 `eos.hardware.mock.arm@v0.2`。入口输入使用 `ei/activation-to-eos/1` Unix socket 桥接；入口输出与技能输入使用进程内的 `eos/activation-local/1`；其余边使用进程内的 `eos/arm-delta/1`。所有端口使用事件传递；激活归类为事件数据，增量归类为命令数据。模拟节点是机械臂适配器，具备已注册的 `mock_arm` 安全适配器和 `joint_positions` 能力。设置目录公开已编译的 `mock-arm` 适配器族和 `arm.delta@1.0.0` 原语。

<a id="arm_control_cn--路由"></a>
### 路由

<a id="arm_control_cn--post-v1activate"></a>
### POST /v1/activate

EI 发送 JSON 正文，字段必须恰好为 `schema_version`（`1`）、`core`（`eos`）、`op`（`activate`）、`run_id`、`revision`（64 个小写十六进制字符）、`source_node`（`isos/ui/button/<id>`，ID 为 1–48 个 ASCII 字符）、`event_id` 和 `kind`（`activate`）。EOS 信任 EI 已根据完整图准入该来源，并检查其仍为规范的 IsOS 按钮路径。未知或无效字段会被拒绝。每个事件 ID 在运行期日志中同步保留后才返回 `accepted`；单执行器工作线程随后处理运动。同一次运行中重复的 ID 返回同一接收回执，不再次移动。成功返回 HTTP 200，拒绝返回 422。传输结果不确定时报告 `outcome_unknown`，不得使用新的事件 ID 自动重试。

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "activate",
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "source_node": "isos/ui/button/trigger",
  "event_id": "click-1",
  "kind": "activate"
}
```

```json
{
  "schema_version": 1,
  "core": "eos",
  "ok": true,
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1",
  "command_id": "arm-click-1",
  "status": "accepted"
}
```

例如，旧运行的请求会被拒绝，且不产生命令：

```json
{
  "schema_version": 1,
  "core": "eos",
  "ok": false,
  "error": "stale_or_missing_run"
}
```

拒绝的请求包含 `ok:false` 和 `error` 代码：`invalid_run_identity`、`stale_or_missing_run`、`invalid_activation`、`worker_unavailable`、`outcome_unknown`、`joint_limit` 或 `event_capacity`。拒绝响应不包含关节数组。工作进程每次运行最多保留 4096 个已保留事件 ID；达到上限后仍接受已知的重复 ID，但拒绝新 ID 且不会运动。工作进程不可用或结果不确定时返回 `503`，未知路由返回 `404`，正文超过 1 MiB 返回 `413`；工作进程把转发消息限制为 4 KiB。v1 路由和节点、数据的 `@v0.2` 标识分别独立演进。

<a id="arm_control_cn--post-v1result"></a>
### POST /v1/result

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "result",
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1"
}
```

执行完成后的成功结果示例（运行中的初始关节位置全为零）：

```json
{
  "schema_version": 1,
  "core": "eos",
  "ok": true,
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1",
  "command_id": "arm-click-1",
  "status": "succeeded",
  "joint_positions_rad": [0.05, -0.04, 0.03, -0.02, 0.01, -0.01]
}
```

EI 用恰好包含 `schema_version`（`1`）、`core`（`eos`）、`op`（`result`）、当前 `run_id`、`revision` 和 `event_id` 的正文读取相关结果。成功响应包含相同运行标识、`command_id` 和 `status`，状态为 `accepted`、`running`、`succeeded`、`failed`、`cancelled` 或 `outcome_unknown`；仅成功状态包含 `joint_positions_rad`。未知 ID 返回 `event_not_found`，停止或旧运行返回 `stale_or_missing_run`。EOS 监控采样独立于结果读取，慢读取者只取得最新观测。

工作进程退出后，所有者从运行日志读取 `/v1/result`。仅有保留或运行记录、没有终态记录时返回 `outcome_unknown`。使用同一运行日志重启时，工作进程恢复终态记录及位置，将中断命令标记为不确定，并在存在不确定结果时禁止进一步运动。执行队列受每次运行 4096 个事件的上限约束；重复 ID 不再入队。

<a id="arm_monitor_cn"></a>
<a id="arm_monitor_cn--eos-模拟机械臂观测"></a>
## EOS 模拟机械臂观测

[English](../../docs/eos/interfaces.md#arm_monitor) · [EOS 接口](interfaces_cn.md#readme_cn) · [机械臂控制](interfaces_cn.md#arm_control_cn)

已应用的 EOS 投影包含独立返回路径：零图输入观测源 `eos.hardware.mock.arm_observer@v0.2` → `eos.dispatcher.arm.monitor@v0.2` → `eos.skill.demo.arm_status@v0.2` → `eos.interface.egress.arm_status@v0.2`。每个端口名为 `observation`，采用事件交付，传递观测数据 `arm_monitor_state@v0.2`。三条内部边使用进程内契约 `{owner:eos,id:arm-monitor-local,version:1}`。出口使用 Unix socket 契约 `{owner:eos,id:arm-monitor-to-ei,version:1}`。EOS 在 Run 前要求这些节点及连接齐全。

观测源是 EOS 硬件层中的只读软件节点，没有图输入或执行器绑定；它读取唯一模拟机械臂适配器的周期样本，采样序号独立于激活事件递增。模拟机械臂仍是唯一执行器。旧的有输入 `eos.hardware.mock.arm_output@v0.2` 类型仍适用于使用该路径的已保存图；其监控载荷中的 `source_node` 仍为模拟机械臂路径。

默认情况下，模拟机械臂每 10 ms（100 Hz）采样并验证六个关节。其可选高级节点设置 `sample_interval_ms`（1–1000 ms，默认 10）和 `freshness_timeout_ms`（2–5000 ms，默认 50）要求新鲜度阈值至少为采样间隔的两倍。初始样本为六个零；每次准入的运动更新关节后立即采样。样本超过配置的新鲜度阈值时进入 `stale`；关节值非有限数或越界进入 `degraded`。两种状态都阻止运动，直到新的有效样本恢复 `ready`。Stop 关闭运动准入并报告 `stopped`。仅 `ready` 观测包含位置数组。

<a id="arm_monitor_cn--载荷"></a>
### 载荷

`arm_monitor_state@v0.2` JSON 对象恰好包含以下字段：

| 字段 | 类型及规则 |
| --- | --- |
| `run_id`、`revision` | 当前运行 ID 和 64 位小写修订摘要。 |
| `source_node` | 图绑定的观测源路径，例如 `eos/hardware/mock/arm_observer`。 |
| `sequence` | 此工作进程运行内的正整数样本序号。 |
| `sampled_at_monotonic_ns` | 非负单调采样时间，单位纳秒。 |
| `joint_names` | 按图顺序排列的六个唯一名称。 |
| `joint_positions_rad` | `ready` 时为六个有限弧度值；其他状态为 `null`。 |
| `state` | `ready`、`degraded`、`stale` 或 `stopped`。 |
| `reason_code` | 就绪时为 `null`；否则为 `invalid_joint_sample`、`sample_timeout` 或 `stopped` 等原因。 |

初始就绪样本示例：

```json
{
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "source_node": "eos/hardware/mock/arm_observer",
  "sequence": 1,
  "sampled_at_monotonic_ns": 1000000000,
  "joint_names": ["joint_1", "joint_2", "joint_3", "joint_4", "joint_5", "joint_6"],
  "joint_positions_rad": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
  "state": "ready",
  "reason_code": null
}
```

<a id="arm_monitor_cn--私有所有者读取"></a>
### 私有所有者读取

EI 在权限为 `0600` 的 EOS 所有者 Unix socket 上调用 `POST /v1/monitor`。精确请求为 `{ "schema_version":1, "core":"eos", "op":"monitor", "run_id":"run_1", "revision":"<64 位小写十六进制>" }`。EOS 要求匹配当前运行和修订，读取最新工作进程样本而不移动机械臂，返回 `{ "schema_version":1, "core":"eos", "ok":true, "run_id":"run_1", "revision":"<revision>", "observation":{...} }`。EI 可最多以 20 Hz 轮询；EOS 安全采样独立保持 100 Hz。工作进程不可用时返回 `worker_unavailable` 和 HTTP 503。运行隔离过期时返回 `stale_or_missing_run` 和 HTTP 422。Stop 使读取隔离失效。

<a id="arm_monitor_cn--私有模拟工作进程输出"></a>
### 私有模拟工作进程输出

EOS 所有者可通过权限为 `0600` 的工作进程 Unix socket，使用 `{"schema_version":1,"core":"eos","op":"observe_output","run_id":"run_1","revision":"<64 位小写十六进制>","node":"eos/hardware/mock/arm_observer","port":"observation"}` 读取已声明的观测源输出。七个字段均为必填，多余字段不予准入。`node` 必须等于该工作进程绑定的图观测源路径，`port` 必须为 `observation`；此操作没有执行器输入。成功响应为 `{"schema_version":1,"core":"eos","ok":true,"run_id":"run_1","revision":"<revision>","node":"eos/hardware/mock/arm_observer","port":"observation","payload":{...}}`，其中 `payload` 为上文定义的完整 `arm_monitor_state@v0.2` 观测源样本。所有者在转发图连接前必须验证外层和载荷的身份。运行、修订或请求格式过期或无效时返回 `observation_fence_stale`；节点或端口未声明时返回 `observation_route_unsupported`；Stop 时返回 `worker_stopping`；样本不可用时返回 `observation_unavailable`。每种错误响应都含 `schema_version:1`、`core:"eos"`、`ok:false` 和 `error`，不含载荷。此操作只读最新样本，不改变机械臂或命令日志；原有面向 EI 的 `monitor` 交换继续可用。

<a id="contracts_cn"></a>
<a id="contracts_cn--eos-契约源码"></a>
## EOS 契约源码

> 历史契约：当前按钮到模拟机械臂图未实现此接口。当前数据及命令路径参见[机械臂控制](interfaces_cn.md#arm_control_cn)；设置、Run、Status 和 Stop 参见[所有者生命周期](../isos/interfaces/server/eos_cn.md#owner_health_cn)。


[English](../../docs/eos/interfaces.md#contracts) · [接口目录](interfaces_cn.md#readme_cn) · [gRPC 与 IPC](../isos/interfaces/server/eos_cn.md#grpc_cn)

IsOS 在 `contract/proto/` 维护共享 EOS protobuf 与 JSON Schema 的权威源码。EOS 编译并据此校验，运行时仍负责硬件行为与 Dispatcher 准入。`gateway.proto`、`skills.proto` 与 `capabilities.proto` 定义当前 Rust 运行时使用的面向进程的服务消息。`robot/`、`perception/`、`skills/`、`actions/`、`body/` 和 `world/` 等领域目录定义载荷与校验注解。[Rust 技能可执行文件契约](interfaces_cn.md#skills_rust_cn)定义独立的工作进程协议。

生成器在 `contract/schemas/` 下生成 JSON Schema 与清单。protobuf envelope 提供传输身份；选定的生成 schema 校验有界参数与结果。图节点类型 ID、进程线协议模式、源码包版本和外部软件包版本是独立身份。

<a id="contracts_cn--更新契约"></a>
### 更新契约

修改所属 `.proto` 或手写 schema，重新生成派生产物，并同步更新接口页及英文镜像。运行：

```sh
python3 scripts/generate_eos_contracts.py --output contract/schemas
python3 scripts/generate_eos_contracts.py --output contract/schemas --check
cargo test --workspace
python3 scripts/check_doc_links.py --all
```

检查生成差异；公开契约变更应加入请求、成功、拒绝和故障夹具。传输边界以下的硬件样本与命令保持类型化 Rust 值。

<a id="general_hardware_cn"></a>
<a id="general_hardware_cn--eos-通用硬件声明"></a>
## EOS 通用硬件声明

[English](../../docs/eos/interfaces.md#general_hardware) · [EOS 接口](interfaces_cn.md#readme_cn) · [节点数据类型](../node/interfaces/type_cn.md#data_types)

下列七种类型由 MuJoCo 0.2.2 候选软件包声明，不再是 EOS 内建注册。已验证的软件包可以在 `eos` 核心内声明自有硬件类型并选择层和分组名称。EOS 保留共享负载模式及安全检查。当前 Linux v2 软件包服务路径可在 IsOS 验证精确软件包并注入运行绑定的服务端点后执行选定的相机、底盘、机械臂、夹爪和躯干节点；命令必须经 EOS Dispatcher 准入并使用持久身份。模拟机械臂适配器保留内建类型和负载。

| MuJoCo 软件包类型 | 放置路径 | 数据端口 |
| --- | --- | --- |
| `eos.hardware.camera.device@v0.2` | `eos/hardware/camera/<id>` | `image_frame@v0.2` 输出 |
| `eos.hardware.base.device@v0.2` | `eos/hardware/base/<id>` | `base_state@v0.2` 输出；可选的 `base_twist_command@v0.2` 输入 |
| `eos.hardware.manipulator.device@v0.2` | `eos/hardware/manipulator/<id>` | `joint_state@v0.2` 输出；可选的 `joint_position_command@v0.2` 输入 |
| `eos.hardware.base.normalized_axes@v0.2` | `eos/hardware/base/<id>` | `pose_state@v0.2` 输出；可选的 `normalized_axes_command@v0.2` 输入 |
| `eos.hardware.manipulator.normalized_cartesian@v0.2` | `eos/hardware/manipulator/<id>` | `pose_state@v0.2` 输出；可选的归一化轴 `command` 和 `control_mode_command@v0.2` `mode` 输入 |
| `eos.hardware.gripper.binary@v0.2` | `eos/hardware/gripper/<id>` | `gripper_state@v0.2` 输出；可选的 `gripper_binary_command@v0.2` 输入 |
| `eos.hardware.torso.normalized_axes@v0.2` | `eos/hardware/torso/<id>` | 可选的 `normalized_axes_command@v0.2` 输入；不伪造位置输出 |

实例指定设备身份与 `node.adapter.type`。由软件包提供的适配器系列还需要准确的 `node.adapter.package` 版本固定值。`node.adapter.settings` 包含由 EOS 验证的系列专有端点字段；软件包服务在此记录声明的服务实例。IsOS 验证版本固定值及进程资源声明。EOS 必须在报告就绪前检查已编译系列、设备类别、端点、进程就绪状态和停止能力。软件包元数据和节点设置不会变成硬件负载字段。底盘或机械臂在没有图生产者连接到命令输入时仍可发布观测。对于已选设备，即使没有连接生产者边，EOS 也会将每个已声明的执行器端口绑定到 Dispatcher；所有者命令 API 可以指定该端口。已选且已验证的 v2 软件包服务执行准入后的命令；未声明端口和不支持的适配器系列仍不可用。适配器或后端不得创建另一条命令路由。Stop 必须先关闭命令准入并使已准入的执行器静止，再退出任何选定的服务子进程。

<a id="general_hardware_cn--通用数据"></a>
### 通用数据

这十个数据 ID 与起始图的 `camera_frame`、`arm_joint_delta@v0.2` 和 `arm_monitor_state@v0.2` 含义不同。首版全部描述符使用 `version:"0.2"`、`media_type:"application/json"` 和 `codec:"json_v1"`。观测 `sample` 必须包含 `source_time_ns`（所声明时钟下的无符号纳秒数）、`clock`（`system`、`simulation` 或 `monotonic`）、`sequence`（无符号整数）和非空 `frame_id`（已声明坐标系）。这些字段没有默认值。结构化值使用有限数值和 SI 单位。命令身份、序号、截止时间、运行围栏和准入归属 Dispatcher 信封，不放入这些负载。

| ID | 负载含义 |
| --- | --- |
| `image_frame@v0.2` | `sample`、`width_px`、`height_px`、`encoding`（`jpeg` 或 `rgb8`）以及 JSON 中有界的 base64 `data`。EOS 验证尺寸和编码字节长度。 |
| `base_state@v0.2` | `sample`、`x_m`、`y_m`、`yaw_rad`、`vx_m_s`、`vy_m_s` 和 `yaw_rate_rad_s`。 |
| `joint_state@v0.2` | `sample`、有序 `joint_names`、`position_rad` 和 `velocity_rad_s`；EOS 检查向量长度相同。 |
| `base_twist_command@v0.2` | `frame_id`、`vx_m_s`、`vy_m_s`、`yaw_rate_rad_s` 和 `lease_ms`。EOS 检查坐标系和租约界限。 |
| `joint_position_command@v0.2` | 有序 `joint_names` 和 `position_rad`；EOS 检查名称、长度和位置界限。 |
| `pose_state@v0.2` | `sample`、三个米单位的 `position_m` 值及四个四元数 `orientation_xyzw` 值。适配器必须检查单位模长和声明的坐标系。 |
| `gripper_state@v0.2` | `sample` 和具名 `channels`，每项具有有限的 `value` 与明确的 `unit`。 |
| `normalized_axes_command@v0.2` | 非空的已声明 `axes` 映射，值在 `[-1,1]` 内，`lease_ms` 为 1–1000 毫秒。 |
| `gripper_binary_command@v0.2` | `action` 为 `open` 或 `close`。 |
| `control_mode_command@v0.2` | 非空 `mode`；适配器仅准入声明的模式。 |

这些类型声明图和载荷契约。Linux `isos_node_v2` 宿主负责观测，只有在 EOS 所有者经 Dispatcher 保留命令后才执行命令，并要求运行绑定的执行器 Stop 证明。IsOS 软件包服务移交为选定服务实例提供私有端点。仅凭声明绝不能使执行器就绪。

对于选定服务，IsOS 将精确软件包版本、`service_instance`、私有端点、代次及配置摘要绑定到运行。EOS 验证选定命令端口与载荷，经 Dispatcher 保留持久命令 ID，再携带私有运行令牌转发一次命令。重复 ID 仅在节点、端口与载荷一致时返回已保存结果；交付结果不明时禁止新命令。Stop 时 EOS 关闭准入，并要求适配器给出运行绑定的 `outcome:"stopped"` 证明，才能报告确认。参见[节点贡献契约](../isos/interfaces/server/eos_cn.md#node_contributions_cn)和[软件包检查契约](../isos/interfaces/package/README_cn.md)。

所列负载字段均为必填；缺失字段没有默认值。`width_px` 和 `height_px` 是正整数像素数；图像 `data` 是 base64 文本，完整 JSON 负载必须符合描述符的 `max_payload_bytes`。`rgb8` 每像素占三个字节；`jpeg` 字节必须解码为声明的尺寸。位置和速度使用米、弧度、秒及相应速率。`lease_ms` 是正毫秒时长。数值范围、允许的坐标系、关节数量及名称、命令频率和运动限值来自选定的注册类型与设备实例，并非通用硬件常量。EOS 运行时适配器必须拒绝缺失或未知时钟、非有限数值、向量不匹配、图像尺寸无效、未声明坐标系、超出范围的租约及超过描述符上限的负载。这些检查适用于可执行适配器边界；不支持的适配器系列和无效载荷均关闭准入。

一个底盘观测样例和一个独立的命令负载样例：

```json
{
  "sample": {
    "source_time_ns": 1000000000,
    "clock": "simulation",
    "sequence": 7,
    "frame_id": "map"
  },
  "x_m": 1.25,
  "y_m": 0.5,
  "yaw_rad": 0.1,
  "vx_m_s": 0.2,
  "vy_m_s": 0.0,
  "yaw_rate_rad_s": 0.05
}
```

```json
{
  "frame_id": "base_link",
  "vx_m_s": 0.2,
  "vy_m_s": 0.0,
  "yaw_rate_rad_s": 0.05,
  "lease_ms": 100
}
```

端口和边要求完全相同的数据 ID；IsOS 不隐式转换坐标系、单位或编解码器。不同表示需要显式注册的转换器。无效声明或不可用适配器使设置／Run 准入失败并返回所有者诊断，且不得报告就绪。运行时负载失败必须将设备标记为不健康并拒绝样本或命令；执行器 Stop 结果不确定时不得报告成功。这些负载含义参考 ROS 2 [JointState](https://github.com/ros2/common_interfaces/blob/rolling/sensor_msgs/msg/JointState.msg)、[CompressedImage](https://github.com/ros2/common_interfaces/blob/rolling/sensor_msgs/msg/CompressedImage.msg) 和 [REP 103](https://www.ros.org/reps/rep-0103.html) 的 SI 单位与坐标系；不声称兼容 ROS 消息或传输格式。归属的 EOS 适配器必须验证负载语义，Dispatcher 必须在运动前验证目标、限值、截止时间、健康状态和运行围栏。

<a id="isolated_library_cn"></a>
<a id="isolated_library_cn--eos-隔离节点库"></a>
## EOS 隔离节点库

[English](../../docs/eos/interfaces.md#isolated_library) · [EOS 接口](interfaces_cn.md#readme_cn) · [节点数据类型](../node/interfaces/type_cn.md#data_types)

EOS 目录包含七个可直接执行的内建类型，用于隔离节点库测试。它们列于 `catalog.direct_only_types`，不列于 `identity.supported_node_types`；普通图设置以 `isolated_node_graph_runtime_unavailable` 拒绝它们。每个输入在注册类型中均为可选，仅声明类型且没有目录路由。直接调用使用夹具测试所调用的 Rust 库 API。运行时所有者尚未提供调用路由。这些类型不改变现有的 17 节点初始图。直接调用者为每个处理器提供可信的 `run_id`。四个 `IsolatedNode` 处理器及注入的 Stop 处理器还接收以纳秒计的当前单调时间；这些值都不是图设置。Rust 处理器依据准确注册的 JSON v1 数据类型验证输入。无效调用返回有界错误且不产生输出。已停止的实例拒绝输入。提供 `read` 方法的处理器可返回最近的类型化输出，且不访问硬件。四个 `IsolatedNode` 处理器提供 `health_at`；当其最近源时间超过一秒时，它报告 `degraded` 和 `sample_stale`。Stop、记录器和清单处理器提供 `health`，不进行此新鲜度检查。`stop` 对本地非执行器处理器确认静止；紧急锁存状态由注入的 EOS Stop 服务保持。

| 类型 ID | 设置 | 输入 → 输出 | 行为 |
| --- | --- | --- | --- |
| `eos.hardware.mock.camera@v0.2` | 必需 `frame_id` | `sample: sample_request@v0.2` → `frame: image_frame@v0.2` | 输出固定的 2×2 RGB8 夹具，`clock:monotonic`；节点身份标记它为模拟，不声称为物理摄像头。 |
| `eos.perception.physical.features@v0.2` | 必需 `frame_id` | `state: joint_state@v0.2` → `event: physical_event@v0.2` | 任一关节位置绝对值 ≥3.0 rad 时报告 `detected`，否则报告 `clear`。 |
| `eos.dispatcher.command.binding@v0.2` | 必需 `frame_id`、`target_node` | `request: movement_binding_request@v0.2` → `binding: binding_result@v0.2` | 要求精确的 `arm.delta@1.0.0`、`joint_positions`、`rad`、六个 ±0.25 rad 范围内的增量和 `eos/hardware/mock/arm`；不下发运动。 |
| `eos.safety.device.guard@v0.2` | 必需 `frame_id` | `state: joint_state@v0.2` → `status: device_status@v0.2` | 超过 3.14 rad 时报告 `inhibited`，否则为 `ready`。Dispatcher 仍在写入时检查物理限位。 |
| `eos.safety.device.estop@v0.2` | 无图设置 | `stop: stop_request@v0.2` → `latch: stop_latch@v0.2` | 一次性委托给注入的 EOS Stop 服务；缺少服务时不可用。本地 Stop 不清除锁存。 |
| `eos.evidence.episode.recorder@v0.2` | 无图设置 | `record: evidence_record@v0.2` → `gap: gap_record@v0.2`；Stop → `episode: episode_reference@v0.2` | 将有界元数据写入所有者提供的只追加本地存储。超出字节预算时产生缺口。 |
| `eos.dataset.export.manifest@v0.2` | 无图设置 | `request: manifest_request@v0.2` → `manifest: dataset_manifest_reference@v0.2` | 仅解析所有者授权的 episode ID，并写入含源摘要和策略 ID 的确定性清单。 |

所有设置拒绝未知键。`frame_id` 是 1–128 个 ASCII 字母、数字、下划线、连字符或点。绑定目标必须精确为 `eos/hardware/mock/arm`。四个 `IsolatedNode` 处理器拒绝坐标系不符、未来源时间或超过一秒的输入；Stop 处理器检查源时间新鲜度，但不检查坐标系。记录器与清单输入检查运行及负载语义，不依据调用者当前时间检查新鲜度。传感节点还拒绝未递增的源时间；关节输入要求 `sample.clock:monotonic`。关节处理器拒绝关节名称、位置和速度向量长度不一致的输入。这些处理器解释调用者提供的样本；它们不证明样本来自实测硬件。

<a id="isolated_library_cn--数据负载"></a>
### 数据负载

每个新描述符使用 `version:0.2`、`media_type:application/json`、`codec:json_v1` 和 16,384 字节的负载上限。未知字段被拒绝。`sample_request@v0.2` 必须有 `source_time_ns`（非负单调纳秒整数）、`sequence`（正整数）、`frame_id`（非空字符串）和 `clock:monotonic`。`movement_binding_request@v0.2` 必须有 `source_time_ns`、`frame_id`、`target_node`、`primitive`、`mode`、`unit` 和 `joint_deltas_rad`（精确六个 ±0.25 范围内的有限弧度值）。primitive、mode 和 unit 是上表所列的精确常量。四个 `IsolatedNode` 处理器将源时间与调用者提供的当前单调时间比较；这些时间不是墙上时钟时间戳。

三个新输出都必须有 `run_id`。`physical_event@v0.2` 还包含 `source_time_ns`、`frame_id`、`source_sequence`、`event:joint_limit_proximity`、`state`（`detected`、`clear` 或 `unknown`）、`provenance:input_joint_state` 和 `rule_version:1`。`device_status@v0.2` 包含相同的源字段，以及 `state`（`ready`、`degraded` 或 `inhibited`）和可空的 `reason`。`binding_result@v0.2` 包含 `target_node`、`primitive`、`mode`、`unit` 和 `valid:true`；这仅证明本地绑定验证。字段都没有默认值。

```json
{
  "source_time_ns": 100,
  "frame_id": "arm",
  "target_node": "eos/hardware/mock/arm",
  "primitive": "arm.delta@1.0.0",
  "mode": "joint_positions",
  "unit": "rad",
  "joint_deltas_rad": [0.01, 0.0, 0.0, 0.0, 0.0, 0.0]
}
```

<a id="isolated_library_cn--stop-与证据归属"></a>
### Stop 与证据归属

`stop_request@v0.2` 必须有 `run_id`、`reason` 和 `source_time_ns`；`stop_latch@v0.2` 返回 `run_id`、`latched:true` 和 Stop 服务提供的有界 `proof_ref`。处理器不能重置锁存。服务错误不产生证明。生产所有者必须提供现有的运行绑定 Stop 服务，类型才能启动；目录声明本身不表示就绪。

`evidence_record@v0.2` 必须有 `run_id`、`record_id`、`source_time_ns`、`kind`（`sample`、`command`、`event`、`intervention`）和 `source_ref`，只携带元数据。重复 ID 和其他运行的记录被拒绝。`episode_reference@v0.2` 在确认刷盘后返回运行、episode ID、记录数量和只追加 JSONL 字节的 SHA-256。`gap_record@v0.2` 报告运行、episode、正数丢失计数及 `byte_budget` 或 `write_failure`；写入失败降低健康状态并返回带可见缺口计数的错误。存储根目录和字节预算由所有者注入，不是图设置。

`manifest_request@v0.2` 必须有 `run_id`、一个或多个包含 `episode_id`、`record_count`、`sha256` 的 episode 引用，以及 `split_policy_id` 和 `label_policy_id`。导出器根据所有者提供的授权集合检查每个 ID，读取有界 episode 字节，检查每条记录的运行和 schema，拒绝被更改的字节或数量，并记录源 SHA-256 以及确定性的 `train` 或 `test` 分组。`dataset_manifest_reference@v0.2` 返回运行、清单 ID、episode 数量和 SHA-256。重复的相同输入得到相同的字节和引用。清单把策略 ID 记录为标签来源；它不生成标签或上传数据。这三个处理器有直接测试夹具，但要在选中图中运行，仍须连接生产所有者的 Stop 服务和受治理存储。现有 arm Dispatcher 和所有者 Stop 仍是执行器与停止操作的权威。使用 `cargo test -p aether-node-profile --test isolated_library` 验证此直接边界；测试覆盖目录、输入验证、健康、拒绝、来源、缺口、确定性导出和停止。

<a id="isolated_library_cn--验证"></a>
### 验证

2026-11-15，`cargo fmt --all`、`cargo fmt --all -- --check`、`cargo test -p aether-node-profile`、`cargo clippy -p aether-node-profile --all-targets -- -D warnings`、`cargo build -p aether-runtime --lib`、`cargo test -p aether-runtime -- --test-threads=1`、`cargo clippy -p aether-runtime --bin aether-profile -- -D warnings`、`python3 scripts/check_doc_links.py --all` 和 `git diff --check` 均通过。完整输出保存在被忽略的 `data/test/20261115-eos-isolated-library/`。完整运行时测试前先构建运行时共享库，因为演示技能测试要加载相邻的共享库。串行运行测试可避免无关测试案例之间的日志所有者竞争。调用者审计保留现有机械臂、原生软件包、Dispatcher 和 Stop 路径；本切片不删除声称为死代码的内容。

<a id="primitive_bindings_cn"></a>
<a id="primitive_bindings_cn--eos-v2-原语绑定与类型化命令信封"></a>
## EOS v2 原语绑定与类型化命令信封

> 历史契约：当前按钮到模拟机械臂图未实现此接口。当前数据及命令路径参见[机械臂控制](interfaces_cn.md#arm_control_cn)；设置、Run、Status 和 Stop 参见[所有者生命周期](../isos/interfaces/server/eos_cn.md#owner_health_cn)。


[English](../../docs/eos/interfaces.md#primitive_bindings) · [硬件 profile](interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm) · [图控制](../isos/interfaces/server/eos_cn.md#control_cn)

EOS 原语绑定命名一个带版本的运动操作，并把它连接到恰好一个硬件设备。IsOS 图拥有绑定边；EOS 读取该边，校验类型化命令头，并通过共享 Dispatcher 传递解码后的 Rust 运动值。绑定本身不拥有驱动，也不授权直接释放执行器命令。

<a id="primitive_bindings_cn--图契约"></a>
### 图契约

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
| `node.adapter.package` | `{id:string,version:string}` 或缺省 | 软件包引用 | 软件包适配器族必填；精确匹配已验证软件包，与内置节点类型的 `node.package` 独立。 |


<a id="primitive_bindings_cn--有界运行时命令"></a>
### 有界运行时命令

已合并的 EOS R03 实现中仅所有者可访问的 Unix 控制 socket 接收最多 64 KiB 的单行 JSON。新绑定使用 `primitive_typed` 操作；`payload` 是[硬件 v2](interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm)定义的类型化运动对象。**私有** `payload_length_bytes` 统计此对象经解析后通过 `serde_json` 紧凑 UTF-8 序列化的字节数。回环 HTTP bridge 接收有界的公开紧凑 UTF-8 长度，在解析后规范化私有请求头，因为 JavaScript 和 Rust 对相同有限数字的序列化字节数可能不同；见[bridge 契约](../isos/interfaces/server/eos_cn.md#control_cn--回环-api)。解码器在请求 Dispatcher 准入前拒绝未知字段、超过 16 KiB 的载荷、不支持的编解码器或版本、伪造的图目标，以及原语与载荷类型不匹配。Dispatcher 继续校验健康、截止时间、限值、日志和停止。

已合并的 EOS R03 实现中，浏览器调用经现有回环 `/api/eos/v2/control` bridge：它允许 `primitive_typed`，检查 Origin 和大小后转发到此私有 runtime 操作。调用方可以发送 `deadline_after_ms`，由 EOS 根据自身单调时钟换算。继续使用绝对截止时间的调用方通过 bridge 获取 EOS 单调时钟时间。EOS 再次验证类型化头、精确活动绑定及目标。响应中的 `command_id`、精确的 `Completed`/`Cancelled`/`Rejected`/`InProgress`/`OutcomeUnknown` 状态和原因驱动面板结果显示；浏览器本地的发送状态仅表示等待。浏览器 bridge 将有界公开 `result` 改写为受保护的私有 `primitive_result`；[R07 授权结果契约](../isos/interfaces/server/eos_cn.md#control_cn--r07-浏览器授权结果后续查询)定义 UI 后续查询 `InProgress` 所需的有界且按权限校验的读取。不确定结果绝不显示为已完成。这是现有契约的 UI 入口，不是新图连线或 gRPC 服务名。

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

<a id="primitive_bindings_cn--流式运动"></a>
#### 流式运动

IsOS HTTP bridge 为每条 `base_movement` 命令发送 `delivery_mode:"stream"`、当前 `run_id`、已声明的 `gateway_path` 和 `deadline_after_ms`。EOS 仅允许 `body_velocity` 使用 stream，并在发布前检查活动修订、运行 ID 和图连线。后续 `primitive_result` 可能返回 `Completed`，原因是 `stream lease published`：这表示命令已交给硬件适配器，不表示实体已经静止。每条命令替换前一条有界租约；按键释放时，零速度命令会及时替换持续运动。该路径的 gateway 和 primitive binding 均声明 `max_rate_hz:20`。

<a id="scene_data_cn"></a>
<a id="scene_data_cn--eos-物理场景数据"></a>
## EOS 物理场景数据

[English](../../docs/eos/interfaces.md#scene_data) · [EOS 接口](interfaces_cn.md#readme_cn) · [通用硬件](interfaces_cn.md#general_hardware_cn)

EOS 注册九个用于隔离物理来源的 JSON v1 元数据和引用类型。它们只声明负载含义：这里不注册新节点、图路由、解码器、物理驱动或工件读取端点。生产者仍须单独证明其真实设备或所选软件包。引用携带所有者控制的工件 ID 和 SHA-256，不携带主机路径、凭据、套接字或内联媒体。来源对采集时间、坐标系、标定和有效性负责。EI 可以衍生任务层含义，但不改变这些 EOS 来源记录。

九个描述符均使用 `version: "0.2"`、`media_type: "application/json"`、`codec: "json_v1"` 和 `max_payload_bytes: 4096`。JSON 对象拒绝未知字段、缺失必需字段和非有限数字。每个负载都必须有 `sample`、`calibration_id` 和 `validity`；字段没有默认值。`sample` 必须有单调时钟中的非负 `source_time_ns`、`clock: "monotonic"`、非负 `sequence`、非空 `frame_id` 和 `source_id`。`calibration_id`、`source_id` 和工件 ID 是 1–128 个 ASCII 字母、数字、下划线、连字符或点。`validity` 为 `valid`、`invalid` 或 `unknown`；未知样本不会被默认为实测真值。调用者向 `validate_scene_payload` 提供当前单调时间、期望坐标系和标定。未来样本、超过一秒的样本或坐标系/标定不匹配都会在使用前被拒绝。

| ID | 额外必需字段和精确含义 |
| --- | --- |
| `depth_frame_ref@v0.2` | `artifact_id`、64 位小写十六进制 `sha256`、1–100,000,000 字节的 `byte_length`、1–8192 像素的 `width_px`/`height_px`、`encoding: "u16le"`、>0–1 m/unit 的 `metres_per_unit`、0–100 m 的 `uncertainty_m`。它引用像素，描述符不解码像素。 |
| `segmentation_mask_ref@v0.2` | 相同的工件和尺寸字段、`source_image_id`、`mask_kind: "binary"`、`encoding: "u8"`。它标识物理二值掩码，不包含语义类别标签。 |
| `point_cloud_ref@v0.2` | 相同的工件字段、1–10,000,000 的 `point_count`、`unit: "m"`、`encoding: "xyz_f32_le"`、0–100 m 的 `uncertainty_m`。没有隐式毫米换算。 |
| `physical_detection@v0.2` | `detection_id`、`kind`（`obstacle`、`person_proximity`、`unknown`）、三元素米制 `position_m`、0–1 的 `confidence`、0–100 m 的 `uncertainty_m`。它报告物理证据，不报告物体或任务身份。 |
| `imu_sample@v0.2` | 三元素 m/s² 的 `acceleration_m_s2`、三元素 rad/s 的 `angular_velocity_rad_s`、0–100 m/s² 的 `uncertainty_m_s2`。 |
| `force_torque_sample@v0.2` | 三元素牛顿制 `force_n`、三元素 N·m 的 `torque_nm`、0–100,000 N 的 `uncertainty_n`。 |
| `tactile_sample@v0.2` | `sensor_id`、1–32 个范围为 0–10,000,000 Pa 的 `pressure_pa` 值、相同范围的 `uncertainty_pa`。 |
| `contact_event@v0.2` | `event_id`、`body_a`、`body_b`、`state`（`began`、`ended`、`unknown`）、可空的 0–100,000 N `force_n`。null 表示未测量力。 |
| `audio_chunk_ref@v0.2` | 相同的工件字段、`encoding: "pcm_s16le"`、8,000–192,000 Hz 的 `sample_rate_hz`、1–8 的 `channels`、1–1,000 ms 的 `duration_ms`。不包含转录文本或语音含义。 |

以下是一个深度引用和一个物理检测示例：

```json
{
  "sample": {
    "source_time_ns": 100,
    "clock": "monotonic",
    "sequence": 7,
    "frame_id": "sensor_frame",
    "source_id": "sensor_1"
  },
  "calibration_id": "cal_1",
  "validity": "valid",
  "artifact_id": "depth_1",
  "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
  "byte_length": 8,
  "width_px": 2,
  "height_px": 2,
  "encoding": "u16le",
  "metres_per_unit": 0.001,
  "uncertainty_m": 0.01
}
```

```json
{
  "sample": {
    "source_time_ns": 100,
    "clock": "monotonic",
    "sequence": 7,
    "frame_id": "sensor_frame",
    "source_id": "sensor_1"
  },
  "calibration_id": "cal_1",
  "validity": "valid",
  "detection_id": "det_1",
  "kind": "obstacle",
  "position_m": [1.0, 0.0, 0.0],
  "confidence": 0.9,
  "uncertainty_m": 0.1
}
```

[夹具测试](../../crates/aether-node-profile/tests/scene_data.rs)为九个 ID 各提供一个有效示例，并拒绝缺失来源身份、坐标系/标定错误、过期源时间、超出限制的 JSON 大小、不安全的工件 ID，以及每个家族的错误单位、形状、范围或编码。运行 `cargo test -p aether-node-profile --test scene_data`。这些描述符本身不证明引用字节与摘要一致；受治理的工件读取器必须在打开工件前验证字节和授权。二进制编解码器和原始媒体传递需要单独的所有者接口和解码器。

对 `u16le` 深度图、`u8` 掩码和 `xyz_f32_le` 点云引用，`validate_scene_payload` 还要求 `byte_length` 分别等于宽 × 高 × 2、宽 × 高以及点数 × 12。音频字节长度受限，但不能从舍入到毫秒的时长推导。引用契约不提供工件读取器或物理采集路径。

在仓库根目录的验证：`cargo fmt --all`、`cargo fmt --all -- --check`、`cargo test -p aether-node-profile`、`cargo clippy -p aether-node-profile --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all` 均通过。完整命令输出保存在本地被忽略的 `data/test/20261115-eos-scene-data/`。

<a id="skills_rust_cn"></a>
<a id="skills_rust_cn--rust-技能可执行程序契约"></a>
## Rust 技能可执行程序契约

[English](../../docs/eos/interfaces.md#skills_rust) · [SkillService](interfaces/node/skill/demo_cn.md#skill_demo_cn) · [开发指南](core/skills/host-rust_cn.md)

EOS 在受限子进程中运行已选中的 Rust 技能可执行程序。作者实现 `aether_skill::Skill` trait，并在 `main` 中调用 `aether_skill::serve`。技能的 `Descriptor` 声明名称、SemVer 版本、说明、对象类型的参数和结果 JSON Schema、primitive 以及可重入性。一个可执行程序最多注册 256 个技能。名称使用小写字母、数字、`.`、`_` 和 `-`；分隔符后必须跟字母或数字。

编写接口见 [`crates/aether-skill/src/lib.rs`](../../crates/aether-skill/src/lib.rs)。`Skill::execute` 接收命令 ID、对象参数和 `Host`。`Host::call_primitive` 是唯一的执行器调用路径。返回值为一次 `Completion`，其结果是对象。`Host::is_cancelled` 表示 Stop 请求。EOS 在发布目录前验证描述符、Schema 和 primitive 声明，并在每次执行时验证参数和 broker 准入。

<a id="skills_rust_cn--图选择"></a>
### 图选择

EOS 进程节点可设置 `settings.skill_executables`，最多包含 16 个明确条目。每项包含 `package: {id, version}` 和以 `.skill` 结尾的相对路径 `executable`。IsOS 在 Apply 及 Run/Restart 前验证软件包、精确路径和摘要；它将 `isos/skill-selection.json` 写入选定 generation，并以 `EOS_SKILL_SELECTION_FILE` 向 EOS 传递路径。仅安装软件包不会启用技能。

选择文件包含整数 `schema_version: 1`、保存图的 SHA-256 `active_revision`，以及按软件包 ID、版本和可执行程序路径排序的 `executables` 数组。每项包含 `package_id`、`package_version`、绝对路径 `package_root` 和 `payload_root`、`receipt_sha256`、`release_sha256`、`payload_sha256`、相对路径 `executable` 和 `executable_sha256`。摘要均为小写十六进制 SHA-256。EOS 拒绝文件变化、路径逃逸、链接、非普通文件、不安全权限、不允许的 ELF 依赖和重复技能身份。它私有暂存已验证字节，并在发布目录前通过受限 worker 检查每个可执行程序。

运行时公布功能 `rust-skill-v1`。非空选择要求 IsOS 候选预检具备该功能。预检报告回显软件包身份、可执行程序路径、摘要和技能数量；SkillService 不暴露软件包代码路径或授权凭据。

<a id="skills_rust_cn--进程与协议"></a>
### 进程与协议

每次检查和执行都启动新的 `aether-runtime --skill-worker` 子进程。运行时只传递私有 `SOCK_SEQPACKET` socket 和暂存的可执行程序文件描述符。子进程清空环境、验证对端和描述符，设置资源限制、Landlock 和 seccomp，然后执行 Rust 程序。进程协议版本为 `2`，包含 `Inspect`、`Invoke`、`PrimitiveReply`、`Catalog`、`PrimitiveCall`、`Completion` 和 `Failure` 消息。消息有大小上限；大消息使用密封的内存文件。EOS 管理取消、有界回收、broker 授权和运行状态。

技能进程不能打开网络 socket 或创建其他进程。每个 primitive 请求均经过已准入描述符、图绑定、dispatcher 和期限检查。Stop 请求向进程发送信号；若进程未退出，EOS 会在限定时间内终止它。EOS 在终止选定的软件包服务子进程前停止已准入执行器。

<a id="skills_rust_cn--编写与验证"></a>
### 编写与验证

可运行的 [`echo` 示例](../../crates/aether-skill/examples/echo.rs)展示 trait 与完成结果。技能构建为 Linux 可执行程序，最终文件名以 `.skill` 结尾，并使用 Cargo 锁定构建。使用 Cargo 构建二进制文件，将其放入经 IsOS 验证的技能 release，并通过图选择精确路径。

执行 `cargo fmt --all -- --check`、`cargo test -p aether-skill` 和 `cargo test -p aether-runtime --features test-hooks --test rust_skill_worker --test rust_skill_catalog`。进程测试覆盖目录准入、变更 payload 拒绝、经 broker 的 primitive 调用、完成和子进程回收。
