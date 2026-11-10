# 配置 IsOS

本页面向选择本地或生产设置的操作人员。IsOS 先读取选定的 TOML 文件，再用 [`config/isos/isos.example.toml`](../../../../config/isos/isos.example.toml) 中的完整默认值补充缺失字段。TOML 的 `#` 注释只是说明，不会成为设置。默认选定文件是 [`config/isos/isos.toml`](../../../../config/isos/isos.toml)；`ISOS_CONFIG` 可用绝对路径指定另一个文件。选定文件只需包含要改变的值。

根目录 `config/` 统一保存各核心的设置：IsOS 位于 `config/isos/`，IsosUI 位于 `config/isosui.toml`，EI 组件位于 `config/ei/`，编写的部署图位于 `config/deployments/`。EOS 设置来自已 Apply 图中的 EOS 节点，不另设 EOS TOML；IsOS 在 Apply 时生成已验证的 EOS 投影。源码默认设置和开发启动脚本为 EOS 程序选择 `target/debug`。

配置优先级为**进程环境变量 → 选定 TOML → 仓库中的 TOML 默认值**。持久的非秘密设置通常写在 TOML；环境变量用于临时部署或秘密。选定的 IsOS TOML 可以省略 `isos.example.toml` 已提供的字段。TOML 中的相对路径以 IsOS 源码目录为基准。安装后的启动脚本默认选择包内 TOML；`ISOS_CONFIG` 可指定另一个绝对路径文件。旧的 `~/.config/isos/active.env` 不再读取；将仍需使用的值迁移到 TOML 或进程环境变量。

临时覆盖可使用对应的 `ISOS_*` 变量：`ISOS_HTTP_PORT` 与 `ISOSUI_PORT` 对应 `[server]`；`ISOS_AETHER_BIN_DIR`、`ISOS_EI_BIN_DIR`、`ISOS_EI_DEFAULTS_DIR`、`ISOS_EI_FACTORY_DEFAULTS_DIR`、`ISOSUI_DIST_DIR` 及 `ISOSUI_REPO_PATH` 对应 `[paths]`；`ISOS_DEPLOYMENT`、`ISOS_STOP_TIMEOUT_MS` 及 `ISOS_EOS_STOP_GRACE_MS` 对应 `[deployment]`；`ISOS_VISION_GATE_PATH` 对应 `[vision_evidence]`；`ISOS_LOG_DEFAULT_SOURCE`、`ISOS_LOG_LEVEL`、`ISOS_LOG_MAX_BYTES`、`ISOS_LOG_BACKUPS` 及 `ISOS_LOG_MAX_SOURCES` 对应 `[logging]`。软件包主机输入使用其声明的大写键。无效覆盖会在服务启动前失败。

`ISOS_HOME=/path/to/home` 选择便携式主目录：配置位于 `config/`，IsOS 记录位于 `data/isos/state/`，日志位于 `data/isos/log/`，管理器 socket 位于 `data/isos/run/`。EOS 日志目录和 socket 分别位于 `data/eos/state/journal/` 与 `data/eos/run/`；EI 状态位于 `data/ei/state/`。未设置 `ISOS_HOME` 时，用户模式的配置位于 `~/.config/isos`，归属方数据位于 `${XDG_DATA_HOME:-~/.local/share}/island/<owner>`，管理器运行文件位于 `${XDG_RUNTIME_DIR:-/run/user/<uid>}/island/isos`。`ISLAND_MODE=system` 改用 `/etc/island/isos`、`/var/lib/island/<owner>` 和 `/run/island/<owner>`。清理 `cache/`、`tmp/` 或过期 `run/` 前先停止归属方；备份 `state/` 和 `user/`。切换默认路径前，使用 `scripts/migrate_var_data.py` 复制旧便携式 `var/` 目录中的持久文件；旧目录保留供操作人员确认。

