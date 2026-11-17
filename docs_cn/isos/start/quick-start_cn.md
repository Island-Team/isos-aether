# 首次使用

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/start</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/isos/start/quick-start.md) · [节点契约](../interfaces/node/README_cn.md)

`isos config init` 创建的无软件包起始图包含三个核心和 17 个节点。IsOS 按钮的激活事件依次经过 EI 输入、认知、安全，以及 EOS 入口、演示技能、调度器和模拟六关节机械臂。只读观测路径将同一机械臂的状态经 EOS 和 EI 返回 IsOS 监控节点，同时提供给 EI 上下文节点。可选的[混合示例](../../../examples/mixed-mujoco.nodes.json)另有软件包相机、EOS 视觉流和 UI 视觉监控，共 17 个节点，控制路径故意缺少激活源，无法发送机械臂命令；Apply 前必须备好已验证的 `island/mujoco-sim@0.2.4` 发行包及其配置。

1. 构建 IsOS、`aether-profile` 和 `aether-runtime`，并按 `ei/pyproject.toml` 安装 EI 程序。将 `ISOS_AETHER_BIN_DIR` 与 `ISOS_EI_BIN_DIR` 指向选定的可执行文件目录。
2. 运行 `isos config init`。它将无软件包起始图保存在所选 IsOS 主目录的 `config/graph-pool/demo.nodes.json`；部署只保存图库名称绑定。仓库[演示图](../../../config/graph-pool/demo.nodes.json)在起始图上增加原生 MuJoCo 相机、EOS 视觉流和 UI 视觉监控。使用它时，先安装或链接已验证的 `island/mujoco-sim@0.2.6` 发行包并导入所需配置，再将仓库演示图复制到该主目录的图库文件中，然后验证并 Apply。独立的[混合示例](../../../examples/mixed-mujoco.nodes.json)使用 `0.2.4`，且没有激活源。

3. 验证并 Apply 该图库文件。Apply 的 `--revision` 必须是其当前字节的精确 SHA-256；`config apply --file` 只接受图库中的文件。
4. 用 `isos web` 打开工作台（或用 `isos serve` 在前台启动服务器）。IsOS 在后台启动持久运行的 EI 与 EOS 所有者服务，内部目录加载期间便显示初始连接状态；此时不启动部署工作进程。拖动节点只修改浏览器保存的画布布局。准备执行时，在工作台使用 **保存并运行**，或明确运行 `isos start` / `isos run`。随后查看 `isos status --json`：EI 应为 `running`，EOS 应为 `ready`，EOS 模拟适配器应为 `ready`。
5. 使用无软件包起始图时，在工作台点击按钮节点上的 **Activate**，或运行 `isos demo activate demo-1`、`isos demo result demo-1` 和 `isos demo monitor` 进行纯命令行回放。混合示例没有按钮激活源，因此其不完整控制路径保持不活动。按钮先显示已接受的事件 ID，然后单独读取关联结果；每个获准且成功的不同激活事件使模拟机械臂按固定的小关节增量移动。检查最终位置、独立的机械臂监控快照和归属方日志。然后运行 `isos deployment stop` 停止两个部署工作进程；所有者服务继续可供设置及后续 Run 使用。按 Ctrl+C 关闭前台 `isos run` 会话。`isos stop` 使用已配置的口令执行工作区紧急停止。

`config init` 不会覆盖已有主目录图。迁移旧的 `demo` 图库文件时，先备份，安装其固定版本的软件包后用仓库演示图替换，或在编辑器中创建**新建图**并保存，然后验证新修订并 Apply。

仓库演示图的相机类型与适配器均固定使用 `island/mujoco-sim@0.2.6`。原生前方相机请求以 30 Hz 生成 1280×720 像素图像，并声明 JPEG 能力。相机直接从所选 MuJoCo 场景输出 JPEG；此图未使用 RoboCasa 相机预设。

