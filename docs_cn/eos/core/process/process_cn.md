# Rust v2 进程主干

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core/process</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)

</details>
<!-- doc-navigation:end -->




`aether-runtime` 是持久化 EOS 所有者启动的私有 Rust 工作进程。它接收一份已验证的节点 profile，仅构建选定的适配器，打开持久命令日志，并在就绪检查后绑定仅所有者可访问的控制 socket。Dispatcher 始终是唯一命令路由。

## 启动和本地控制字段

| 字段 | 类型 | 用途与规则 |
| --- | --- | --- |
| `--profile` | UTF-8 文件路径 | 提供一份有大小上限的统一节点 profile；根字段 `version` 必须为 `"0.2.1"`。 |
| `--journal-dir` | 目录路径 | 保存持久化命令预留和结果；单一进程持有独占锁。重启测试应使用持久存储。 |
| `--socket` | Unix socket 路径 | 本地控制端点；绑定失败则不报告就绪。应通过文件系统权限限制访问。 |
| READY `version` | 字符串 | 标识进程协议版本 `"0.2.1"`。 |
| READY `status` | 字符串 | 仅在 profile、适配器、日志和 socket 全部准入后为 `"ready"`。 |
| READY `socket` | 字符串 | 已绑定的控制 socket 路径。 |
| READY `hardware_nodes` | 无符号整数 | 已注册分发的 EOS 硬件节点数；软件节点保留在 profile 中，但没有适配器。 |
| 请求 `op` | 字符串 | 选择 `discover`、`health`、`time`、`dispatch`、`result`、`stop` 或 `shutdown`。 |
| 请求 `node_path` | 字符串，`health`、`dispatch` 和 `stop` 必填 | 完整的 `cores/.../node/...` 路径。短 ID 和软件节点路径不能控制硬件。 |
| 请求 `command_id` | 字符串，`dispatch` 和 `result` 必填 | 持久化幂等键；重放相同请求可取得此前结果。 |
| 请求 `deadline_monotonic_ns` | 无符号 64 位整数，`dispatch` 必填 | 进程本地的绝对截止时间。先通过 `time` 获取当前值，在到期前提交；重启后重放时仍使用原始值。 |
| 请求 `movement` | 带类型标签的对象，`dispatch` 必填 | `body_velocity`、`relative_pose`、`joint_positions` 或 `end_effector_pose`，字段见[硬件类型接口](../../interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm)。节点 profile 必须准许该模式与限值。 |
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

使用 `isos packages` 安装或链接精确、已验证的发布版本。保存的图在选定节点上固定软件包版本并指定稳定资源；安装本身不会选择运行进程。IsOS 在 Apply 前以及 Run 或 Restart 前检查软件包元数据、载荷摘要、节点贡献和资源。EOS 接收已验证的进程与资源交接，启动选定的直属服务子进程，并按所有者契约报告就绪和 Stop。