选择 EI 时，将 `paths.ei_bin_dir` 设为包含 `ei` 和 `ei-config` 的目录，将 `paths.ei_defaults_dir` 设为已安装的 EI 组件配置目录，并将 `paths.ei_factory_defaults_dir` 设为同一 EI 前缀下的 `share/defaults/ei`。对应的进程环境变量是 `ISOS_EI_BIN_DIR`、`ISOS_EI_DEFAULTS_DIR` 和 `ISOS_EI_FACTORY_DEFAULTS_DIR`。`EI_PYTHON` 指定两个 EI 程序使用的 Python 解释器。IsOS 在 Apply 时验证所选程序和配置字节，只向 EI 提供图中的 `ei_` 层，并从所选生成版本启动受管理的 `ei` 角色。

验证方法：构建工作区程序，将 `ISOS_AETHER_BIN_DIR` 设为该构建目录，然后运行 `cargo test -p isos --all-targets --no-fail-fast`。带桩的监管器测试覆盖受管理 EI 启动及 Stop；`python3 crates/isos/tests/unified_owner_replay_cli.py` 使用相同图投影检查真实 EI 配置适配器。



如果 `isos` 报告配置错误，读取错误中的文件路径、行号或字段，修正文件或覆盖它的环境变量，然后再次运行 `isos --help` 或 `isos status --json`。TOML 格式错误会指出所选的 `ISOS_CONFIG` 文件。如果它指向不存在的文件，创建该文件或取消设置该变量。进程环境变量覆盖值也必须通过与 TOML 相同的字段验证。

从源码目录 `target/` 运行的 IsOS 程序默认以仓库根目录为便携式主目录。从仓库根目录运行已安装的 `isos` 启动脚本时，若未显式设置 `ISOS_HOME`，也会选择该仓库的 `config/` 和 `data/`；显式设置 `ISOS_CONFIG` 只更改所选 TOML 文件。在其他目录运行已安装的命令仍使用上述用户或系统布局。在仓库根目录执行 `isos doctor`、`isos serve` 和 `isos logs all --lines 200`，即可检查同一部署及 `data/` 下的文件。IsOS 的 JSONL 位于 `data/isos/log/deployments/<deployment>/isos.jsonl`；EOS 和 EI 分别写入各自的 `eos.jsonl` 和 `ei.jsonl`，IsOS 在读取时合并。参见[本地日志指南](../runtime/logging_cn.md)。设置、API 与工作台启动、请求完成、失败和停止使用稳定事件代码。成功 HTTP 请求为 `debug`，失败为 `warn` 或 `error`。请求字段只含方法、匹配的路由模板、状态和耗时，不含查询字符串或请求正文。默认级别是 `debug`，每个文件上限 8 MiB，保留五个轮换备份；`[logging]` 可以覆盖这些有界默认值。受管理 EI 的日志、缓存、运行和临时目录均位于选定的 `data/ei/` 下。

[English](../../../../docs/isos/design/app/configuration.md)

在源码仓库诊断时，先执行 `isos doctor`，在另一个终端启动 `isos serve`，再请求 `curl -fsS http://127.0.0.1:16000/api/isos/v1/graph`，然后查看 `isos logs all --level debug --lines 200` 和 `data/isos/log/deployments/default/isos.jsonl`。命令失败事件记录命令与有界错误代码；缺少 IsOS core 时还记录安全的校验原因。EI 自己的诊断流位于 `data/ei/log/default/ei.jsonl`，使用相同级别名称（`trace`、`debug`、`info`、`warn`、`error`）。

2026-11-08，`cargo test -p isos runtime::logs`、`cargo clippy -p isos --all-targets -- -D warnings`、`cargo fmt --all -- --check`、五项安装器测试和 EI 日志安全测试均通过。从源码仓库运行重新构建的已安装启动器，完成了 `isos doctor`、`isos serve`、图 API 请求、停止以及无效图拒绝；设置、HTTP debug、失败及停止事件均写入 `data/isos/log/default/`。替身集成回放以 EOS 和 EI 进程完成 Apply → Run → Stop。证据保存在 `data/test/20261108-root-config-e2e/` 和 `data/test/20261108-user-launch-smoke/`。