2026-11-17 的本机隔离严格套接字源测试在多次运行中于 0.88–1.07 秒内返回 31 个不同捕获时间戳的 1280×720 JPEG 帧（新鲜帧速率为 28–34 FPS）。可在 MuJoCo 源码工作树中使用 `tests/vision_rate.py` 复现。另一项隔离的 EOS 到 WebSocket 测试在 3.115 秒内传送了 72 个不同帧（22.79 FPS）。因此，30 Hz 是请求的捕获和传输速率，不代表此主机上已验证的端到端保证。运行部署时，Vision 页面显示实际接收帧率。

若验证报告 `generated_missing_input`，括号中会显示故障阶段。`binary` 阶段检查所选归属进程可执行文件和 EI 源码；仅包含已忽略字节码的可选 EI 源码目录不再阻止图验证。从仓库根目录运行 `./isos.sh config validate config/graph-pool/demo.nodes.json` 可复现加载检查。修复空目录问题后，该命令于 2026-11-16 返回 `valid node graph`；`ISOS_AETHER_BIN_DIR=$PWD/target/debug RUST_TEST_THREADS=1 cargo test -p isos --offline` 全套测试、聚焦回归测试、格式检查及严格 IsOS Clippy 均通过。输出位于已忽略的 `data/test/20261116-demo-load/checks.log`。2026-11-17 使用 `ISOS_AETHER_BIN_DIR=$PWD/target/debug RUST_TEST_THREADS=1 cargo test -p isos --offline`、`./isos.sh config validate config/graph-pool/demo.nodes.json` 及 `ui/` 目录下的 `npm run typecheck`、`npm test`、`npm run build`、`npm run build:tui` 检查相机频率编辑器与相机来源查找，全部通过。完整输出位于已忽略的 `data/test/20261117-camera-frequency/`。

