# IsOS 服务端

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS 核心](README_cn.md)
  - [IsOS 架构](architecture_cn.md)
  - [IsOS 命令行](cli_cn.md)
  - [IsOS 配置](config_cn.md)
  - [本地核心日志](logger_cn.md)
  - [IsOS 软件包](pkg_cn.md)
  - [IsOS 路线图](roadmap_cn.md)
  - [IslandOS 服务](service_cn.md)
- 下级目录：
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

## 目录

- [IsOS 服务端与所有者](#isos-服务端与所有者)
- [服务端与启动器设计](#服务端与启动器设计)
- [IsOS 本地服务器](#isos-本地服务器)
- [三核心按钮运行时](#三核心按钮运行时)
- [紧急关闭](#紧急关闭)
- [回放证据](#回放证据)
- [IsOS 监控](#isos-监控)
- [IsOS UI 节点](#isos-ui-节点)
- [按钮分组](#按钮分组)
- [触发节点](#触发节点)
- [监控分组](#监控分组)
- [机械臂监控节点](#机械臂监控节点)
- [展示选择](#展示选择)
- [视觉分组](#视觉分组)
- [视觉监控节点](#视觉监控节点)
- [Vision 面板消费者](#vision-面板消费者)
- [验证](#验证)
- [当前设计文档对齐（2026-11-10）](#当前设计文档对齐2026-11-10)
- [异步演示 API 与 UI（2026-11-10）](#异步演示-api-与-ui2026-11-10)
- [软件包配置编辑（2026-11-12）](#软件包配置编辑2026-11-12)
- [节点目录验证记录](#节点目录验证记录)
- [核心文档布局（2026-11-16）](#核心文档布局2026-11-16)
- [文档布局验证（2026-11-16）](#文档布局验证2026-11-16)
- [文档验证，2026-11-16](#文档验证2026-11-16)


[English](../../../docs/isos/core/server.md)

## IsOS 服务端与所有者

[English](../../../docs/isos/core/server.md) · [核心索引](README_cn.md)

## 服务端与启动器设计

本页面向贡献者说明当前源码边界。[HTTP 契约](../interfaces/api/http_cn.md)和[归属方契约](../interfaces/manager_cn.md)定义公开字段；[验证](server_cn.md)记录检查结果。

```text
src/
  main.rs               启动应用
  app/                  CLI、本地 HTTP、UI 包
    server/routes/       按领域划分的 HTTP 处理器
  domain/               图、注册表、软件包、启动决策
  runtime/              manager IPC、子进程、归属方交换、日志
  interfaces/           有类型的边界值
```

`app` 验证身份并解析请求，再把决策交给 `domain`，把实时操作交给 `runtime`。`domain` 验证精确版本和注册声明，但不拥有进程。`runtime` 串行化 manager 变更，只监管 IsOS 直接启动的子进程，并保存运行及停止状态。`interfaces` 保存交换值，而不是第二份状态。这种分工使软件包验证和修订检查不落在传输处理器中，并在 HTTP 边界统一映射有类型错误。

当前 Start/Restart 包装器包含保存修订、有序进程规格、已解析输入和归属模式。监管器创建并绑定运行 ID；就绪与停止仍归 `runtime` 负责。下文分别说明软件包准入和输出选择；未来的统一准入计划见[架构目标](roadmap_cn.md#目标架构尚未实现)。

根目录 `isos.sh` 在进入 Rust CLI 前选择 `dev` 或 `product`。`dev` 使用本仓库的调试版程序、EOS 程序、EI 程序、UI 构建、配置和便携式主目录；`ISOS_HOME` 更改主目录，显式配置环境变量更改所选文件。`product` 调用配置安装目录中的已安装启动器，并在没有显式覆盖时选择源码仓库的配置与主目录。直接运行已安装启动器时，若未选择主目录，还会根据当前工作目录识别源码仓库。操作命令见[安装](config_cn.md)。

CLI 入口先加载运行配置与路径，再分派命令。直接执行 `isos`、`web`、`tui` 会确保设置完成，并复用或自动启动服务。仅当没有 manager IPC 时，`run` 才在前台启动服务及部署；否则要求现有 manager 启动部署。`serve` 启动控制面，并在后台启动持久运行的 EI 与 EOS 所有者服务，但不启动其部署工作进程。`start` 要求 manager 已运行。

复用服务前，CLI 检查回环 HTTP 健康状态、服务标识和 API 模式，再通过所选的仅所有者可访问 Unix 套接字取得 manager 身份并比较 API 来源。开发模式还比较可执行文件路径和 SHA-256，防止重新构建的本地程序静默复用旧 manager。没有健康服务时，CLI 拒绝仍活动的孤立 manager 套接字，以当前程序启动脱离终端的 `serve` 子进程并保存其输出，在配置的启动超时内等待 HTTP 健康状态及 manager IPC。启动超时只说明未观察到就绪，不证明子进程已退出。

前台 `serve` 准备路径和默认图、验证活动 IsOS 程序、启动 EI 与 EOS 所有者连接任务，并绑定独立的回环 UI 与 API 监听器及 manager IPC。所有者进程启动不会阻塞 IsOS 监听器或工作台就绪。UI 包缺失或不兼容时，API 与 manager 仍可使用。工作台报告就绪前，必须确认可用的 UI 监听器以及 API 和 manager 监听器。IsOS 首先用有界的健康及运行身份响应探测每个所有者并公布其概况；只有相应操作需要时才请求目录或其他内部配置。存活的所有者套接字按精确运行身份重新附着；身份冲突时仍不可执行 Run。随后服务持续监视线程与归属方服务，直至中断或失败；Stop 由 manager 和归属方证明协调。参见[manager IPC](../interfaces/manager_cn.md#manager_protocol)、[HTTP 路由](../interfaces/api/http_cn.md)和[依赖停止](../interfaces/core_cn.md#core_dependent_stop)契约。

图库是可编辑图字节的唯一存储。`config/deployments/` 下的部署绑定仅包含图库名称；`Paths::node_graph_path` 为运行时和生成修订检查解析该绑定。路径准备时在迁移锁下迁移旧部署图：字节完全相同的副本绑定现有演示图；字节不同的图先导入唯一图库文件，再移除旧文件。Apply 在图锁下读取命名图库文件、检查精确修订、验证并发布生成配置，然后更新绑定。保存已绑定图库文件会改变其保存修订；就绪状态报告 `pending_apply`，Run 在 Apply 前继续受限。格式错误的绑定或缺失文件不会静默选择其他图。客户端请求和错误见[图 API](../interfaces/api/http_cn.md#http_graph)。


修改时同步更新归属契约及中文镜像，测试成功与失败路径，并将完整输出保存在被忽略的 `data/test/<run-id>/`。运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings` 以及相关的单元与集成测试。

2026-11-15 的持久归属方回放使用隔离主目录及 API 端口运行 `python3 data/test/20261115-persistent-owner-reattach/reattach_replay.py`。回放启动 `serve --no-ui`，确认只有一个 EOS 与一个 EI 归属方且启动日志进入 IsOS 缓存，然后停止并重新启动 IsOS。归属方 PID 保持不变，重新启动的 IsOS API 返回保留的 EOS 日志。脚本与服务输出位于被忽略的 `data/test/20261115-persistent-owner-reattach/`。除另行修改的仓库演示图夹具外，IsOS 测试、全部 37 项 EOS 归属方测试、严格 IsOS/EOS Clippy、格式检查及双语文档检查均通过。

2026-11-10 的图库唯一来源改动运行了 `cargo fmt --all -- --check`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" CARGO_TARGET_DIR="$PWD/target" cargo test -p isos --no-fail-fast`、使用相同环境的 `cargo clippy -p isos --all-targets -- -D warnings -A dead_code`，以及 `python3 scripts/check_doc_links.py --all`。测试覆盖旧图字节相同与不同、并发迁移、格式错误的绑定、缺失图库文件、过期 Apply 修订，以及待 Apply 的就绪状态。隔离的临时主目录 CLI 回放运行 `isos config init`，随后在选定 EOS/EI 程序及默认配置的环境下运行 `isos config apply <home>/config/graph-pool/demo.nodes.json --revision <精确 SHA-256>`；Apply 成功，只有演示图库文件存在，过期修订被拒绝。完整输出位于被忽略的 `data/test/graph-pool-source-20261110/`。

2026-11-10 的源码 beta 回放发现无界面 `serve` 边界测试存在竞态：测试在首次收到成功的 API 回复后就发送 SIGTERM，有时工作台尚未记录就绪，因而把正常关闭报告为启动失败。测试现在等待 `workbench_ready` 后才发送 SIGTERM，同时仍断言 UI 包不可用时 HTTP 版本回复和干净退出。构建所选 EOS 程序后，`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace --all-targets -- --test-threads=1` 通过；`cargo clippy --workspace --all-targets -- -D warnings -A dead_code` 通过。不带死代码例外的严格 Clippy 仍因 18 项未使用代码诊断失败，属于待解决的发布资格缺口。完整输出位于被忽略的 `data/test/20261110-v021-source-beta/`。

2026-11-09 的节点图编辑回放通过 `isos web` 启动脱离终端的控制面，确认 `isos status --json` 报告 `stopped` 且无部署工作进程，拖动 EI 监控治理节点，并观察到浏览器重新加载前后 `GET /api/isos/v1/graph` 均返回 200。此前前台 `isos run` 进程消失，日志中没有 Stop 或归属方故障事件；其终端会话结束导致 UI 服务独立于拖动操作而退出。可复现检查为 `cargo fmt --all -- --check`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos`、`python3 scripts/check_doc_links.py --all` 和 `python3 crates/isos/tools/check_isos_docs.py`，均已通过。完整输出位于被忽略的 `data/test/20261109-graph-edit-stability/`。

后续点击回放发现另一项浏览器故障：打开 EI 监控治理节点的“配置”卡片时，缺少 `data_type` 的设置描述符使 React 卸载节点图。EI 归属方现在为 `forward_rate_hz` 输出 `data_type: "number"`。Web 编辑器可容忍不完整描述符；节点图渲染错误边界保留工作台外壳和浏览器日志面板。IsOS HTTP 日志无法诊断 React 渲染异常；浏览器错误日志记录堆栈和组件路径。回放打开“配置”→“高级设置”，确认显示数值型 `forward_rate_hz` 控件，浏览器无错误且部署保持停止。EI 的聚焦及完整测试、Ruff、Pyright、UI 类型检查、测试、浏览器及终端构建均通过；输出位于被忽略的 `data/test/20261109-node-settings-crash/`。

节点图库回放发现，已保存的 `demo` 文件已与十六节点默认图相同，但 Web 图库载入器丢弃了归属方选定目录。全局目录只有 10 个 IsOS/EOS 类型，使载入的图库草稿中六个 EI 节点没有定义。载入器现在合并经修订检查的选定目录与全局目录、恢复浏览器本地位置，并把内容相同的图库图视为已保存。不同的图库草稿若缺少归属方类型，则保持只读。浏览器回放载入 `demo` 后，全部 16 个定义均可用，没有类型不可用或端口缺失卡片、没有浏览器错误，部署保持停止。UI 类型检查、测试、本地及托管浏览器与终端构建和双语链接审计均通过；输出位于被忽略的 `data/test/20261109-pool-demo-load/`。

2026-11-12 的相机编辑器回放在 Web 图中选中了已保存但未完成的 MuJoCo 相机。点击节点后，配置检查器打开；选项加载期间显示加载文字，尚未选定来源不再触发目录不可用提示。先选 `native` 服务而不选适配器族，也会显示 `cam-0`；选择该设备后，适配器、坐标系、健康时序和观测配置复制到未保存草稿。私有网络预检仅向明确获准的本地和托管 UI 来源授权。Rust 格式检查、严格 IsOS Clippy、UI 类型检查、218 项测试、浏览器与终端构建及双语链接审计均通过。此次回放没有保存或运行图。证据位于被忽略的 `data/test/20261112-camera-editor-config/`。

节点卡片操作清理移除了图卡片上的“信息”和“配置”按钮。选择节点会打开常驻的配置检查器；焦点在卡片上时按 Enter 或空格也可选中，检查器向键盘和触屏用户提供类型、端口及数据契约详情。触发等运行时操作仍留在其所属节点上。已移除的“配置”浮层不再有入口。

2026-11-15 的演示图迁移使仓库中的 `config/graph-pool/demo.nodes.json` 与当前 17 节点 `/graph/new` 草稿一致，包括 EI 上下文输入。软件包相机草稿移至 `examples/mixed-mujoco.nodes.json`，因此新编辑器会话可打开无软件包的可编辑演示图。隔离主目录回放执行了 `config init`、精确 SHA-256 校验与 Apply，然后启动 IsOS API：`/graph` 返回 `read_only:false` 且没有诊断，选定目录在相同修订下包含全部 17 个节点类型，`/graph/validate` 返回有效且没有警告。Run 达到 EI `running` 和 EOS `ready`；按钮激活成功，关节位置为 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`，监控返回相同位置，部署 Stop 得到确认且没有活动运行或子进程。已运行 Rust 一致性测试、IsOS 测试、UI 类型检查／测试／构建、格式检查、严格 Clippy 与双语文档审计。复现输出保存在忽略的 `data/test/20261115-demo-migration/` 下。

## IsOS 本地服务器

[服务](service_cn.md)说明受管理服务的运行方式。HTTP 路由由[HTTP 接口](../interfaces/api/http_cn.md)定义。

本地服务器提供回环 IsOS API 并托管工作台。服务器通过 Host 和 Origin 校验本地请求，管理器则拥有节点图修订、软件包准入和运行标识。选定的服务子进程由已验证交接启动，并接受就绪及 Stop 证明监控。请求与响应格式由 [HTTP 契约](../interfaces/api/http_cn.md)定义。

## 三核心按钮运行时

[English](../../../docs/isos/core/server.md) · [节点契约](../interfaces/node/README_cn.md)

图格式 `0.2.1` 有三个平级核心。IsOS 管理保存修订、Apply、启动以及带运行栅栏的按钮入口。EI 和 EOS 通过私有 Unix 套接字提供归属方服务并监管自己的工作进程。EI 将类型化激活事件传过输入、认知和安全阶段。EOS 经接口接收事件，由演示技能映射为固定六关节增量，经调度器准入后更新模拟机械臂。同一运行中的重复事件 ID 保持幂等；Stop 阻止后续运动。

IsOS 退出后归属方服务仍可运行。IsOS 在私有运行目录持久化运行 ID 和图修订。启动时，IsOS 异步启动或重新附着两个所有者：在请求任何目录之前，先通过快速健康探测获得核心状态和可为空的已选运行身份。管理器重启后，它将此身份与保留的运行回执比较，重新连接两个服务，并分别报告工作进程状态。显式部署 Stop 向两个归属方发出停止请求，确认后才删除运行身份。所有者日志泵从保留的字节游标恢复，并填充 IsOS 的持久分核心日志池，供后来连接的 UI 读取。

若后续归属方进程不再识别该运行，保存的收据会使 IsOS 保持降级并阻止监控读取。[依赖停止契约](../interfaces/core_cn.md#core_dependent_stop)定义了带证明的 Stop 和本地紧急关闭。

部署 Status 并发检查 EI、EOS、IsOS 子进程及软件包服务，并按固定顺序报告结果。可用时，`last_error` 指明失败的检查项。新 Run 读取精确的已保存图后，立即检查可用的归属方健康身份，然后才刷新源码和验证图。验证完成后、写入运行收据或启动软件包服务之前，它再次检查所有选定归属方；首次检查时仍在连接的归属方由最终检查处理。若归属方仍持有旧运行，错误会列出其运行 ID 和修订，以便先处理该运行的 Stop 证明。Restart 保持先 Stop 再执行最终归属方检查的顺序。

管理器在每个进程中只计算一次当前可执行文件的摘要，后续身份响应复用该摘要。重复执行 CLI Status 因而无需每次重新计算整个可执行文件的摘要，同时仍标识正在运行的二进制文件。2026-11-17 的本地计时回放中，`./install_dev.sh` 在重建 UI 时耗时 6.28 秒，内容未变化时耗时 0.47 秒；命令和输出保存在忽略目录 `data/test/20261117-owner-startup-latency/`。

2026-11-17 的 Run 阶段检查执行了 `cargo test -p isos owner_deployment::tests -- --test-threads=1`（9 项通过）和 `cargo clippy -p isos --all-targets -- -D warnings`（通过）。同时对已停止的实时部署计时：`/runtime/process-status` 用时 0.032 秒，`/cores/readiness` 用时 21.97 秒，`/cores/setup-status` 用时 8.99 秒。这些目录与 setup 开销独立于管理器的部署状态路径。此检查没有对实时部署执行 Run 或 Stop。

`/system/health` 本地健康概览无需完整图接纳即可读取 EI 和 EOS 归属方身份；2026-11-17 的一次读取用时 0.002 秒，EOS 报告保留的失败运行，EI 报告空闲。Web 首页独立于图、软件包、就绪及 setup 请求显示归属方服务状态和进程状态。就绪检查并发读取选定目录及 EI/EOS setup 身份，只验证一次生成的启动输入，并在阻塞线程池执行。Setup 状态在阻塞线程池执行图验证；`validate_value` 只进行一次完整接纳检查。这些检查仍遵守源码软件包重建与精确接纳要求。

浏览器共用的[运行状态 WebSocket](../interfaces/api/http_cn.md#http_runtime--get-apiisosv1runtimestatusws)投影与两个快速 HTTP 路由相同的管理器和归属方状态。每条连接通过阻塞工作线程每 200 毫秒探测一次，仅发送变化，并在断线后停止探测。浏览器视图断线时回退到 HTTP；节点图、就绪状态、日志及命令时围栏仍使用各自的权威读取。此流只传送观察结果，不承载控制命令或 Stop 证明。

2026-11-17 的隔离回放使用 `ISOS_HOME=data/test/20261117-runtime-status-stream/isolated`、`ISOS_HTTP_PORT=16130`、`ISOSUI_PORT=16131`、`target/debug/isos config init` 和 `target/debug/isos serve`。带获准 Origin 的 Node `ws` 客户端立即收到 EOS 与 EI 均为空闲的 stopped 快照，两秒内没有重复事件。隔离的 `isos start` 随后产生 starting 与 stopped 流更新；图验证时因缺少本地文件而失败，因此此次回放不声称成功的 Run 转换。上文较早的隔离 Run/Stop 回放提供了归属方边界证据。聚焦 WebSocket 测试、Rust Clippy、UI 类型检查、239 项 UI 测试、本地与 TUI 构建以及文档检查均通过。

2026-11-17 的隔离回放使用 `ISOS_HOME=data/test/20261117-status-convergence/isolated`、`ISOS_HTTP_PORT=16120` 和 `ISOSUI_PORT=16121`，依次运行 `target/debug/isos config init`、`serve`、带精确修订的 `config apply`、`start` 和 `deployment stop`。重建后的管理器上，`/system/health` 用时 0.002 秒，`/runtime/process-status` 用时 0.045 秒，`/cores/readiness` 用时 1.226 秒（重复读取 0.954 秒），`/cores/setup-status` 用时 1.160 秒。EI 与 EOS 均报告新运行，监控返回 ready 观测，Stop 确认了两个归属方。随后关闭隔离服务和空闲归属方服务；现有部署未被修改。证据保存在已忽略的隔离目录中。

## 紧急关闭

CLI `isos stop` 命令在 IsOS 服务未运行、保存的运行 ID 不匹配时仍可工作。它向属于此工作区的进程发送 SIGTERM，必要时升级为 SIGKILL。确认没有匹配进程后，以紧急审计记录退役过期的归属方运行收据。进程成功退出并不证明执行器处于物理安全状态。命令警告此操作可能中断运行中的硬件，要求操作员核实硬件状态。

本地操作系统权限控制可发送信号的进程。`config/isos/emergency-stop.toml` 或其 `ISOS_STOP_CODE_SHA256` 环境变量覆盖值可增加口令校验，通过 `--code-stdin` 从标准输入提供；口令不会进入进程参数。在源码工作区中，`isos stop` 调用 `kill.sh` 发送进程信号；脚本还支持 dry-run 预览。[契约](../interfaces/core_cn.md#core_dependent_stop--本地紧急-stop)定义了命令范围和警告。

2026-11-17 的跨工作区归属方回放依次运行 `./kill.sh --dry-run`、`./isos stop`、再次 dry-run 和 `./isos status --json`。首次预览选中了管理器、绑定到本工作区套接字的两个归属方，以及 EOS 工作进程子进程。Stop 完成后，第二次预览未选中任何进程，状态为 `stopped`，终止结果为 `emergency_signal_unproven`。这仅验证进程退出，不证明物理安全。

## 回放证据

2026-11-14 的无界面演示回放保存在忽略提交的 `data/test/2026-11-14-headless-demo/`。在新的短路径 `ISOS_HOME` 中设置下文所列的可执行文件和 EI 默认值变量，然后运行 `isos config init`、`isos config validate`、`isos config apply --file <图库文件> --revision <精确 SHA-256>` 和 `isos run --no-ui`。API 在 `ISOS_HTTP_PORT` 应答；`ISOSUI_PORT` 没有监听器。`isos status --json` 显示 EI 运行中、EOS 和模拟硬件就绪。向 `/api/isos/v1/nodes/events` 发送请求接受了 `headless-demo-1`；`/nodes/events/result` 和 `/nodes/monitor` 均报告 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`。重复事件 ID 保持幂等；过期的运行 ID 返回 HTTP 409。两条警告记录来自这次有意的拒绝。`isos deployment stop` 确认两个归属方均已停止。回放还发现并修复了无界面 Run 错误打印 UI URL 的问题。

2026-11-14 的纯命令行回放保存在被忽略的 `data/test/20261114-integrated-demo/`。使用较短的独立 `ISOS_HOME`（满足 Unix 套接字路径长度）、`ISOS_MODE=dev`、已构建的 `target/debug` 二进制，以及 `ISOS_AETHER_BIN_DIR=$PWD/target/debug`、`ISOS_EI_BIN_DIR=$PWD/ei/bin`、`ISOS_EI_DEFAULTS_DIR=$PWD/config/ei`、`ISOS_EI_FACTORY_DEFAULTS_DIR=$PWD/config/defaults/ei` 和 `EI_PYTHON=$PWD/ei/.venv/bin/python`，依次执行 `isos config init`、`config validate`、按精确修订 `config apply`、`serve` 和 `start` 后，EI 运行、EOS 与模拟硬件就绪。向 `/api/isos/v1/nodes/events` 发起命令行 HTTP 请求成功；关联结果和 `/nodes/monitor` 均报告关节位置 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`。干净运行的 `isos logs --current --level warn --format jsonl --lines 100` 没有返回警告或错误。`isos deployment stop` 确认两个归属方均已停止。此次回放发现并修复 EI 出厂默认配置指向旧 sandbox 位置的失效链接。

2026-11-09 的回放保存在被忽略的 `data/test/20261109-arm-button-integration/`。它执行 `isos config init`、验证、Apply、Run、两次不同激活、重复重试、旧修订拒绝、统一日志以及已确认 Stop。模拟机械臂位置依次为 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]` 和 `[0.1,-0.08,0.06,-0.04,0.02,-0.02]`。重复重试保留第一次位置；旧修订返回 409。Rust 工作区测试、UI 检查与构建、格式检查、Clippy 和双语链接审计均通过。复现时使用全新的 `ISOS_HOME`、已构建的 `target/debug` 归属方二进制、`ISOS_AETHER_BIN_DIR=$PWD/target/debug`、`ISOS_EI_BIN_DIR=$PWD/ei/bin` 和 `EI_PYTHON=$PWD/ei/.venv/bin/python`。

2026-11-10 的紧急 Stop 回放保存在被忽略的 `data/test/emergency-stop-20261110/`。`cargo fmt --all -- --check`、`cargo test -p aether-runtime --bin aether-profile`、IsOS 恢复聚焦测试、`ei/.venv/bin/python -m pytest ei/tests/ei/test_owner_service.py -q`、Ruff 和 `python3 scripts/check_doc_links.py --all` 均通过。`target/debug/isos stop` 在没有管理器时清除旧归属方；重新 Apply 和 Run 后 EOS 就绪、EI 运行，`/api/isos/v1/nodes/monitor` 返回包含观察源的 HTTP 200；紧急 Stop 也清除了正在运行的模拟部署。所选测试环境中的 IsOS 广泛测试有两个目录队列／准入用例失败；对 IsOS 测试目标使用 `-D warnings` 的 Clippy 报告未使用代码警告。完整输出保存在证据目录。

## IsOS 监控

[日志](logger_cn.md)说明运行证据。API 契约见[运行时](../interfaces/api/http_cn.md#http_runtime)、[所有者](../interfaces/api/http_cn.md#http_owners)和[日志游标](../interfaces/manager_cn.md#manager_log_cursors)。

监控以选定的运行和修订为起点。IsOS 汇总所有者就绪状态与子进程状态，提供带游标的当前运行日志，并将缺失或过期的所有者证据视为不可用。Stop 仍由管理器执行，必须取得所有者证明并确认子进程退出；可见日志行本身不能证明完成。

## IsOS UI 节点

[English](../../../docs/isos/core/server.md) · [核心索引](README_cn.md)

## 按钮分组

`isos/ui/button` 分组将工作台激活转换为绑定运行实例的节点图事件。

## 触发节点

`isos.ui.button.trigger@v0.2` 是内置工作台按钮。用户激活请求仅在当前运行和节点图修订匹配时被接受；IsOS 通过声明的输出发出类型化激活事件。[节点事件契约](../interfaces/api/client_cn.md#button)定义请求和失败字段。[按钮到机械臂路径](server_cn.md)说明下游所有者。

## 监控分组

`isos/ui/monitor` 分组显示当前运行的只读返回链路观测。

## 机械臂监控节点

`isos.ui.monitor.arm@v0.2` 呈现返回链路的机械臂观测。它只读取当前运行，不发送执行器命令；运行或修订变化时清除过期观测。[节点监控契约](../interfaces/api/client_cn.md#monitor)定义响应字段。

## 展示选择

本设计解释 IsOS 只选择展示元数据、不充当数据生产者的原因。[数据接口](../interfaces/api/README_cn.md)定义输出类型和选择；[HTTP 展示路由](../interfaces/api/http_cn.md#http_presentations)定义当前响应。

注册的输出端口及其 `DataType` 标识客户端可能显示的内容。归属生产者提供的描述符还应绑定表示形式、交付方式、端点标识、运行 ID 和图修订。生产者拥有负载字节及端点授权。IsOS 不会从软件包 ID 或类型名称推断图像、深度、检测或音频语义。

当前路由拒绝畸形或过期请求；在没有注册已验证的公开归属方描述符时返回不可用。它不伪造端点。归属方描述符集成是[未完成门槛](roadmap_cn.md#目标架构尚未实现)。

## 视觉分组

`isos/ui/vision` 分组呈现选定的节点图视觉流及其来源。

## 视觉监控节点

`isos.ui.vision.monitor@v0.2` 呈现选定的节点图视觉流。IsOS 将监控节点绑定到活动节点图、运行和来源；[视觉流契约](../interfaces/api/client_cn.md#vision)定义帧准入。下文说明用户界面的面板行为。

## Vision 面板消费者

IsosUI 归属的 Vision 节点可以引用已应用图输出。这里列出它，是为了帮助读者理解 core → layer → group → node 图层级；IsOS 本身不注册或渲染此 UI 类型。所选软件包或归属方目录必须提供注册类型声明。

图引用标识源节点路径及输出端口。IsOS 有纯[数据选择器](../interfaces/api/client_cn.md#selection)，但当前[展示路由](../interfaces/api/http_cn.md#http_presentations)因没有归属方发布已验证的公开描述符而返回 503。IsosUI 拥有布局和订阅生命周期；生产者拥有负载字节与端点授权。关闭面板不会停止生产者。

[English](../../../docs/isos/core/server.md)

## 验证

本页面向检查当前 IsOS 源码树的贡献者。将完整命令输出保存在忽略的 `data/test/<run-id>/` 下；每次运行记录源码提交和工具链。

2026-11-15 的首次使用指南合并将 EI 与 EOS 的准备、诊断和硬件说明移入 `docs_cn/isos/start/quick-start_cn.md`，并移除重复的 IsOS、EI、EOS 入门页面。在仓库根目录运行 `python3 scripts/check_doc_links.py --all`、`python3 crates/isos/tools/check_isos_docs.py`、`python3 tests/fixtures/jsonl_events/check_events.py`、`python3 crates/isos/tests/fixtures/reader_features/check_docs.py`、针对已跟踪 Markdown 文件的全仓库 `check_file` 检查，以及 `git diff --check`。检查均通过：355 个文档页面与 400 个已跟踪 Markdown 文件的相对链接错误为零，64 对 IsOS 中英文页面对齐，两项夹具检查通过。完整输出位于被忽略的 `data/test/20261115-unified-first-use/`。

2026-11-11 在 `refactor/v2/owner-architecture-design` 上对公开文档发布说明的后续修改运行了 `python3 scripts/check_doc_links.py --all`（380 个 Markdown 文件、零断链）、`python3 crates/isos/tools/check_isos_docs.py`（英文和中文各 65 页、零问题）及 `git diff --check`。完整输出位于忽略的 `data/test/20261111-public-architecture-release/`。此项仅记录公开快照的文档修改门禁，不代表已发布或验证公开快照。

2026-11-11 的归属架构设计变更位于 `refactor/v2/owner-architecture-design`，运行了 `python3 scripts/check_doc_links.py --all`（380 个 Markdown 文件、零断链）、`python3 crates/isos/tools/check_isos_docs.py`（英文及中文各 65 页、零问题）、`git diff --check HEAD` 和三对架构文档的标题层级一致性检查。完整输出与审查补丁位于忽略的 `data/test/20261111-owner-architecture-design/`。本变更仅修改文档，不声称运行时回放或代码测试。

2026-11-09 的文档导航切片位于 `refactor(v2)/docs-navigation-mermaid`，运行了 `git diff --check`、`python3 scripts/check_doc_links.py --all`（376 个 Markdown 文件、零断链）及 `python3 crates/isos/tools/check_isos_docs.py`（英文和中文 IsOS 页面各 58 页、零问题）。两份节点类型 JSON 示例均解析成功。完整输出位于被忽略的 `data/test/20261109-docs-navigation-mermaid/`。

2026-11-09 在同一分支上的文档合并后续检查了全部 175 个 `json` 代码块，确认均可解析且使用易读的多行排版，并运行了 `python3 scripts/check_doc_links.py --all`（364 页、零断链）、`python3 crates/isos/tools/check_isos_docs.py`（每种语言各 57 页、零问题）及 `git diff --check`。完整输出位于被忽略的 `data/test/20261109-docs-consolidation/`。

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
XDG_CONFIG_HOME=/tmp/isos-generic-node-empty cargo test --all-targets -- --test-threads=1
python3 tools/check_isos_docs.py
python3 tests/fixtures/control_grants/check_cases.py
cargo build --workspace
python3 tests/reader_features_cli.py
git diff --check
```

2026-11-05 的通用节点运行通过格式检查、严格 Clippy、324 个单元测试及 24 个集成测试；1 个依赖归属方的测试被忽略。两个无关包家族的聚焦测试还覆盖注册声明、选择器 ID、类型化边、资源变更、不安全路径和过大元数据。控制授权和读取器功能文档夹具均通过。文档检查发现英文与中文各 55 页，目录、标题、相对链接、锚点及 JSON 示例均无问题。完整输出位于 `data/test/2026-11-05-generic-node/`。临时空白 `XDG_CONFIG_HOME` 让测试不依赖操作者的有效 IsOS 配置。洁净主目录下的归属方回放在 `data/test/20261105-generic-node-bridge/replay-r29.log` 中通过了两个模拟器场景：通用包安装、图 Validate 和 Apply、Run 就绪、经 EOS 传递的 IsOS 按键按下／续租／释放、每场景 20 帧相机图像、完成的原语命令，以及模拟器已回收的确认 Stop。该回放使用的 EOS 与包侧改动仍为本地未提交状态。

2026-11-06 的配置验证运行了 `cargo test --all-targets --no-fail-fast`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`python3 -m unittest discover -s tests -p 'test_install_user_service.py'`、`python3 tools/check_isos_docs.py` 和 `git diff --check`，全部通过。文档检查发现两种语言各 56 页、零问题。还针对选定的本地主目录运行了已安装的 `isos --help` 和 `isos version check --json`。完整 Cargo 与安装器输出位于 `data/test/2026-11-06-config/`。

配置单元测试验证选定 TOML 优先、示例补充缺失字段。

2026-11-08 的 Cargo 路径修复运行了 `python3 crates/isos/tests/test_product_installer.py`（5 项通过，包含 Cargo 仅位于 `~/.cargo/bin` 的模拟构建）、`bash -n install.sh`、`cargo fmt --all -- --check`、工作区 Clippy，以及 `python3 scripts/check_doc_links.py --all`（462 个 Markdown 文件，零断链）。完整输出位于被忽略的 `data/test/20261108-cargo-path/`。

2026-11-08 的 TOML／环境配置切片位于 `refactor(v2)/toml-env-config`，运行了 `cargo fmt --all -- --check`、`cargo test -p isos --bin isos app::config::tests`（8 项通过）、`cargo test -p isos --bin isos app::isosui::tests`（4 项通过）、`cargo test -p isos --bin isos app::cli::product::tests`（1 项通过）、`cargo test -p isos --test active_home_cli`（5 项通过），以及 `cargo clippy --workspace --all-targets -- -D warnings`（通过）。`PYTHONPATH=ei/src:ei/libexec ei/.venv/bin/python -m pytest ei/tests -q` 通过 751 项测试；EI Ruff 检查通过。双语链接检查覆盖 462 个文件，零断链；IsOS 文档检查发现两种语言各 60 页、零问题。完整输出位于被忽略的 `data/test/20261108-toml-env-config/`。


2026-11-08 的启动恢复和图库切片运行了 `./install_dev.sh`、`cargo fmt --all -- --check`、`cargo test -p isos --bin isos`（347 项通过、3 项忽略）、`cargo clippy -p isos --all-targets -- -D warnings`、`npm --prefix ui run typecheck`、`npm --prefix ui test -- --run`（211 项通过），以及 `python3 scripts/check_doc_links.py --all`（466 个 Markdown 文件、零断链）。位于 `data/test/20261108-launch-readiness-recovery/replay.py` 的隔离 HTTP 回放拒绝过时图、以保存字节修订创建空白草稿、单独保存并列出命名图库草稿、拒绝过期图库修订、按准确修订替换活动图，并在 API 诊断和统一事件日志中以 `graph_has_no_processes` 拒绝运行空白图。第二次隔离回放在 Apply 后更改复制的 EOS 程序；就绪状态、Run 和统一事件日志均报告 `generated_launch_input_mismatch`。完整输出位于 `data/test/20261108-launch-readiness-recovery/`。
2026-11-08 的服务启动器和文档合并切片位于 `refactor(v2)/server-launcher-docs`，运行了 `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo clippy -p isos --all-targets -- -D warnings` 和 `cargo test -p isos --test cli_guide`（5 项通过）。`python3 crates/isos/tools/check_isos_docs.py` 检查了英文和中文各 56 页，零问题；`python3 scripts/check_doc_links.py --all` 检查了 454 个 Markdown 文件，零相对断链。`cargo test -p isos --bin isos app::server::tests -- --test-threads=1` 通过 19 项，两个有界工作队列测试失败（`global_type_and_node_set_reads_share_bounded_workers` 和 `selected_catalog_queue_is_bounded_and_does_not_block_runtime`）；并发运行时同两项也失败。本切片没有修改这些处理器。完整输出位于被忽略的 `data/test/20261108-server-launcher-docs/`。

独立审查发现 UI 可能在等待 API／manager 就绪期间退出。共享等待现在会在报告就绪前检查 UI 是否仍在运行；修正后格式检查、严格 Clippy 和 CLI 指南测试再次通过。

2026-11-08 将本切片变基到 `master` 后，`cargo fmt --all -- --check`、`cargo clippy -p isos --all-targets -- -D warnings`、`cargo test -p isos --test cli_guide`（5 项通过）以及 `cargo build --workspace` 均通过。设置 `ISOS_AETHER_BIN_DIR="$PWD/target/debug"` 后执行 `cargo test -p isos --bin isos -- --test-threads=1`，347 项通过、3 项忽略，其中包含 22 项有效服务端测试。独立工作树首次执行服务端测试时未设置 `ISOS_AETHER_BIN_DIR`，由于本工作树缺少选定的 EOS 构建，两项目录测试失败；执行 `cargo build --workspace` 并指定本工作树构建目录后重新运行通过。IsOS 文档检查发现英文和中文各 57 页，零问题；双语链接检查覆盖 456 个 Markdown 文件，零相对断链。完整输出位于被忽略的 `data/test/20261108-server-launcher-rebase/`。

[English](../../../docs/isos/core/server.md)

## 当前设计文档对齐（2026-11-10）

在 `refactor(v2)/docs-current-design` 分支运行 `python3 scripts/check_doc_links.py --all`，检查 378 个 Markdown 文件，相对链接无错误；`python3 crates/isos/tools/check_isos_docs.py` 检查 61 个英文及 61 个中文 IsOS 页面，无问题；`git diff --check` 通过。完整输出位于忽略目录 `data/test/20261110-docs-current-design/`。解析图模板确认三个核心共 16 个节点、15 个已声明输入。本次仅修改文档，没有更改或重放运行时行为。

## 异步演示 API 与 UI（2026-11-10）

在 `refactor(v2)/async-isos-ui` 分支，`cargo fmt --all` 和 `cargo fmt --all -- --check` 通过。`cargo test -p isos --bin isos app::server::routes::nodes::tests` 的 7 项测试通过。严格的 `cargo clippy -p isos --all-targets -- -D warnings` 因本切片之外现有的 18 项未使用代码诊断而失败；添加 `-A dead_code` 后通过。在 `ui/` 目录中，`npm run typecheck`、`npm test`（189 项通过）及 `npm run build`（本地、托管和 TUI）通过。IsOS 文档检查发现每种语言 61 页，零问题；链接检查覆盖 378 个 Markdown 文件，零断链。完整输出位于被忽略的 `data/test/async-isos-ui-20261110/`。

组合后的 `refactor(v2)/async-graph-test` 重放运行了 `cargo build --workspace`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace`、EI 完整 `pytest`（745 项通过）、Ruff、Pyright、导入契约、UI 测试（189 项通过）及本地／托管／TUI 构建、格式检查和双语文档检查。严格的工作区 Clippy 遇到现有的 18 项未使用代码诊断；`cargo clippy --workspace --all-targets -- -D warnings -A dead_code` 通过。隔离的真实归属方重放验证并 Apply 初始图，确认 EI/EOS 就绪，同时接受三个激活事件并让八个监控读取者工作，读取三个成功结果，确认最终关节位置和日志，最后收到两个归属方确认的 Stop。完整命令、输出及重放脚本位于被忽略的 `data/test/async-graph-20261110/`。

后续演示数据与画布双通道切片重新检查了 `cargo fmt --all -- --check`、`cargo build --workspace`、`cargo test -p isos starter`、`npm test`（190 项通过）、`npm run build` 以及 `python3 scripts/check_doc_links.py --all`（378 页，零断链）。隔离的 `isos config init` 生成新图，`isos config validate` 与带修订围栏的 `isos config apply` 接受了 EI 10 Hz 和 UI 5 Hz 的明确设置。此前真实归属方重放覆盖了未改变的命令和 Stop 路径。聚焦输出和可复现命令位于被忽略的 `data/test/async-demo-visual-20261110/`。

## 软件包配置编辑（2026-11-12）


后续拒绝重放通过测试编辑器追加重复的 `[package]` 标题。`pkg configure` 拒绝该 TOML、以 0600 权限保留编辑草稿，且 `pkg config show` 的字节保持不变。命令及输出位于忽略目录 `data/test/20261112-config-draft/`。

移除编辑器启动前的终端提示后，PTY 冒烟测试直接打开 nano；按 Ctrl+X 退出后返回 `package config unchanged`，编辑器界面上不再叠加提示文字。

## 节点目录验证记录

### 所属 core 选择器实现证据


在仓库根目录运行 `cargo fmt --all && cargo fmt --all -- --check`、`cargo test -p isos --bin isos --offline`、`cargo test -p aether-runtime --bin aether-profile --offline`、`cargo clippy -p isos -p aether-runtime --bins --offline -- -D warnings`、`cargo build --workspace --bins --offline`、`ei/.venv/bin/python -m pytest ei/tests/ei/test_config_cli.py -q`、`cd ui && npm run typecheck && npm test -- --run && npm run build`、`python3 scripts/check_doc_links.py --all`，以及 `ei/.venv/bin/python -m unittest discover -s crates/isos/tests -p unified_owner_replay_cli.py -v`。2026-11-09 的运行全部通过：349 项 IsOS 测试、16 项 EOS profile 测试、16 项 EI setup 测试、215 项 UI 测试、双语链接零错误，以及四项真实所属 core 回放。完整输出保存在被忽略的 `data/test/20261109-owner-options-*.log`。

[English](../../../docs/isos/core/server.md)

### 隔离节点库集成验证

2026-11-15 的测试分支 `test/isolated-node-library` 按依赖顺序合入共享数据类型准入、EI 与 EOS 直接调用节点库、场景元数据类型及调用方审计。先运行 `cargo build -p aether-runtime --bin aether-profile`，再运行 `cargo fmt --all && cargo fmt --all -- --check`、`cargo clippy -p isos -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`、`cargo test -p aether-node-profile`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos -- --test-threads=1` 和 `cargo test -p aether-runtime -- --test-threads=1`。另运行 `ei/.venv/bin/python -m pytest ei/tests/ei -q`、`ei/.venv/bin/ruff check ei/src/ei ei/tests/ei/test_isolated.py ei/tests/ei/test_existing_isolated.py ei/tests/ei/test_isolated_native.py ei/tests/ei/test_stages.py`、`ei/.venv/bin/pyright -p ei/pyproject.toml`、`PYTHONPATH=ei/src:ei/libexec:ei ei/.venv/bin/lint-imports --config ei/pyproject.toml`、`python3 scripts/check_doc_links.py --all` 和 `python3 crates/isos/tools/check_isos_docs.py`。最终检查通过：686 项 EI 测试、339 项 IsOS 二进制测试及其集成测试、所有 EOS profile 与 runtime 测试、四项导入约束、严格 Clippy、格式化、571 个 Markdown 文件的相对链接检查以及 74/74 页 IsOS 中英对应检查。完整输出保存在忽略目录 `data/test/20261115-isolated-node-integration/`。初次组合测试发现目录数量断言与意外的相机路由；将端口改为仅声明并重新构建 EOS 读取器后，最终复测通过。

### 旧 IsOS 节点清理审计

2026-11-15 的隔离节点库审计没有发现可在保留当前调用者和已支持软件包契约的前提下删除的 IsOS 或共享旧路径。这是源码、安装、运行和测试调用者的审计结论，并不表示历史节点类型可用。

| 候选项 | 当前调用者或契约 | 删除前提 |
| --- | --- | --- |
| `isos.eos_*` 选择器及绑定分支 | `node_options.rs`、`edge_admission.rs`、`NodeGraphManager.tsx` 和 `node-graph-edits.ts` 仍解释这些 ID。仓库内的 `etc/eos/backends/mock-cxx.toml` 由 `tests/reader_features_cli.py` 和 `tests/r02_egia_client_replay.py` 使用；类型化图夹具和 UI 测试也使用这些 ID。 | 用归属方正确的身份替换仓库配置、回放调用者、夹具和编辑器行为，然后验证安装和运行路径。 |
| 历史软件包进程准入 | `release.rs` 接受现有准确锁定的 `0.1.x` 版本；[软件包发行契约](../interfaces/package/README_cn.md#release)保持模式 `0.1.0` 和 `0.2.0` 可读取。 | 修改软件包契约并验证现有锁定的迁移后，才能删除兼容分支。 |
| 仅供测试的 `Supervisor` 和候选技能预检 | `process/supervisor.rs` 和 `candidate_skill_preflight.rs` 仅在测试中编译，但其回归测试覆盖子进程生命周期、替换和不确定 Stop。 | 先用活动归属方监督器的测试替代这些边界覆盖，再删除测试框架。 |

可用 `rg -n 'isos\.eos_|eos_hardware|eos_process|eos_dispatcher' crates/isos/src ui/src etc/eos tests` 和 `rg -n 'candidate_skill_preflight|Supervisor|reader_features_cli|mock-cxx.toml' crates/isos/src tests docs` 复现调用者审计。运行 `cargo fmt --all -- --check`、`cargo test -p isos --bin isos historical --offline`、`cargo test -p isos --test edge_admission --offline`、`python3 scripts/check_doc_links.py --all` 和 `python3 crates/isos/tools/check_isos_docs.py`。审计通过一项历史软件包测试、八项边测试、格式检查和两项文档检查。完整输出保存在被忽略的 `data/test/20261115-isos-dead-code-audit/`。

### 严格端口契约验证记录

2026-11-15 的隔离数据类型改动验证了共享 JSON 描述符准入规则及双语契约。在仓库根目录运行 `cargo fmt --all && cargo fmt --all -- --check`、`cargo test -p isos --test edge_admission --offline`、`cargo test -p isos --bin isos data_type --offline`、`cargo clippy -p isos --test edge_admission --bin isos --offline -- -D warnings`、`python3 scripts/check_doc_links.py --all` 和 `python3 crates/isos/tools/check_isos_docs.py`。全部通过；完整输出保存在被忽略的 `data/test/20261115-isolated-data-types/`。

运行 `cargo fmt --all -- --check`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos`、`cargo test -p aether-node-profile`、`cd ui && npm run typecheck && npm test && npm run build`、`python3 scripts/check_doc_links.py --all` 和 `python3 crates/isos/tools/check_isos_docs.py`。2026-11-09 的 Rust 与 UI 测试、两种 UI 构建、格式检查和双语文档审计均通过。完整命令输出保存在被忽略的 `data/test/20261109-strict-node-ports/`。非致命 Clippy 检查通过，但现有的 IsOS 未使用代码警告阻碍了 `-D warnings` 检查。

### 相连按钮演示验证记录

首个连通按钮切片包含八个节点和七条类型化连接。当前演示图还包含下文所述的八节点观测路径。IsOS 准入带运行栅栏的按钮事件，EI 将其依次传过输入、认知和安全节点，EOS 将其转换为有界六关节增量并传给模拟机械臂。内置类型目录替换了旧的仅健康演示 ID。在仓库根目录构建 `isos` 和 `aether-runtime` 二进制，然后运行 `ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --all-targets`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets`、`cd ui && npm run typecheck && npm test && npm run build`、`python3 scripts/check_doc_links.py --all` 和 `python3 crates/isos/tools/check_isos_docs.py`。

2026-11-09 的隔离回放执行了初始化 → 验证 → Apply → Run → 就绪 → 控制 → 日志 → 已确认 Stop。第一次点击返回 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]` 弧度；重试同一事件 ID 不会再次移动；新 ID 累积增量；旧修订返回 409。完整命令输出保存在被忽略的 `data/test/20261109-arm-button-integration/`。

### 模拟机械臂监控回放记录

2026-11-12 的节点集冲突改动把保存的演示图恢复为有效的 16 节点链。Graph 编辑器现可建立空白软件包草稿，避免节点集继承演示执行节点。预览保留已有节点，为新增路径或 EOS 硬件 ID 冲突分配确定性后缀；组合执行模式不受支持时返回有界拒绝原因。验证运行了 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace --all-targets --quiet -- --test-threads=1`、UI 类型检查/测试/构建及双语文档检查。已安装的通用软件包节点集在空白草稿及已有软件包图上均成功预览，后者的新增节点获得新名称；完整输出保存在被忽略的 `data/test/20261112-generic-package-core-boundary/`。

16 节点起始图新增八个只读返回节点，并绑定控制链使用的同一模拟机械臂。组合回放运行了 `cargo fmt --all -- --check`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --all-targets`、`ei/.venv/bin/python -m pytest ei/tests -q`、`cd ui && npm run typecheck && npm test && npm run build` 以及双语链接检查。隔离环境的 `config init` → 验证 → Apply → Run → 就绪 → 按钮激活 → 监控读取 → 日志 → 已确认 Stop 回放在移动前返回六个零位，移动后返回准确的六关节位置，Stop 后监控值清空。随后直接替换现用保存图，浏览器点击后在节点和监控面板均显示相同位置。完整输出与载荷位于被忽略的 `data/test/20261109-arm-monitor-integration/`。

## 核心文档布局（2026-11-16）

IsOS 设计页面整合到 `docs/isos/core/*.md`，中文对应页面位于 `docs_cn/isos/core/`。从仓库根目录运行 `python3 scripts/check_doc_links.py --all`、`python3 crates/isos/tools/check_isos_docs.py`、`python3 crates/isos/tests/fixtures/control_grants/check_cases.py`、`python3 crates/isos/tests/fixtures/reader_features/check_docs.py` 和 `git diff --check`。检查全部通过：341 个 Markdown 文件无相对断链，57 对 IsOS 页面无问题，两项夹具检查通过，差异无空白错误。完整输出位于忽略目录 `data/test/20261116-isos-core-docs/checks.log`。

## 文档布局验证（2026-11-16）

根目录约定和发布指南、IsOS 核心服务与 UI 指南、前端接口已重组，运行时行为未改变。从仓库根目录运行 `python3 scripts/check_doc_links.py --all`、`python3 crates/isos/tools/check_isos_docs.py`、`python3 crates/isos/tests/fixtures/control_grants/check_cases.py`、`python3 crates/isos/tests/fixtures/reader_features/check_docs.py`、`python3 ui/tests/fixtures/log_cursors/check_log_cursors.py`、`python3 ui/tests/fixtures/browser_results/check.py` 和 `git diff --check`。检查全部通过：324 个 Markdown 文件无相对断链，59 对 IsOS 中英文页面无问题，四项聚焦文档夹具通过，差异无空白错误。完整输出位于忽略目录 `data/test/20261116-isos-doc-layout-round2/checks.log`。

## 文档验证，2026-11-16

接口契约修订后，从仓库根目录运行 `python3 scripts/check_doc_links.py --all`、`python3 crates/isos/tools/check_isos_docs.py`，以及 R03、R07、P06、控制授权、读取器特性和 UI 的聚焦夹具脚本，全部通过。完整输出保存在已忽略的 `data/test/20261116-interface-contract-focus/checks.log`。
