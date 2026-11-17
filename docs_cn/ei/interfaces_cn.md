# EI interfaces

<!-- doc-navigation:start -->
<details>
<summary>目录：ei</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EI 文档](README_cn.md)
- 下级目录：
  - [core/](core/README.md)
  - [interfaces/](interfaces/)

</details>
<!-- doc-navigation:end -->

<a id="readme_cn"></a>
<a id="readme_cn--ei-外部接口"></a>
## EI 外部接口

[English](../../docs/ei/interfaces.md#readme) · [EI 架构](core/architecture.md)

本目录描述其他所有者和客户端可使用的边界。EI 的实现与算法位于[设计](core/architecture.md)。

| 契约 | 消费者与边界 |
| --- | --- |
| [配置](../isos/interfaces/server/ei_cn.md#setup_cn) | IsOS 的 `ei-config` 一次性及所有者套接字交换。 |
| [运行时](interfaces/server_cn.md#runtime_cn) | 面向操作员的 EI 运行时 WebSocket API。 |
| [节点](../isos/interfaces/server/ei_cn.md#node_cn) | IsOS 图节点引用与 EI–EOS 路径所有权。 |
| [事件](../isos/interfaces/server/ei_cn.md#events_cn)、[安全复位](../isos/interfaces/server/ei_cn.md#safety_reset_cn)、[读取特性](../isos/interfaces/server/ei_cn.md#reader_features_cn) | 外部事件和专题所有者交换。 |

<a id="readme_cn--节点层和组"></a>
### 节点层和组

- [io](interfaces_cn.md#readme_cn--io)
- [evidence](interfaces_cn.md#readme_cn--evidence)
- [cognition](interfaces_cn.md#readme_cn--cognition)
- [governance](interfaces_cn.md#readme_cn--governance)

<a id="readme_cn--io"></a>
### io

IO 阶段接收所有者边界输入，并按声明的连接与其他所有者通信。input 节点接纳触发和机械臂状态事件；link 组说明所有者交换网关。硬件控制仍须经过已声明的安全门与 EOS 分发器路径。

本层的节点组：

- [input](interfaces/node/io/input_cn.md#io_input_cn)
- [link](interfaces/node/io/link_cn.md#io_link_cn)

<a id="readme_cn--evidence"></a>
### evidence

证据层在认知处理之前接纳感知数据。perception 组说明接纳阶段及其边界；源载荷与外部契约仍由 EI 接口页面定义。

本层的节点组：

- [perception](interfaces/node/evidence/perception_cn.md#evidence_perception_cn)

<a id="readme_cn--cognition"></a>
### cognition

认知层将已接纳的观测和任务状态组织为有界上下文、决策与意图。这些阶段节点沿所选 EI 投影传递事件，不负责执行器准入。各组页面分别介绍上下文构建、规划、任务排队和机械臂状态投影。

本层的节点组：

- [context](interfaces/node/cognition/context_cn.md#cognition_context_cn)
- [decision](interfaces/node/cognition/decision_cn.md#cognition_decision_cn)
- [intent](interfaces/node/cognition/intent_cn.md#cognition_intent_cn)
- [missions](interfaces/node/cognition/missions_cn.md#cognition_missions_cn)
- [state](interfaces/node/cognition/state_cn.md#cognition_state_cn)
- [独立认知节点库](interfaces_cn.md#cognition_isolated_library_cn)

<a id="readme_cn--governance"></a>
### governance

治理层在前序阶段之后执行 EI 的安全与观测策略。安全门是激活请求到达 EOS 前的最后一道 EI 检查；monitor 阶段处理只读健康与机械臂状态观测。

本层的节点组：

- [monitor](interfaces/node/governance/monitor_cn.md#governance_monitor_cn)
- [safety](interfaces/node/governance/safety_cn.md#governance_safety_cn)

<a id="cognition_isolated_library_cn"></a>
<a id="cognition_isolated_library_cn--ei-独立认知节点库"></a>
## EI 独立认知节点库

[English](../../docs/ei/interfaces.md#cognition_isolated_library) · [认知层](interfaces_cn.md#readme_cn--cognition) · [节点契约](../isos/interfaces/server/ei_cn.md#node_cn)

EI 注册六个额外的 `@v0.2` 认知类型，用于直接调用单个节点。它们没有 `port_routes`、桥接适配器、跨核心投递或图执行路由。`ei-config catalog` 描述这些类型；图校验以 `isolated_node_only` 拒绝实例。现有 17 节点起始图保持不变。所有者或测试工具在调用处理器前校验配置和精确类型的输入。

| 类型 ID | 输入端口与数据类型 | 输出数据类型 | 可用性 |
| --- | --- | --- | --- |
| `ei.cognition.policy.system1_python@v0.2` | `input`：`grounded_goal@v0.2` | `motion_proposal@v0.2` | 需要已验证的模型提供者。 |
| `ei.cognition.policy.system1_native@v0.2` | `input`：`grounded_goal@v0.2` | `motion_proposal@v0.2` | 需要已验证的模型提供者；编译的 `isos_node_v1` 夹具经 EI 原生宿主测试；已安装软件包仍须完成资格验证。 |
| `ei.cognition.policy.vla@v0.2` | `input`：`vla_input@v0.2` | `grounded_goal@v0.2` | 需要已验证的模型提供者。 |
| `ei.cognition.world.predictor@v0.2` | `candidate`：`motion_proposal@v0.2` | `prediction@v0.2` | 确定性的影子基线。 |
| `ei.cognition.memory.remember@v0.2` | `record`：`memory_reference@v0.2` | `memory_reference@v0.2` | 有界的注入式认知存储。 |
| `ei.cognition.memory.recall@v0.2` | `query`：`memory_reference@v0.2` | `memory_reference@v0.2` | 有界的注入式认知存储。 |

模型类型要求 `model_resource_id`，即长度不超过 128 字符的非空资源选择器。提供者只有与匹配的资源 ID、精确资源文件及 SHA-256 摘要一起才能注入；构造及每次输入时均检查文件字节。缺少提供者时，`health` 返回 `unavailable` 和 `missing_model` 原因，`input` 拒绝。Python 与原生 System 1 类型共用输入、输出和拒绝测试。原生夹具测试 `isos_node_v1` 输入、输出和 Stop，但不证明已安装软件包合格或原生延迟界限。这里的节点均不调用执行器。

可选 `max_input_age_ms` 默认 5000，范围 1–60000 毫秒。`frame_id` 限制输入坐标系。`deadline_ms` 默认 1000，范围 1–30000 毫秒；提供者超时回复被拒绝。`max_output_bytes` 默认 65,536，范围 1–65,536 字节。`max_rate_hz` 默认 20，允许不超过 100 Hz 的正有限频率。`shadow_mode` 默认 true，预测器必须保持 true。记忆节点接受 `max_records`，范围 1–256，默认 32。未知配置、错误类型、非有限数值、过期或未来的源时间、已过期的目标或提议，以及超过 65,536 字节的载荷均被拒绝。目录中的所有输入端口都是可选声明；直接调用方选择一个端口和值。`read` 返回最后的类型化输出，不触发工作。`health` 包含代次计数和源时间。`stop` 关闭输入并确认本地静止；停止后的节点拒绝输入。

<a id="cognition_isolated_library_cn--数据类型"></a>
### 数据类型

六种直接 EI 认知数据类型均使用 `json_v1`、`application/json`、版本 `0.2`，最大 65,536 字节。每个对象必须包含 `source_id`（1–128 字符）、带时区偏移的 `source_time`、非负 `sequence` 和 `frame_id`（1–128 字符）。未知字段被拒绝。源时间早于配置的新鲜度界限或比当前时间晚超过一秒时被拒绝。不会隐式转换坐标系或单位。

| 数据类型 | 额外必需字段与界限 |
| --- | --- |
| `language_instruction@v0.2` | `text`（1–4096 字符）、`language`（2–16 字符）。 |
| `vla_input@v0.2` | 必须含 `image`、`language`、`robot_state` 对象。图像含资源 ID、SHA-256 摘要、源时间、坐标系、1–8192 像素宽高、`jpeg` 或 `rgb8` 编码以及标定 ID。语言含文本（1–4096 字符）和语言代码（2–16 字符）。机器人状态含源时间、坐标系、1–64 个不重复关节名、等量的有限 `position_rad` 值（±12.57 以内）和模型版本。图像与机器人状态各自必须新鲜且坐标系与外层一致。图像是有界引用，不内嵌字节。直接验证器检查引用元数据和摘要格式，但不读取图像字节，也不核验其摘要或标定。 |
| `grounded_goal@v0.2` | `goal_id`、`target_id`、`model_version`（各 1–128 字符）、`confidence`（0–1）、未来的 `expires_at`。 |
| `motion_proposal@v0.2` | `proposal_id`、`target_id`、`resource_id`、`model_version`（各 1–128 字符）、`horizon_ms`（1–1000）、`delta_rad`（1–32 个有限弧度值，各在 ±0.25 内）、未来的 `expires_at`。这是提议，不是命令。 |
| `prediction@v0.2` | `target_id`、有限的 `predicted_value`、`unit:"rad"`、非负 `uncertainty_rad`、`shadow_mode:true`。预测不替代 EOS 实测状态。 |
| `memory_reference@v0.2` | `record_id`、`policy_id`（1–128 字符）、小写 SHA-256 `digest`。查询必须与存储的策略 ID 匹配。此存储属于 EI 认知状态，与 EOS 物理数据集分离。 |

之前的六个流水线数据 ID（`resource_bundle`、`observation_set`、`context_snapshot`、`selected_mission`、`primitive_proposal`、`health_update`，均为 `@v0.2`）仍使用宽泛的对象模式。其现有阶段输出采用不同的旧载荷形状；只有在处理器及直接测试统一输出带来源信息且有界的传输值后，才能注册严格模式。本页的直接认知验证器不验证这六种值。

例如，直接预测器接受以下候选值，并输出带有相同来源身份、`unit:"rad"`、`uncertainty_rad:0.25` 和 `shadow_mode:true` 的 `prediction@v0.2`：

```json
{
  "source_id": "fixture-camera",
  "source_time": "2026-01-01T00:00:00+00:00",
  "sequence": 4,
  "frame_id": "base_link",
  "proposal_id": "proposal-1",
  "target_id": "arm-1",
  "resource_id": "joint-1",
  "horizon_ms": 100,
  "delta_rad": [0.1],
  "expires_at": "2026-01-01T00:00:02+00:00",
  "model_version": "fixture-v1"
}
```

输出值是影子预测，不是执行器命令：

```json
{
  "source_id": "fixture-camera",
  "source_time": "2026-01-01T00:00:00+00:00",
  "sequence": 4,
  "frame_id": "base_link",
  "target_id": "arm-1",
  "predicted_value": 0.1,
  "unit": "rad",
  "uncertainty_rad": 0.25,
  "shadow_mode": true
}
```

`ei/tests/ei/test_isolated.py` 是直接调用的夹具单元测试，覆盖模式准入、模型可用性、两个 System 1 类型的等价行为、VLA 输出拒绝、影子预测、记忆策略、健康状态和 Stop。`ei/tests/ei/test_isolated_native.py` 编译夹具 `isos_node_v1` 库，并经 EI 宿主检查原生输入、输出与 Stop。从仓库根目录运行 `PYTHONPATH=ei ei/.venv/bin/python -m pytest ei/tests/ei/test_isolated.py ei/tests/ei/test_isolated_native.py -q`。测试使用固定时钟，不需要物理设备或模型权重。

VLA 输入改动的检查：`PYTHONPATH=ei ei/.venv/bin/python -m pytest ei/tests/ei/test_isolated.py ei/tests/ei/test_isolated_native.py ei/tests/ei/test_stages.py -q` 通过 55 个测试。`ei/.venv/bin/ruff check ei/core/system/isolated.py ei/tests/ei/test_isolated.py ei/tests/ei/test_stages.py`、`ei/.venv/bin/pyright --project ei/pyproject.toml ei/core/system/isolated.py ei/tests/ei/test_isolated.py` 和导入约束均通过。完整历史日志位于已忽略的 `data/test/ei-vla-multimodal-20261115/`；新检查请使用 [EI 快速入门](../isos/start/quick-start_cn.md)中的当前导入约束命令。
