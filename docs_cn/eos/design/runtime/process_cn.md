# Rust v2 进程主干


`aether-runtime` 是持久 EOS 所有者在 IsOS 部署编排下启动的私有 Rust 工作进程。它读取并校验一份 `1.0.0` 统一节点 profile，为每个 EOS 硬件节点创建隔离的适配器，打开独占的持久化命令日志，再绑定本地 Unix socket。全部成功后才报告就绪。内置适配器为 `mock-cxx` 与 `mujoco-v2`；实体 ROS 2 与厂商设备尚未准入。


## 启动和本地控制字段

| 字段 | 类型 | 用途与规则 |
| --- | --- | --- |
| `--profile` | UTF-8 文件路径 | 提供一份有大小上限的统一节点 profile；根字段 `version` 必须为 `"0.2.0"`。 |
| `--journal-dir` | 目录路径 | 保存持久化命令预留和结果；单一进程持有独占锁。重启测试应使用持久存储。 |
| `--socket` | Unix socket 路径 | 本地控制端点；绑定失败则不报告就绪。应通过文件系统权限限制访问。 |
| READY `version` | 字符串 | 标识进程协议版本 `"0.2.0"`。 |
| READY `status` | 字符串 | 仅在 profile、适配器、日志和 socket 全部准入后为 `"ready"`。 |
| READY `socket` | 字符串 | 已绑定的控制 socket 路径。 |
| READY `hardware_nodes` | 无符号整数 | 已注册分发的 EOS 硬件节点数；软件节点保留在 profile 中，但没有适配器。 |
| 请求 `op` | 字符串 | 选择 `discover`、`health`、`time`、`dispatch`、`result`、`stop` 或 `shutdown`。 |
| 请求 `node_path` | 字符串，`health`、`dispatch` 和 `stop` 必填 | 完整的 `cores/.../node/...` 路径。短 ID 和软件节点路径不能控制硬件。 |
| 请求 `command_id` | 字符串，`dispatch` 和 `result` 必填 | 持久化幂等键；重放相同请求可取得此前结果。 |
| 请求 `deadline_monotonic_ns` | 无符号 64 位整数，`dispatch` 必填 | 进程本地的绝对截止时间。先通过 `time` 获取当前值，在到期前提交；重启后重放时仍使用原始值。 |
| 请求 `movement` | 带类型标签的对象，`dispatch` 必填 | `body_velocity`、`relative_pose`、`joint_positions` 或 `end_effector_pose`，字段见[硬件类型接口](../../interfaces/hardware_cn.md)。节点 profile 必须准许该模式与限值。 |
| 响应 `ok` | 布尔值 | 表示操作是否完成。 |
| 响应 `nodes` | 数组，`discover` 成功时 | 从已准入 profile 派生的硬件节点描述。 |
| 响应 `health` | 对象，`health` 成功时 | 包含 Rust 侧观测故障的实际适配器健康信息。 |
| 响应 `monotonic_ns` | 无符号 64 位整数，`time` 成功时 | 用于计算命令截止时间的进程本地时间，不得与其他进程时钟比较。 |
| 响应 `result` | 对象，`dispatch` 或 `result` 成功时 | 持久化命令 ID、结果状态和可选原因。被拒绝的命令仍是已记录的分发结果。 |
| 响应 `error` | 字符串，失败时 | 人类可读的校验或操作错误，不作为稳定错误码解析。 |

socket 每连接处理一条有界的逐行 JSON 请求并返回一条响应。每条请求上限为 64 KiB。此接口用于本地集成和生命周期测试。`dispatch` 沿用分发器的完整路径、类型化运动、截止时间和持久化日志。重放时语义字段（包括绝对截止时间）必须相同；日志确保重启后不再次下发执行器命令。

## 验证和下一道门槛

进程测试应覆盖拒绝错误 profile、不支持的适配器、日志重复持有、就绪状态、发现、mock 命令、结果查询、停止、关闭、信号处理及重启时不重放命令。输出保存到 `data/test/<run-id>/`。实体部署仍须在真实控制器上证明独立停止和速度租约到期停止。

## 已验证软件包交接

使用 `isos packages` 安装或链接精确、已验证的发布版本。保存的图在选定节点上固定软件包版本并指定稳定资源；安装本身不会选择运行进程。IsOS 在 Apply 前以及 Run 或 Restart 前检查软件包元数据、载荷摘要、节点贡献和资源。EOS 接收已验证的进程与资源交接，启动直接仿真器子进程，并按所有者契约报告就绪和 Stop。

对于旧软件包贡献拓扑，节点类型须匹配 `isos.eos_process.<name>@v0.2` 位置和精确软件包固定版本。硬件节点通过自己的设置引用选定的适配器软件包。参见 [IsOS 发布契约](../../../isos/interfaces/packages/release_cn.md)与[验证契约](../../../isos/interfaces/packages/verification_cn.md)。
