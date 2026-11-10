# 服务端与启动器设计

本页面向贡献者说明当前源码边界。[HTTP 契约](../../interfaces/http/README_cn.md)和[归属方契约](../../interfaces/manager/README_cn.md)定义公开字段；[验证](../verification_cn.md)记录检查结果。

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

当前 Start/Restart 包装器包含保存修订、有序进程规格、已解析输入和归属模式。监管器创建并绑定运行 ID；就绪与停止仍归 `runtime` 负责。下文分别说明软件包准入和输出选择；完整统一的准入计划仍是[待办](../remaining-work_cn.md)。

根目录 `isos.sh` 在进入 Rust CLI 前选择 `dev` 或 `product`。`dev` 使用本仓库的调试版程序、EOS 程序、EI 程序、UI 构建、配置和便携式主目录；`ISOS_HOME` 更改主目录，显式配置环境变量更改所选文件。`product` 调用配置安装目录中的已安装启动器，并在没有显式覆盖时选择源码仓库的配置与主目录。直接运行已安装启动器时，若未选择主目录，还会根据当前工作目录识别源码仓库。操作命令见[安装](installer_cn.md)。

CLI 入口先加载运行配置与路径，再分派命令。直接执行 `isos`、`web`、`tui` 会确保设置完成，并复用或自动启动服务。仅当没有 manager IPC 时，`run` 才在前台启动服务及部署；否则要求现有 manager 启动部署。`serve` 启动控制面和空闲的归属方目录服务，不启动 EI 或 EOS 部署工作进程。`start` 要求 manager 已运行。

复用服务前，CLI 检查回环 HTTP 健康状态、服务标识和 API 模式，再通过所选的仅所有者可访问 Unix 套接字取得 manager 身份并比较 API 来源。开发模式还比较可执行文件路径和 SHA-256，防止重新构建的本地程序静默复用旧 manager。没有健康服务时，CLI 拒绝仍活动的孤立 manager 套接字，以当前程序启动脱离终端的 `serve` 子进程并保存其输出，在配置的启动超时内等待 HTTP 健康状态及 manager IPC。启动超时只说明未观察到就绪，不证明子进程已退出。

前台 `serve` 准备路径和默认图、验证活动 IsOS 程序、启动空闲归属方服务，并绑定独立的回环 UI 与 API 监听器及 manager IPC。UI 包缺失或不兼容时，API 与 manager 仍可使用。工作台报告就绪前，必须确认可用的 UI 监听器以及 API 和 manager 监听器。随后服务持续监视线程与归属方服务，直至中断或失败；Stop 由 manager 和归属方证明协调。参见[manager IPC](../../interfaces/manager/protocol_cn.md)、[HTTP 路由](../../interfaces/http/README_cn.md)和[依赖停止](../../interfaces/core/dependent-stop_cn.md)契约。

图库是可编辑图字节的唯一存储。`config/deployments/` 下的部署绑定仅包含图库名称；`Paths::node_graph_path` 为运行时和生成修订检查解析该绑定。路径准备时在迁移锁下迁移旧部署图：字节完全相同的副本绑定现有演示图；字节不同的图先导入唯一图库文件，再移除旧文件。Apply 在图锁下读取命名图库文件、检查精确修订、验证并发布生成配置，然后更新绑定。保存已绑定图库文件会改变其保存修订；就绪状态报告 `pending_apply`，Run 在 Apply 前继续受限。格式错误的绑定或缺失文件不会静默选择其他图。客户端请求和错误见[图 API](../../interfaces/http/graph_cn.md)。


修改时同步更新归属契约及中文镜像，测试成功与失败路径，并将完整输出保存在被忽略的 `data/test/<run-id>/`。运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings` 以及相关的单元与集成测试。

2026-11-10 的图库唯一来源改动运行了 `cargo fmt --all -- --check`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" CARGO_TARGET_DIR="$PWD/target" cargo test -p isos --no-fail-fast`、使用相同环境的 `cargo clippy -p isos --all-targets -- -D warnings -A dead_code`，以及 `python3 scripts/check_doc_links.py --all`。测试覆盖旧图字节相同与不同、并发迁移、格式错误的绑定、缺失图库文件、过期 Apply 修订，以及待 Apply 的就绪状态。隔离的临时主目录 CLI 回放运行 `isos config init`，随后在选定 EOS/EI 程序及默认配置的环境下运行 `isos config apply <home>/config/graph-pool/demo.nodes.json --revision <精确 SHA-256>`；Apply 成功，只有演示图库文件存在，过期修订被拒绝。完整输出位于被忽略的 `data/test/graph-pool-source-20261110/`。

2026-11-10 的源码 beta 回放发现无界面 `serve` 边界测试存在竞态：测试在首次收到成功的 API 回复后就发送 SIGTERM，有时工作台尚未记录就绪，因而把正常关闭报告为启动失败。测试现在等待 `workbench_ready` 后才发送 SIGTERM，同时仍断言 UI 包不可用时 HTTP 版本回复和干净退出。构建所选 EOS 程序后，`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace --all-targets -- --test-threads=1` 通过；`cargo clippy --workspace --all-targets -- -D warnings -A dead_code` 通过。不带死代码例外的严格 Clippy 仍因 18 项未使用代码诊断失败，属于待解决的发布资格缺口。完整输出位于被忽略的 `data/test/20261110-v021-source-beta/`。

2026-11-09 的节点图编辑回放通过 `isos web` 启动脱离终端的控制面，确认 `isos status --json` 报告 `stopped` 且无部署工作进程，拖动 EI 监控治理节点，并观察到浏览器重新加载前后 `GET /api/isos/v1/graph` 均返回 200。此前前台 `isos run` 进程消失，日志中没有 Stop 或归属方故障事件；其终端会话结束导致 UI 服务独立于拖动操作而退出。可复现检查为 `cargo fmt --all -- --check`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos`、`python3 scripts/check_doc_links.py --all` 和 `python3 crates/isos/tools/check_isos_docs.py`，均已通过。完整输出位于被忽略的 `data/test/20261109-graph-edit-stability/`。

后续点击回放发现另一项浏览器故障：打开 EI 监控治理节点的“配置”卡片时，缺少 `data_type` 的设置描述符使 React 卸载节点图。EI 归属方现在为 `forward_rate_hz` 输出 `data_type: "number"`。Web 编辑器可容忍不完整描述符；节点图渲染错误边界保留工作台外壳和浏览器日志面板。IsOS HTTP 日志无法诊断 React 渲染异常；浏览器错误日志记录堆栈和组件路径。回放打开“配置”→“高级设置”，确认显示数值型 `forward_rate_hz` 控件，浏览器无错误且部署保持停止。EI 的聚焦及完整测试、Ruff、Pyright、UI 类型检查、测试、浏览器及终端构建均通过；输出位于被忽略的 `data/test/20261109-node-settings-crash/`。

节点图库回放发现，已保存的 `demo` 文件已与十六节点默认图相同，但 Web 图库载入器丢弃了归属方选定目录。全局目录只有 10 个 IsOS/EOS 类型，使载入的图库草稿中六个 EI 节点没有定义。载入器现在合并经修订检查的选定目录与全局目录、恢复浏览器本地位置，并把内容相同的图库图视为已保存。不同的图库草稿若缺少归属方类型，则保持只读。浏览器回放载入 `demo` 后，全部 16 个定义均可用，没有类型不可用或端口缺失卡片、没有浏览器错误，部署保持停止。UI 类型检查、测试、本地及托管浏览器与终端构建和双语链接审计均通过；输出位于被忽略的 `data/test/20261109-pool-demo-load/`。