修改选定的核心二进制后，先重新 Apply 图，再 Run，以更新经过验证的启动输入摘要。IsOS 退出后 EI 和 EOS 所有者仍可运行；重启 IsOS 并读取状态，即可按保留的运行身份重新附着。工作台日志区从 IsOS 的持久日志池重放选定的所有者启动和运行记录。某个核心失败时会单独报告，健康核心保持运行。参见[运行时设计与重放](../core/server_cn.md)及[图 API](../interfaces/api/http_cn.md#http_graph)。

对于带配置描述的软件包，在 Web 工作台打开 **运行概览 → 软件包配置**。选择已安装软件包的精确版本，编辑表单字段或切换到原始 TOML 视图，然后点击 **检查配置** 和 **保存配置**。如果只安装了一个可配置的软件包，面板会自动打开。保存会更新本地配置修订；运行前须重新 Apply 选定的节点图。

添加软件包节点时，先安装或链接精确发行包，再从节点库选择其注册类型，或预览节点集。软件包类型显示在其声明的核心、层和分组下；保存的实例带有顶层 `package` 锁定。对于新硬件节点，先选择适配器族，再填写必需的设备字段和服务实例。草稿尚未完整时，工作台可以显示已验证目录中的选项；Apply 前 EOS 会检查完整草稿。选择软件包适配器时会设置准确的 `adapter.package` 版本固定值。类型声明缺失或软件包字节变化时，Apply 会拒绝。旧版 MuJoCo v0.2.1 发行包未声明硬件类型；软件包自有硬件路径需要使用通过验证的后续发行包。

要检查本机已安装软件包的更新，运行 `isos pkg update`。它只检查这些软件包 ID，并列出与已安装主/次版本线相同的较新、连续且已批准的补丁发行版；不下载工件。跨版本线或有缺失补丁时，使用 `isos pkg install` 明确安装。运行 `isos pkg upgrade <命名空间/名称>` 安装其中一个，或运行 `isos pkg upgrade` 处理全部。按提示确认每个精确工件预览。验证后的新版本与旧版本并存；在适用处修改图中的 `package` 与 `adapter.package` 版本固定值，验证并 Apply 后，后续 Run 才会使用新版本。若检查更新后本地已安装清单发生变化，请再次运行 `isos pkg update`。产品更新使用独立的 `isos update` 与 `isos upgrade` 命令。

如果较新软件包仍只是本地受检候选包，在其精确版本进入已批准目录前，`pkg update` 不会列出它。发布者须连续发布通过检查的补丁版本：若最新已发布版本为 `0.2.1`，须先分别检查并发布 `0.2.2`，再发布 `0.2.3`；每个版本都有匹配的不可变标签和资格证明。见[软件包发布契约](../interfaces/package/README_cn.md#软件包准入与发布规则)。安装任一版本后，图的精确版本固定值仍保持不变。

仅编辑节点图而不运行部署时，请使用 `isos web`。`isos run` 是会明确启动两个部署工作进程的前台命令；结束其终端会同时结束该工作台服务器。独立启动的 `isos web` 服务器会在编辑节点图期间保持可用。仅使用 CLI 重放时，运行 `isos run --no-ui`：它启动管理器、本地 API 和部署，不打开浏览器，也不启动 UI 监听器。`isos serve --no-ui` 只启动管理器和 API；随后可用 `isos start` 运行已 Apply 的图。无界面的按钮事件和监控请求使用与工作台相同的[本地 HTTP API](../interfaces/api/http_cn.md)。

## EI 准备与诊断

产品安装器 `./install.sh` 会连同 IsOS 和 EOS 构建并安装 EI。仅构建 EI wheel 时运行 在仓库根目录运行 `./scripts/ei/platform/dispatch.sh build`。作者编写的 EI TOML 位于 `config/ei/`，工厂默认值位于 `config/defaults/ei/`，安装后的配置位于 `etc/ei/` 和 `etc/island.toml`。`EI_ETC` 或 `ei/bin/ei --etc <path>` 可选择其他配置根目录；`--mode` 为本次调用选择 `config/ei/modes.toml` 中的模式。API 密钥应放在已忽略的仓库根目录 `.env` 或导出的环境变量中；[`.env.example`](../../../.env.example)只列变量名，现有进程变量优先。凭据不应进入 TOML 或日志。

IsOS 选择 EI 程序，向 `ei-config` 发送有界设置请求，并监管运行时。受管设置只接受 EI 的独立核心投影；设置健康检查本身不能证明 EI 或 EOS 已就绪。在受管运行之外进行源码诊断时，从仓库根目录运行 `ei/bin/ei --mode sandbox`。`--managed` 仅供 IsOS 使用，并拒绝内部启动的沙盒。启动器选项见 `ei/bin/ei --help` 和 `ei/bin/ei-config --help`；线协议细节由 [EI 设置](../interfaces/server/ei_cn.md#setup_cn)及[运行时](../../ei/interfaces/server_cn.md#runtime_cn)契约定义。

独立运行的 EI 诊断位于 `data/ei/log/local/ei.jsonl`，受管运行位于 `data/ei/log/<deployment>/ei.jsonl`。IsOS 从本地文件读取日志面板；EI 所有者回复传递生命周期状态，不转发日志。`EI_LOG_DIR` 和 `EI_DEPLOYMENT` 可选择私有诊断根目录与部署。有限时本地检查可运行 `timeout 8s ei/bin/ei --mode sandbox`，随后运行 `tail -n 30 data/ei/log/local/ei.log`。若 EOS 发现缺失或不完整，应先检查 EOS 健康及 EI 链路诊断，再重试；EI 必须获得完整发现代才能准入命令。可选模型、召回或技能的降级不会放宽安全约束。备份 `data/ei/state/` 和用户文件；仅在 EI 停止后清理缓存或临时文件。

进行 EI 开发检查时，在 `ei/` 运行 `uv sync --extra dev --extra vec`，再运行 `uv run ruff check .`、`uv run pyright`、`PYTHONPATH=.:package uv run lint-imports --cache-dir ../data/cache/tools/import-linter-ei` 以及针对性或完整的 `uv run pytest`。EI 源码包直接位于 `ei/` 下；私有外部包源码位于 `ei/package/`。安装后的 wheel 与私有模块使用软件包中的 `pkg` 目录。完整验证输出保存在已忽略的 `data/test/<run-id>/`。

<a id="arm64-build"></a>

## EOS 构建与 ARM64 设置

从仓库根目录运行 `cargo build -p aether-runtime --bins -p aether-server` 和 `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`。IsOS 使用 `aether-profile` 所有者 socket 进行设置，并以同目录的 `aether-runtime` 二进制文件启动工作进程。在 ARM64 上，使用受支持的 Rust 工具链和 CXX 硬件适配器边界所需的平台 C/C++ 编译器构建 Rust 工作区。运行时可执行文件为 `aether-runtime`、`aether-profile`、`aether-server`；IsOS 将其作为私有归属程序启动。使用 `isos tui` 检查已应用的部署。实体适配器在部署前须通过类型化样本、健康、命令边界、取消及 Stop 的硬件集成测试。

<a id="configure-eos"></a>

## EOS 适配器与技能

对于 EOS 软件包适配器，在 IsOS 图中选择其精确固定版本、声明的资源 ID 和所属核心提供的选项。EOS 通过与内置适配器相同的[硬件接口](../../eos/interfaces/node/hardware/mock_cn.md#hardware_mock_cn)验证系列、设备身份、命令端口和 Stop 能力。IsOS 在 Apply 时渲染仅含 EOS 的投影；EOS 只读已应用图。对于 Rust 技能，使用 `aether_skill::Skill` 构建 `.skill` 可执行文件，将其放入已验证软件包，并通过 EOS 进程节点的 `settings.skill_executables` 选择。IsOS 检查精确字节；EOS 在受限工作进程中执行，并通过 Dispatcher 代理原语调用。参见 [Rust 技能编写指南](../../eos/core/skills/host-rust_cn.md)。

<a id="debug-eos"></a>

## EOS 检查、故障与 Stop

先查看 `./isos.sh status --json` 和活动图修订。对于起始图，EOS 有一个持久的 `aether-profile` 所有者及一个 `aether-runtime` 机械臂工作进程。IsOS 读取 `data/eos/log/<deployment>/eos.jsonl` 更新日志面板；直接阅读可使用配套的 `eos.log`。独立运行的 EOS 使用 `data/eos/log/local/eos.jsonl`，绝对路径 `EOS_LOG_DIR` 可选择其他目录。所有者回复传递 Run、Status、激活和 Stop 结果。每条 JSONL 记录包含 UTC `timestamp`、小写 `level`、`source` 和 `message`；来源包括 `eos.aether_profile.owner@aether-v0.2.3`、`eos.aether_runtime.worker@aether-v0.2.3` 及 `eos.hardware.mock.arm@aether-v0.2.3`。应比较同一次运行的状态与日志；单条日志不能证明就绪。

若 **Activate** 失败，请比较事件 ID、运行修订与所有者状态。过时的身份在移动前被拒绝。`invalid_eos_graph` 表示投影缺少所需链或连线；`joint_limit` 表示下一增量超过配置的模拟机械臂限值或 ±3.14 rad；`outcome_unknown` 表示工作进程可能已经执行动作，因此不得以新的事件 ID 自动重试。成功结果返回六个以弧度计的模拟关节位置。不同的已接受激活会累积小增量；重复同一事件 ID 不会再次移动。EI 独立于 UI 读取者刷新机械臂观测，并在返回路径上传递新鲜度、健康和位置。精确值由[机械臂控制](../../eos/interfaces_cn.md#arm_control_cn)和[机械臂监控](../../eos/interfaces_cn.md#arm_monitor_cn)契约定义。

普通受管 Stop 使用 `isos deployment stop`，随后检查 `isos status --json`。EOS 关闭准入、等待工作进程退出，并为匹配的运行 ID 与修订提供终态证据；已失败的子进程仍会被回收。超时、断连、取消或未经确认的 Stop 都使物理结果保持未知，直到 EOS 提供终态结果。仅在先前的所有者运行停止后再次 Run。`isos stop` 是独立的工作区紧急停止命令。模拟机械臂不连接实体硬件。

实现细节见 IsOS 的[安装器](../core/config_cn.md)、[配置](../core/config_cn.md)和[日志](../core/logger_cn.md)设计，以及 [EI 架构](../../ei/core/architecture.md)和 [EOS 架构](../../eos/core/architecture_cn.md)。对外的精确行为仍由 [IsOS HTTP](../interfaces/api/http_cn.md)、[EI](../../ei/interfaces_cn.md#readme_cn)和 [EOS](../../eos/interfaces_cn.md#readme_cn)契约定义。[发布者身份验证](../interfaces/package/README_cn.md#authentication)与本地图控制分离。
