# 开发 EOS 硬件适配器

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EOS 硬件运行时设计](hardware_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/core/hardware/hardware-guide.md) · [硬件接口](../../interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm) · [运行时设计](hardware_cn.md) · [Dispatcher 设计](../dispatcher/dispatcher_cn.md)

从注册节点类型及其所属接口契约开始。当前演示的 EOS 硬件节点在图格式 `0.2.1` 中位于 `eos/hardware/mock/arm`。注册表声明能力、类型化端口、设置、界限、健康时序及适配器族；图实例提供身份和选定值。不要创建第二份清单或执行器路线。

## 修改或增加节点

1. 同步更新[硬件契约](../../interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm)及其英文镜像，说明每个公开字段的类型、单位、用途、默认值和校验。
2. 每个节点实现一个适配器。设备通信留在 Rust HAL 或其狭窄的 CXX/C ABI 边界后；传递有界的类型化样本与命令。
3. 创建设备前校验设置与引用。显式返回健康和状态，拒绝过期或畸形观察，并用持久命令 ID 关联每个命令。
4. 所有运动经 Dispatcher 准入。在适配器写入边界实现独立实体 Stop，并验证租约运动的看门狗。
5. 添加 mock 单元测试及集成边界，覆盖成功、拒绝、故障、重启和 Stop。

## 检查与日志

运行 `cargo fmt --all`、`cargo fmt --all -- --check`、工作区 Clippy、聚焦的 HAL/Dispatcher/运行时测试、适用的模拟适配器或硬件集成，以及 `python3 scripts/check_doc_links.py --all`。完整输出保存到忽略的 `data/test/<run-id>/`；在所属指南记录可复现命令与结果。基准测试先使用固定工作负载，报告吞吐量、中位数、p99 和环境，再确定性能目标。

2026-11-12，`cargo fmt --all -- --check`、聚焦的 `aether-profile` 设置与目录测试、`cargo clippy -p aether-runtime --bin aether-profile --offline -- -D warnings`、`cargo build -p aether-runtime --bin aether-profile --offline`、UI 类型检查、全部 218 项 UI 测试、Web 与 TUI 构建以及 `python3 scripts/check_doc_links.py --all` 均通过。针对已安装 MuJoCo `0.2.2` Native 图的实时 IsOS 选项请求返回四个可用的 `mujoco-v2` 适配器组；未修改的模拟图返回其 mock-arm 选项。输出位于已忽略的 `data/test/20261112-eos-package-adapter-options/`。较广泛的 EOS 二进制测试遇到 `config/graph-pool/demo.nodes.json` 中不相关的进行中编辑；该编辑添加了未完成的软件包相机，导致四个依赖此夹具的测试失败。该编辑已保留。

内置 EOS 目录直接由 `crates/aether-node-profile/src/registry.rs` 中当前的 `eos.*@v0.2` 节点定义构建。使用 `cargo test -p aether-node-profile` 验证注册表与边准入，使用 `cargo test -p aether-runtime --lib` 验证命令准入和运行时集成。

2026-11-07 的按所有者组织文件目录文档变更通过了 `python3 scripts/check_doc_links.py --all`（464 个 Markdown 文件，无失效相对链接）。完整输出位于已忽略的 `data/test/20261107-eos-paths/doc-links.log`。EOS 运行时路径由必填参数或显式参数注入；源码、便携式及系统安装的默认值由 IsOS 决定。

2026-11-08 的节点不匹配事件和侧文件位置对齐通过了 `cargo fmt --all -- --check`、`cargo test -p aether-common -p aether-runtime --bin aether-runtime --bin aether-profile`、`cargo clippy -p aether-common -p aether-runtime --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all`。完整输出位于已忽略的 `data/test/20261108-eos-node-alignment/`。

包贡献的节点类型同样使用 `isos.<layer>.<name>@v0.2` 身份，并提供非空描述及完整说明的类型化端口和设置。注册表在其他图类型身份进入目录前予以拒绝。

## 按钮到模拟机械臂演示，2026-11-09