对于旧软件包贡献拓扑，节点类型须匹配 `isos.eos_process.<name>@v0.2` 位置和精确软件包固定版本。硬件节点通过自己的设置引用选定的适配器软件包。参见 [IsOS 发布契约](../../../isos/interfaces/package/README_cn.md#release)与[验证契约](../../../isos/interfaces/package/README_cn.md#verification)。

<a id="eos-运行时基础"></a>

## EOS 运行时基础

本页说明当前 Rust 基础并保留带日期的验证记录。后面的设计章节同时记录已实现的图、就绪、路由和停止行为，以及尚待完成的实体适配器工作；契约由[硬件](../../interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm)、[节点贡献](../../../isos/interfaces/server/eos_cn.md#node_contributions_cn)、[控制](../../../isos/interfaces/server/eos_cn.md#control_cn)与 [gRPC](../../../isos/interfaces/server/eos_cn.md#grpc_cn) 页面负责。

### 边界与故障规则

- 节点类型定义模式及层级位置；节点实例定义身份和设置。类型化连线声明数据流；只有 dispatcher 可以放行执行器命令并持久记录结果。
- IsOS 启动 EOS；EOS 启动所选且已验证的工作进程，并等待其私有就绪证明，然后报告自身就绪。必需工作进程故障时 EOS 退出，IsOS 可按有限重启策略重新启动 EOS 组件。
- EOS 在绑定控制套接字前取得命令日志的所有权。重启时仅当路径确实是 Unix 套接字且没有进程接受连接时清理旧路径。可能已执行但结果未知的命令不会自动重放。
- EOS 正常关闭时向所有已准入执行器发送停止。实际硬件部署仍须测量控制器停止时限。
- HTTP bridge 和只读配置服务仅绑定回环地址。bridge 不直接访问适配器，只转发有界请求。

### 0.x 运行时设计与后续工作





R05 证明严格身份及同一子进程内的传输重发处理。R06 所有者实现提供停止准入栅栏、有界的直接工作进程停止通道、底盘/机械臂分别确认或持久未知，以及停止尝试之后才终止/回收。精确提交及最终负例跨所有者重放覆盖选定路径；完整 R06 资格验证仍需完成[停止设计](process_cn.md#托管例行关闭与持久安全状态)记录的实时故障。

<a id="rust-eos-服务"></a>

## Rust EOS 服务

持久化 EOS 所有者监管运行时工作进程。IsOS 提供已验证的图投影和所选软件包交接；EOS 构建声明的适配器，通过 Dispatcher 准入命令，并报告活动运行的健康与 Stop。



| 可执行文件 | 调用 | 职责 |
| --- | --- | --- |
| `aether-profile` | `validate --file <graph>` | 准入前验证图和已注册节点类型。 |
| `aether-server` | `--socket <path> --listen <控制回环地址:端口> --profile <graph> --catalog-listen <目录回环地址:端口>` | 在一个进程的两个回环监听器上提供有界控制、健康、视觉和只读图目录路由。 |

`aether-system::io::control` 提供服务器使用的有界私有客户端。控制请求上限为 64 KiB、响应上限为 16 MiB，socket 读取截止时间为两秒。`capture` 以 `rgb8_base64` 返回 RGB8，以可选的 `depth_u16_le_base64` 返回深度；`observe` 返回类型化底盘或机械臂数据。bridge 在转发前验证请求大小和身份。Dispatcher 始终是唯一执行器准入与释放路线。

### Rust skill worker

运行时只加载 IsOS 选中并验证的原生 Rust `.skill` 可执行文件。它私有暂存已验证字节，每次检查或调用都启动新的受限 `aether-runtime --skill-worker` 子进程。worker 接收私有协议 socket 与暂存可执行文件描述符，然后施加资源限制、Landlock 和 seccomp。Primitive 调用经宿主 broker 和 Dispatcher 返回；worker 不持有直接适配器句柄。EOS 负责取消、有界回收和实时执行状态。详见 [Rust skill 契约](../../interfaces_cn.md#skills_rust_cn)。

### 首次使用与检查

在仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings` 和 `RUST_TEST_THREADS=1 cargo test --workspace --no-fail-fast`。使用 `isos serve` 启动 IsOS 管理器，再用 `isos run` 运行已应用部署；通过 `isos status --json` 和 `isos logs eos --lines 200` 检查。使用 `isos deployment stop` 停止部署，或使用 `isos stop` 停止管理器。直接调用服务适用于聚焦的边界测试；生产图所有权和软件包解析由 IsOS 负责。

针对 2026-11-16 的 crate 布局变更，`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`python3 scripts/check_doc_links.py --all`、`git diff --check` 和 `python3 -m unittest crates.isos.tests.test_product_installer`、`python3 tests/reader_features_cli.py` 均通过。新服务端的 12 项测试覆盖两个监听器及 SIGTERM；设置 `ISOS_AETHER_BIN_DIR="$PWD/target/debug"` 后，72 项 IsOS 进程测试通过。完整运行 `ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --no-fail-fast` 时，两个未修改的夹具边界失败：两项 `package_observer` 用例在进入运行时前因帧数据类别不匹配而拒绝，仓库演示图的相机宽度为 128，而对应 IsOS starter 测试预期 1280。完整输出保存在忽略的 `data/test/20261116-eos-crate-layout/` 下。

<a id="托管例行关闭与持久安全状态"></a>

## 托管例行关闭与持久安全状态

### 问题与边界


实现应区分归属方请求的**托管例行关闭**与真正的 `safety.estop`。不新增公开重置操作、图字段、授权凭据、执行器入口或协议标识。真正的急停锁存保持持久；Stop、Restart、普通 `robot.stop` 或新的所选服务代际都不能清除它。崩溃、结果不明的运动释放、物理停止或子进程回收继续以拒绝运行为默认结果。

### 状态转换

1. 在停止执行器前，EOS 关闭运动和证据接纳，并设置 Dispatcher 内存抑制。托管例行路径记录已接纳的运动目标。日志增加绑定机器人的 `1.1.0` 版 `shutdown_pending` 标记及所选 `shutdown_targets`；待处理为真当且仅当目标非空。读取器先严格按原有体形状及校验和验证 `1.0.0` 控制记录，再赋予假/空默认值；仅在下次成功修改控制状态时升级体形状与校验和。重新打开时，待处理标记的目标须匹配已接纳的机器人和配置。该标记与现有 `robot_inhibited` 恢复状态及 `estop_latched` 分离；新 EOS 进程看到未解决标记时禁止运动。不得覆盖既有原因或把它视为本次关闭所创建。
4. 仅在上述条件均成立且没有尚在执行的公开停止操作时，EOS 才可利用已证明的更新物理停止 epoch，持久栅栏终态结果不明的运动并保留其机器人抑制，再在同一日志锁下解除**本次关闭标记及其目标列表**。非终态或旧格式未解决运动仍阻止解除。写入失败时回执为未知。此操作绝不修改 `robot_inhibited` 或 `estop_latched`；两者均可独立保持为真。当前进程直至退出仍保持关闭和内存抑制，绝不恢复运动。随后只发布一次所选 stdout 原始回执；IsOS 仍须核验严格回执和 EOS 成功退出，才能启动新运行实例。
5. 任何条件未知时，关闭标记保持持久；迟到的适配器结果不得清除它。若真正的 `safety.estop` 在关闭前或关闭期间锁存，即使例行物理停止和子进程回收都已确认，锁存仍为真。新 EOS 运行实例继续把运动标为不可用，直到另行规定并授权安全恢复。


### 归属方实现的限定范围证据（2026-11-02）


从此工作树根目录运行以下可复现命令：

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
cargo build --workspace
git diff --check
```

均已通过；完整输出保存于 `data/test/20261102-managed-shutdown/`。此处不作吞吐量或延迟声称。
