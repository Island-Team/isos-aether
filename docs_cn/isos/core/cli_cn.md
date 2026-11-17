# IsOS 命令行

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS 核心](README_cn.md)
  - [IsOS 架构](architecture_cn.md)
  - [IsOS 配置](config_cn.md)
  - [本地核心日志](logger_cn.md)
  - [IsOS 软件包](pkg_cn.md)
  - [IsOS 路线图](roadmap_cn.md)
  - [IsOS 服务端](server_cn.md)
  - [IslandOS 服务](service_cn.md)
- 下级目录：
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/isos/core/cli.md) · [首次使用](../start/quick-start_cn.md) · [服务端](server_cn.md) · [配置](config_cn.md)

`isos` 命令选择本地主目录和管理进程，然后执行图、软件包、部署和诊断操作。[首次使用指南](../start/quick-start_cn.md)给出可执行顺序；`isos --help` 和各子命令的 `--help` 是准确的选项参考。本页按操作任务组织命令，不重复定义 HTTP 或管理进程契约。

| 任务 | 命令 | 所属说明 |
| --- | --- | --- |
| 准备主目录 | `isos config init`、`isos config validate`、`isos config apply --file <pool-file> --revision <sha256>` | [配置](config_cn.md)和[图 API](../interfaces/api/http_cn.md#http_graph) |
| 打开控制平面 | `isos`、`isos web`、`isos tui`、`isos serve`、`isos serve --no-ui` | [服务启动](server_cn.md) |
| 运行部署 | `isos run`、`isos run --no-ui`、`isos start`、`isos status --json`、`isos deployment stop` | [服务与所有者生命周期](server_cn.md)和[运行时 API](../interfaces/api/http_cn.md#http_runtime) |
| 测试无软件包演示 | `isos demo activate <事件 ID>`、`isos demo result <事件 ID>`、`isos demo monitor` | [浏览器事件契约](../interfaces/api/client_cn.md) |
| 查看证据 | `isos logs --current`、`isos doctor` | [日志](logger_cn.md)和[配置](config_cn.md) |
| 管理软件包 | `isos pkg install`、`isos pkg link`、`isos pkg update`、`isos pkg upgrade` | [软件包](pkg_cn.md)和[软件包契约](../interfaces/package/README_cn.md) |
| 更新产品 | `isos update`、`isos upgrade` | [发布](../../release_cn.md) |
| 紧急关闭 | `isos stop` | [依赖停止](../interfaces/core_cn.md#core_dependent_stop) |

Apply 要求图池文件当前字节的精确修订。`serve` 打开控制平面和空闲的 EI/EOS 所有者服务，不启动部署工作进程。`run` 启动部署，`deployment stop` 停止其工作进程并保留所有者服务以供后续运行。`isos stop` 用于独立的紧急关闭路径。

`demo` 命令使用所选管理器的本地 API。Activate 和 result 从管理器取得活动运行 ID 与修订，再发送与工作台相同的受围栏请求。Activate 只提交一次且不重试；每次新激活使用新的事件 ID，不确定的结果用同一 ID 查询。API 返回非成功状态时，命令打印 JSON 诊断并以非零状态退出。这些命令仅针对无软件包起始图的 `isos/ui/button/trigger`；其他图按钮使用[本地 HTTP API](../interfaces/api/client_cn.md)。

2026-11-17 的独立命令行演示回放使用新的短路径 `ISOS_HOME`、`ISOS_MODE=dev`、`ISOS_CONFIG=$PWD/config/isos/isos.toml`、`ISOS_AETHER_BIN_DIR=$PWD/target/debug`、`ISOS_EI_BIN_DIR=$PWD/ei/bin`、`ISOS_EI_DEFAULTS_DIR=$PWD/config/ei`、`ISOS_EI_FACTORY_DEFAULTS_DIR=$PWD/config/defaults/ei` 和 `EI_PYTHON=$PWD/ei/.venv/bin/python`。依次运行 `isos config init`、`config validate <home>/config/graph-pool/demo.nodes.json`、按精确 SHA-256 执行 `config apply --revision <sha256> <图池文件>`、`serve --no-ui`、`start` 和 `status --json` 后，EI 运行、EOS 与模拟硬件就绪，且两者使用相同运行 ID。按钮激活成功，`/nodes/monitor` 返回 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`。故意使用过期运行 ID 返回 HTTP 409 `run_fence_stale`；`deployment stop` 确认两个所有者均已停止，最终命令行状态为已停止。证据保存在被忽略的 `data/test/20261117-actionable-validation-cli/`。

增加演示命令后，在被忽略的 `data/test/dcli/` 中使用第二个新主目录，重新执行初始化、验证、Apply、serve 和 Start。`isos demo monitor` 首先返回六个零；`isos demo activate cli-demo-2` 返回 `accepted`；`isos demo result cli-demo-2` 返回 `succeeded` 及 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`；下一次 monitor 读取与这些位置一致；`isos deployment stop` 确认两个所有者均已停止。`cargo build -p isos --bin isos` 和严格 IsOS Clippy 检查通过。