EOS 投影包含一个实时入口、技能、调度器和模拟机械臂节点。模拟机械臂注册为机械臂适配器，并具有 `mock_arm` 安全适配器。工作进程在同一次运行中保存六个关节位置。内置技能将已准入的激活转换为固定相对增量，Dispatcher 在模拟适配器更新状态之前检查单次命令和结果位置的限值。线上的字段和单位见[机械臂控制契约](../../interfaces_cn.md#arm_control_cn)。

通过 `cargo test -p aether-node-profile`、`cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`、`cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和 `python3 scripts/check_doc_links.py --all` 验证。socket 集成测试覆盖 Run、两次激活、重复事件回放、过时修订版本拒绝及 Stop。完整命令输出保存于被忽略的 `data/test/20261109-eos-arm-demo/`。

## 模拟机械臂观测返回，2026-11-09

[机械臂监控契约](../../interfaces_cn.md#arm_monitor_cn)定义读取模拟机械臂采样器的四个返回路径节点、一种类型化观测载荷和 `POST /v1/monitor`。工作进程默认以 100 Hz 以及每次运动后立即采样。模拟机械臂高级设置公开 10 ms 采样周期和 50 ms 新鲜度阈值；有界的图覆盖值传至工作进程。过期或无效样本阻止运动，直到新的有效样本恢复就绪。

## 独立模拟观测源，2026-11-10

新演示以零图输入的 `eos.hardware.mock.arm_observer@v0.2` 作为返回链首节点。它读取现有适配器的周期采样器；命令链结束于唯一的模拟机械臂执行器。旧的有输入 `arm_output@v0.2` 路径仍适用于已保存图。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo test -p aether-node-profile --tests`、`cargo test -p aether-runtime --test arm_activation`、`cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all`。Profile 测试覆盖两种图路径，并拒绝观测源从命令链接收输入。所有者/工作进程 socket 测试证明激活前观测序号已经推进，运动后继续推进，并在 Stop 时结束。完整输出保存在忽略目录 `data/test/eos-observer-20261110/`。

从仓库根目录运行 `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo test -p aether-node-profile --tests`、`cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`、`cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all`。单元测试覆盖初始零位、过期和无效样本、恢复、运动后立即观测及 Stop。真实所有者与工作进程 socket 测试覆盖 Run、采样推进、监控读取、运动、过期修订拒绝和 Stop。完整输出保存于被忽略的 `data/test/20261109-eos-arm-monitor/`。

## 监控健康状态与时序控制，2026-11-09

模拟机械臂目录公开可选高级设置 `sample_interval_ms`（默认 10）和 `freshness_timeout_ms`（默认 50）。Profile 限制两者范围，并要求新鲜度阈值至少为采样间隔的两倍。EOS 将有效值传给工作进程。所有者 Health 与 Status 保留工作进程的 `degraded` 或 `stale` 状态；Monitor 仍可读取，运动保持阻止。通过 `cargo test -p aether-node-profile --tests`、`cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`、`cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和 `python3 scripts/check_doc_links.py --all` 验证。完整输出位于忽略的 `data/test/20261109-eos-monitor-health/`。

## 异步演示验证（2026-11-10）

运行 `cargo fmt --all -- --check`、`cargo clippy -p aether-runtime --all-targets -- -D warnings`、`cargo test -p aether-runtime` 及 `python3 scripts/check_doc_links.py --all`，均通过。真实所有者/工作进程 socket 测试验证接收回执、结果查询、重复 ID、监控及 Stop；持久保留单元测试验证写入日志及单次运动。完整输出保存在忽略目录 `data/test/async-eos-20261110/`。

## 文件验证 CLI（2026-11-11）

`aether-profile validate --file <graph>` 现在依据合并后的注册表验证仅含 EOS 核心的图投影，并执行与设置及 Run 相同的可执行模式准入检查。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo test -p aether-runtime --bin aether-profile --test profile_validate`、`cargo clippy -p aether-runtime --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all`，均通过。CLI 测试覆盖可执行演示、不完整链、不可执行 EOS 投影、多核心图及不可用侧文件。完整输出保存在忽略目录 `data/test/20261111-eos-profile-validate/checks.log`。


## 通用硬件契约阶段（2026-11-11）

在这一较早的通用契约检查点，注册表声明了有界的图像、底盘和关节观测、底盘速度与关节位置命令，以及通用相机、底盘和机械臂节点。`aether-profile validate --file <eos-projection> --node-types <verified-sidecar>` 只验证 EOS 图而不启动工作进程。当时 setup 拒绝执行这三类通用设备。下文的软件包观测实现仅允许已验证的摄像头观测；底盘和机械臂执行仍要求 Dispatcher 准入及实体 Stop 证明。

从仓库根目录执行 `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo test -p aether-node-profile`、`cargo test -p aether-runtime`、`cargo clippy -p aether-node-profile -p aether-runtime -p isos --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all`，均通过。配置测试覆盖缺失后端绑定、软件包／socket／设备一致性、schema 边界和仅允许 Dispatcher 发出命令；EOS 测试覆盖可执行演示图准入、拒绝尚不可执行的通用图、所有者／工作进程控制及 Stop。完整输出和重放脚本位于被忽略的 `data/test/20261111-general-hal/`。

## 外部软件包移交准入（2026-11-11）

EOS 所有者仅在 `owner --socket` 模式下接受 `--adapter-library family=<绝对路径 .so>@<sha256>` 和一个 `--package-process <绝对路径文件>`。启动时它重新计算每个适配器的摘要，与选中的侧文件比较，并按精确的软件包收据、发布摘要、受包含的可执行文件及所选资源摘要验证 IsOS 进程移交。重复系列或不一致在所有者监听前失败。直接子进程监督辅助函数建立独立进程组，即使所有者专属的 Stop 证明失败也会回收子进程；仅凭进程退出不会报告已确认 Stop。在这一交接检查点，通用硬件执行仍被拒绝。后续软件包观测阶段加入了摄像头采样和有界 Stop；执行器执行仍受限。从仓库根目录执行 `cargo test -p aether-runtime --bin aether-profile`、`cargo clippy -p aether-runtime --bin aether-profile -- -D warnings`、`cargo fmt --all -- --check` 和 `python3 scripts/check_doc_links.py --all` 验证。

## 软件包观测执行（2026-11-11）

所有者现在先启动一个已验证、仅用于观测的软件包进程，再启动原生相机工作进程；它从有界进程参数绑定 `${socket}` 和选中的资源路径，并公开 `/v1/observe`。工作进程调用适配器的 `isos_node_v1` 回调，EOS 验证已声明的输出端口及载荷 schema。Stop 要求适配器静止并确认进程组退出。执行器执行现经 Dispatcher 准入，并要求选定 v2 服务提供运行绑定的实体 Stop 证明。独立 ABI 探测命令为 `aether-runtime --probe-adapter <绝对路径 .so>`。参见[软件包观测契约](../../../isos/interfaces/server/eos_cn.md#package_observer_cn)。

从仓库根目录运行 `cargo fmt --all -- --check`、`cargo test -p aether-node-profile`、`cargo test -p aether-runtime --bin aether-profile`、`cargo test -p isos --bin isos pkg_` 和 `python3 scripts/check_doc_links.py --all`，可复现这一边界。用候选软件包自身的 `v0.2.x` 脚本构建后，运行 `isos pkg check --source <绝对源码路径> --version <精确版本> --output <空的绝对目录> --graph <候选图>`。检查要求隔离 ABI 探测及托管 Run、观测、日志与已确认 Stop 重放。完整输出保存于忽略的 `data/test/<run-id>/`。

## 通用软件包 v2 声明及 ABI 检查点（2026-11-12）

EOS 目录现在除现有 SI 底盘及关节位置设备外，还注册归一化底盘与笛卡尔机械臂接口、二值夹爪及归一化躯干。节点 profile 读取器接受带精确软件包固定版本的有效 `adapter.settings.service_instance`，并拒绝图作者写入 `socket_path`；IsOS 必须在 Run 时注入私有端点。Linux `isos_node_v2` 宿主探测 ABI，将观测与命令分开调用，并检查含选定执行器结果、带运行围栏的停止证明。在这一较早的 ABI 检查点，宿主在所有者接通 Dispatcher 预留和持久日志前拒绝 v2 命令。当前所有者现已提供该绑定。现有 v1 软件包路径仍可用。

对于相机来源选择，EOS `options` 现在会在可执行配置验证前读取尚未完成的硬件实例，返回与准确软件包版本匹配的适配器选项。IsOS 从经过验证的软件包节点集读取相机来源配置，公开服务和设备选项，并在标记服务可用前检查其软件包配置和主机前提条件。编辑器没有默认相机。选择设备会复制其声明的坐标系与适配器字段，同时保留实例身份和连线。当前 MuJoCo `0.2.2` 发布声明原生 `cam-0` 和三个 RoboCasa 相机。在确认主机路径存在后，已从配置完成的 `0.2.1` 版本将本机准确版本 `0.2.2` 的软件包配置导入。验证命令与完整输出位于忽略的 `data/test/20261112-mujoco-camera-options/`。软件包观测桩通过了帧传递与 Stop。隔离的候选版本回放对原生与 RoboCasa 图均完成 Apply，观测到原生 JPEG 帧与 RoboCasa 腕部 RGB 帧，并对两者确认 Stop；但整个 package-check 命令在这些回放记录后以失败状态退出，未生成最终 `check.json`，因此不能据此认定候选版本通过发布资格检查。

从仓库根目录运行 `cargo fmt --all -- --check`、`cargo test -p aether-node-profile --tests`、`cargo test -p aether-runtime --bin aether-runtime native_node`、`cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`、`cargo check --workspace` 和 `python3 scripts/check_doc_links.py --all`，均通过。完整输出位于忽略的 `data/test/20261112-eos-full-hal/`。在此检出中运行完整的 `aether-runtime` 二进制测试集时，无关的现有测试 `reserved_event_is_durable_and_repeated_once` 失败，随后的测试遭遇已中毒的测试锁。本段记录较早的 ABI 检查点；后续完整托管软件包资格检查已覆盖 Dispatcher 路由和 IsOS 服务移交。


## 完整托管软件包资格检查（2026-11-12）

重放命令为 `isos pkg check --source <绝对干净源码路径> --version 0.2.1 --output <空的绝对目录> --package-config <绝对package.toml> --graph <绝对native-graph.json> --graph <绝对robocasa-graph.json>`；随后检查 `check.json`、`qualification.json` 和每个 `graph-*/managed-replay/` 目录。独立的精确版本隔离测试同时安装并验证 `0.2.0` 和 `0.2.1`，只为 `0.2.1` 导入配置，分别移除任一版本而不改变另一版本的摘要；本地记录位于 `<qualification-checkout>/data/test/two-version-isolation/README.md`。

## 图中的 Vision 传输

将已选择的软件包相机 `frame` 输出连接到 `eos.interface.egress.vision_flow@v0.2` 的 `frame` 输入，再将输出连接到 IsOS UI Vision 监视节点。EOS 将传输作为标准接口拥有，不把它放进原生插件 worker。私有 owner 的 `vision_frame` 请求检查已应用的相机绑定和运行身份，通过已验证的原生相机适配器观察，并返回一帧经校验的图像。`cargo test -p aether-runtime --test package_observer` 用真实 owner 与原生 ABI 测试成功传输、错误来源拒绝和 Stop。

2026-11-12 的隔离重放使用 `island/mujoco-sim@0.2.2` 的原生 `cam-0`：`isos config validate` → `isos config apply --revision <sha256>` → `isos run` → `GET /api/isos/v1/vision/bindings` → 一帧图绑定的 WebSocket 图像 → `isos deployment stop`。重放获得带运行栅栏的 640×480 JPEG，并确认 Stop；之后绑定接口返回 503。可按忽略目录 `data/test/20261112-eos-ui-vision-flow/replay.txt` 中的准确命令重放并查看完整日志。格式检查、Clippy、Rust 聚焦测试、UI 类型检查／测试／构建和中英文链接审计均通过。跳过三个把用户尚未完成的本地相机草稿误当成固定示例图的现有测试后，工作区测试套件通过；完整输出保存在同一证据目录。

## 模拟工作进程观测交换（2026-11-12）

模拟工作进程通过只读私有 `observe_output` 交换公开已声明观测源的样本，供所有者跨工作进程路由。复现命令为 `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo build -p aether-runtime --lib`、`cargo test -p aether-runtime --bin aether-runtime`、`cargo clippy -p aether-runtime --bin aether-runtime -- -D warnings` 和 `python3 scripts/check_doc_links.py --all`。这些检查通过，完整输出保存在忽略的 `data/test/20261112-eos-mock-worker-exchange/`。在全新工作树中必须显式构建库，因为现有示例技能测试会加载同目录下的 `libaether_runtime.so`，而仅测试二进制文件的命令不会构建该库。构建库前的首次失败输出保留为 `bin-tests.log`；通过的完整测试输出为 `bin-tests-after-lib.log`。

## 混合模拟机械臂与软件包相机重放（2026-11-13）

修订号为 `5175494c07b46f31979582b1c39082d48d268631e4ea7d6c7dd26386a88ab386` 的检查点 19 节点图，使用已验证的 `island/mujoco-sim@0.2.2` 软件包通过了隔离的设置与 Apply。Run 时 EI、EOS、模拟工作进程、原生工作进程和 RoboCasa 服务子进程均达到就绪。图绑定的 Vision WebSocket 送达带运行栅栏的 `robocasa/camera/agentview-right` 图像；按钮激活成功，六个关节位置为 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`，机械臂监控返回相同位置。读取日志后，`isos deployment stop` 返回 `outcome:"confirmed"`；最终状态没有活动运行或子进程，选中的服务和工作进程 PID 均已退出。`ISOS_HOME` 应使用较短路径：第一次证据目录使 `ei-owner.sock` 路径达到 108 字节，所有者服务无法绑定。稳定的 16 节点测试夹具保留纯演示回归覆盖，仓库图池保存这个混合示例；全新的 `isos config init` 仍创建无需软件包的起始图。`ISOS_AETHER_BIN_DIR=$PWD/target/debug RUST_TEST_THREADS=1 cargo test --workspace`、`cargo clippy -p aether-runtime -p isos --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和 `python3 scripts/check_doc_links.py --all` 均通过。准确命令、HTTP 响应、完整图像和日志输出、Stop 证明及测试输出保存在忽略的 `data/test/2026-11-12-eos-mixed-replay/evidence/`。

原生 v2 Dispatcher 回执边界通过 `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo build -p aether-runtime --bin aether-runtime --lib`、`cargo test -p aether-runtime --bin aether-runtime`、`cargo clippy -p aether-runtime --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all` 验证，均通过；完整输出位于忽略的 `data/test/20261113-native-command-result/`。工作进程 socket 测试覆盖私有令牌准入、过期运行拒绝、唯一有效回执、格式错误和重复输出、保留图端口及 Stop 证明。在新工作树中，首次完整二进制测试早于同目录库的构建，因此演示激活测试失败；构建库后通过的重跑记录为 `runtime-test-after-build.log`。

2026-11-13 的 RoboCasa 2048 像素边界更新把内置图像 JSON 上限提高到 20 MiB、原生节点图传递预算提高到 24 MiB。`cargo fmt --all -- --check`、受影响 crate 的严格 Clippy、完整的 `aether-node-profile`、运行时及 IsOS 二进制测试（设置 `ISOS_AETHER_BIN_DIR`）、UI 类型检查／测试／本地与托管 Web 和 TUI 构建，以及双语链接审计均通过。2048 × 2048 的 RoboCasa 合成帧及更大的通用 RGB 帧通过 Vision 验证器。完整输出位于忽略的 `data/test/20261113-robocasa-2048/`。匹配的软件包候选版发布前仍需通过托管 `pkg check`。

## 仿真器契约清理（2026-11-14）

已移除未被使用的 EOS 仿真器 protobuf，以及 IsOS 中的 `adapter.settings.simulator_node` 设置选项特例。通用托管服务子进程和模拟硬件仍有覆盖。从仓库根目录运行 `cargo test -p isos --bin isos managed_process_admission_is_general_across_owners`、`cargo build -p aether-runtime --lib`、`cargo test -p aether-runtime --test package_observer`、`python3 scripts/check_doc_links.py --all` 和 `git diff --check` 可复现聚焦检查。IsOS 测试通过，两项 package observer 测试通过；双语链接审计检查了 367 个 Markdown 文件，没有失效的相对链接。完整输出保存在忽略目录 `data/test/20261114-eos-simulator-removal/`。首次运行 package observer 时，同目录原生库尚未构建，测试失败；显式构建库后通过。随后运行 `cd ui && npm ci --ignore-scripts --no-audit --no-fund` 安装 UI 依赖，catalog 测试、完整 UI 测试（236 项）、类型检查，以及本地、托管和 TUI 构建均通过；`cargo clippy --workspace --all-targets -- -D warnings` 与 `ISOS_AETHER_BIN_DIR=$PWD/target/debug RUST_TEST_THREADS=1 cargo test --workspace` 在执行 `cargo build -p aether-runtime --bins --lib` 后也通过；首次完整工作区测试因缺少这些二进制文件失败。输出位于已忽略的 `data/test/20261114-ei-layer-refactor/`。

## 来源频率与 Vision 池化交付（2026-11-15）

相机输出端口可选的官方 `frequency_hz` 用于检查和诊断，目标输入端口的 `desired_rate_hz.max` 可限制 Vision 交付。v1 或 v2 原生 Vision 来源需要在 `node.adapter.settings.frequency_hz` 中设置为 `(0, 1000]` Hz，才能调度采集并写入来源池。EOS 所有者检查已声明的 Vision 传输节点，并转发其最新的有效样本；浏览器不会触发采集。可用 `cargo fmt --all -- --check`、`cargo clippy -p aether-node-profile -p aether-runtime -p isos --all-targets -- -D warnings`、`cargo test -p aether-node-profile --tests`、`cargo build -p aether-runtime --lib`、`cargo test -p aether-runtime --test package_observer`、`ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --bin isos`、`cargo test -p isos --test edge_admission`、`python3 scripts/check_doc_links.py --all` 复现验证，并在 `ui/` 执行 `npm run typecheck`、`npm test` 和 `npm run build`。这些检查均通过；UI 测试通过 226 项。完整输出位于已忽略的 `data/test/2026-11-15-source-frequency-pools/`。在全新 worktree 中运行 package observer 测试前，需要先构建同目录运行时库。
