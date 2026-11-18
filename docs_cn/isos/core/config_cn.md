# IsOS 配置

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS 核心](README_cn.md)
  - [IsOS 架构](architecture_cn.md)
  - [IsOS 命令行](cli_cn.md)
  - [本地核心日志](logger_cn.md)
  - [IsOS 软件包](pkg_cn.md)
  - [IsOS 路线图](roadmap_cn.md)
  - [IsOS 服务端](server_cn.md)
  - [IslandOS 服务](service_cn.md)
- 下级目录：
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/isos/core/config.md) · [核心索引](README_cn.md)

## 配置 IsOS

本页面向选择本地或生产设置的操作人员。IsOS 先读取选定的 TOML 文件，再用 [`config/isos/isos.example.toml`](../../../config/isos/isos.example.toml) 中的完整默认值补充缺失字段。TOML 的 `#` 注释只是说明，不会成为设置。默认选定文件是 [`config/isos/isos.toml`](../../../config/isos/isos.toml)；`ISOS_CONFIG` 可用绝对路径指定另一个文件。选定文件只需包含要改变的值。

根目录 `config/` 统一保存各核心的设置：IsOS 位于 `config/isos/`，IsosUI 位于 `config/isosui.toml`，EI 组件位于 `config/ei/`，编写的部署图位于 `config/deployments/`。EOS 设置来自已 Apply 图中的 EOS 节点，不另设 EOS TOML；IsOS 在 Apply 时生成已验证的 EOS 投影。源码默认设置和开发启动脚本为 EOS 程序选择 `target/debug`。

配置优先级为**进程环境变量 → 选定 TOML → 仓库中的 TOML 默认值**。持久的非秘密设置通常写在 TOML；环境变量用于临时部署或秘密。选定的 IsOS TOML 可以省略 `isos.example.toml` 已提供的字段。TOML 中的相对路径以 IsOS 源码目录为基准。安装后的启动脚本默认选择包内 TOML；`ISOS_CONFIG` 可指定另一个绝对路径文件。旧的 `~/.config/isos/active.env` 不再读取；将仍需使用的值迁移到 TOML 或进程环境变量。

临时覆盖可使用对应的 `ISOS_*` 变量：`ISOS_HTTP_PORT` 与 `ISOSUI_PORT` 对应 `[server]`；`ISOS_AETHER_BIN_DIR`、`ISOS_EI_BIN_DIR`、`ISOS_EI_DEFAULTS_DIR`、`ISOS_EI_FACTORY_DEFAULTS_DIR`、`ISOSUI_DIST_DIR` 及 `ISOSUI_REPO_PATH` 对应 `[paths]`；`ISOS_DEPLOYMENT`、`ISOS_STOP_TIMEOUT_MS` 及 `ISOS_EOS_STOP_GRACE_MS` 对应 `[deployment]`；`ISOS_LOG_DEFAULT_SOURCE`、`ISOS_LOG_LEVEL`、`ISOS_LOG_MAX_BYTES`、`ISOS_LOG_BACKUPS` 及 `ISOS_LOG_MAX_SOURCES` 对应 `[logging]`。软件包主机输入使用其声明的大写键。无效覆盖会在服务启动前失败。

`ISOS_HOME=/path/to/home` 选择便携式主目录：配置位于 `config/`，IsOS 记录位于 `data/isos/state/`，日志位于 `data/isos/log/`，管理器 socket 位于 `data/isos/run/`。EOS 日志目录和 socket 分别位于 `data/eos/state/journal/` 与 `data/eos/run/`；EI 状态位于 `data/ei/state/`。未设置 `ISOS_HOME` 时，用户模式的配置位于 `~/.config/isos`，归属方数据位于 `${XDG_DATA_HOME:-~/.local/share}/island/<owner>`，管理器运行文件位于 `${XDG_RUNTIME_DIR:-/run/user/<uid>}/island/isos`。`ISLAND_MODE=system` 改用 `/etc/island/isos`、`/var/lib/island/<owner>` 和 `/run/island/<owner>`。清理 `cache/`、`tmp/` 或过期 `run/` 前先停止归属方；备份 `state/` 和 `user/`。切换默认路径前，使用 `scripts/migrate_var_data.py` 复制旧便携式 `var/` 目录中的持久文件；旧目录保留供操作人员确认。

选择 EI 时，将 `paths.ei_bin_dir` 设为包含 `ei` 和 `ei-config` 的目录，将 `paths.ei_defaults_dir` 设为已安装的 EI 组件配置目录，并将 `paths.ei_factory_defaults_dir` 设为同一 EI 前缀下的 `share/defaults/ei`。对应的进程环境变量是 `ISOS_EI_BIN_DIR`、`ISOS_EI_DEFAULTS_DIR` 和 `ISOS_EI_FACTORY_DEFAULTS_DIR`。`EI_PYTHON` 指定两个 EI 程序使用的 Python 解释器。IsOS 在 Apply 时验证所选程序和配置字节，只向 EI 提供图中的 `ei_` 层，并从所选生成版本启动受管理的 `ei` 角色。
在源码目录中，示例配置将 `paths.ei_factory_defaults_dir` 解析到仓库根目录的 `config/defaults/ei`。若选择其他路径，Apply 会在 `defaults` 阶段以 `generated_invalid_default` 拒绝。修正路径后，重新构建 IsOS 并重启现有 manager，使其继承的环境使用修正值。
工厂默认值路径回放（2026-11-17）：修正示例路径后，`cargo test -p isos --bin isos example_factory_defaults_resolve_to_checkout_directory -- --test-threads=1` 通过。重启已停止部署的本地 manager 后，`./isos.sh config apply config/graph-pool/demo.nodes.json --revision "$(sha256sum config/graph-pool/demo.nodes.json | cut -d' ' -f1)"` 返回 `applied node graph`，`GET /api/isos/v1/graph` 未返回诊断。本次修复未编辑当前图文件。输出位于已忽略的 `data/test/20261117-generated-default-path/`。

验证方法：构建工作区程序，将 `ISOS_AETHER_BIN_DIR` 设为该构建目录，然后运行 `cargo test -p isos --all-targets --no-fail-fast`。带桩的监管器测试覆盖受管理 EI 启动及 Stop；`python3 crates/isos/tests/unified_owner_replay_cli.py` 使用相同图投影检查真实 EI 配置适配器。



如果 `isos` 报告配置错误，读取错误中的文件路径、行号或字段，修正文件或覆盖它的环境变量，然后再次运行 `isos --help` 或 `isos status --json`。TOML 格式错误会指出所选的 `ISOS_CONFIG` 文件。如果它指向不存在的文件，创建该文件或取消设置该变量。进程环境变量覆盖值也必须通过与 TOML 相同的字段验证。

从源码目录 `target/` 运行的 IsOS 程序默认以仓库根目录为便携式主目录。从仓库根目录运行已安装的 `isos` 启动脚本时，若未显式设置 `ISOS_HOME`，也会选择该仓库的 `config/` 和 `data/`；显式设置 `ISOS_CONFIG` 只更改所选 TOML 文件。在其他目录运行已安装的命令仍使用上述用户或系统布局。在仓库根目录执行 `isos doctor`、`isos serve` 和 `isos logs all --lines 200`，即可检查同一部署及 `data/` 下的文件。IsOS 的 JSONL 位于 `data/isos/log/deployments/<deployment>/isos.jsonl`；EOS 和 EI 分别写入各自的 `eos.jsonl` 和 `ei.jsonl`，IsOS 在读取时合并。参见[本地日志指南](logger_cn.md)。设置、API 与工作台启动、请求完成、失败和停止使用稳定事件代码。成功 HTTP 请求为 `debug`，失败为 `warn` 或 `error`。请求字段只含方法、匹配的路由模板、状态和耗时，不含查询字符串或请求正文。默认级别是 `debug`，每个文件上限 8 MiB，保留五个轮换备份；`[logging]` 可以覆盖这些有界默认值。受管理 EI 的日志、缓存、运行和临时目录均位于选定的 `data/ei/` 下。

[English](../../../docs/isos/core/config.md)

在源码仓库诊断时，先执行 `isos doctor`，在另一个终端启动 `isos serve`，再请求 `curl -fsS http://127.0.0.1:16000/api/isos/v1/graph`，然后查看 `isos logs all --level debug --lines 200` 和 `data/isos/log/deployments/default/isos.jsonl`。命令失败事件记录命令与有界错误代码；缺少 IsOS core 时还记录安全的校验原因。EI 自己的诊断流位于 `data/ei/log/default/ei.jsonl`，使用相同级别名称（`trace`、`debug`、`info`、`warn`、`error`）。

2026-11-08，`cargo test -p isos runtime::logs`、`cargo clippy -p isos --all-targets -- -D warnings`、`cargo fmt --all -- --check`、五项安装器测试和 EI 日志安全测试均通过。从源码仓库运行重新构建的已安装启动器，完成了 `isos doctor`、`isos serve`、图 API 请求、停止以及无效图拒绝；设置、HTTP debug、失败及停止事件均写入 `data/isos/log/default/`。替身集成回放以 EOS 和 EI 进程完成 Apply → Run → Stop。证据保存在 `data/test/20261108-root-config-e2e/` 和 `data/test/20261108-user-launch-smoke/`。

## IsOS 配置

[配置](config_cn.md)和[安装](config_cn.md)说明 Apply 前的设置流程。公开字段由[核心配置契约](../interfaces/core_cn.md#core_setup)定义。

配置是部署 Apply 之前由管理器作出的决定。配置文件选择本地数据目录、软件包来源及所有者启动输入；安装流程准备已验证资源。保存的节点图仍须经过验证和 Apply 才能启动运行。无效配置会阻止启动并给出可操作的错误；软件包或所有者失败则通过配置状态 API 可见。

## 安装和升级 IsOS

根目录 `install.sh` 构建并安装统一产品版本，包含 IsOS 管理器、EOS 程序、EI 文件、Web 和终端客户端。需要 Rust Cargo、系统 `python3.12` 可执行文件、uv、Node 22 和 npm 10。安装脚本从 `PATH` 或 `${CARGO_HOME:-$HOME/.cargo}/bin/cargo` 查找 Cargo。在完整源码目录执行：

```sh
./install.sh --prefix "$HOME/.local/isos"
```

缺少 `config/isos/launch.toml` 时默认使用产品模式。先运行 `./install_dev.sh` 构建 Rust 工作区、同步 EI 依赖、构建 UI，并选择本地开发模式，再从仓库根目录执行 `./isos <命令> [参数]`。`./isos.sh` 是等价的显式脚本。开发模式直接选用 `target/debug/isos`、`target/debug/aether-*`、`ei/bin` 和 `ui/dist/local`，无需安装版本。即使从其他工作目录调用，脚本仍选择本仓库的配置文件（除非显式设置 `ISOS_CONFIG` 或 `ISOSUI_CONFIG`）和 `data/`（除非显式设置 `ISOS_HOME`）。修改源码后先重建相关组件，再重启管理器。设置 `ISOS_MODE=product` 可运行已安装版本；`install.sh` 用于产品发布暂存。`install_dev.sh` 写入被忽略的本地启动选择和构建产物，并刷新所选的本地源码包；它不创建系统安装。
重复运行 `./install_dev.sh` 时，Cargo 使用增量构建，`uv sync` 检查 EI 环境；若 UI 源码、锁文件、构建脚本、源码提交、工具版本及 API 地址均未变化，UI 步骤复用本地 Web 与 TUI 输出。UI 输入变化时，仅重新构建本地 Web 和 TUI。`./install.sh` 在这些输入匹配时也复用 npm 依赖和已验证的 UI 输出，但仍执行 Rust 与 EI 产品构建步骤。`--no-build` 会明确暂存已有产品输出，仅应在这些输出已针对预期源码构建并检查后使用。
完成本地构建后，`./install_dev.sh` 会刷新绑定图中从源码安装的软件包版本。需要暂缓软件包重建时，使用 `./install_dev.sh --no-source-refresh`。从目录安装的发行包继续使用已验证的安装字节。
安装器缓存检查（2026-11-17）：运行 `python3 scripts/ui_build_cache.py release write` 后，`release check` 接受未变化的输入，拒绝新增的 UI 源码文件，并在恢复源码后重新接受。`dev check` 接受当前本地输出。`python3 crates/isos/tests/test_product_installer.py -q` 的七项安装器桩测试与 `bash -n install.sh install_dev.sh` 均通过。日志位于已忽略的 `data/test/20261117-frontend-dev/`。
共享字体修改后实际运行 `./install_dev.sh`，刷新 npm 依赖和本地 UI 资源用时 9.77 秒；输入未变化的第二次运行用时 0.45 秒，并报告复用本地 Web 与 TUI 输出。当前安装的 uv 会对仓库根目录不含 Python 发行包的 `pyproject.toml` 输出非致命警告；EI 仍从 `ei/pyproject.toml` 和 `ei/uv.lock` 解析。

如果 `./isos.sh web` 报告 manager IPC 不可用或 HTTP 对应另一 manager，表示 API 端口由不同的 IsOS manager 服务。先在仓库外执行 `cd /tmp && ~/.local/isos/bin/isos status --json` 检查该服务；若有运行中的部署，先停止部署，再根据状态中的 PID 执行 `kill -TERM <manager.pid>`。也可以为 `ISOS_HTTP_PORT` 和 `ISOSUI_PORT` 选择未占用端口。重新运行 `./isos.sh web`，再用 `./isos.sh status --json` 和 `./isos.sh logs all --level debug --lines 200` 检查。持久事件位于 `data/isos/log/deployments/default/isos.jsonl`。

2026-11-08，在端口 16000 上运行的旧 IsOS 空闲服务中重现了此冲突。确认没有活动部署后，停止旧服务，执行 `./install.sh --no-build`，随后 `./isos.sh web` 在 `http://127.0.0.1:16001/en/isos` 成功启动。`./isos.sh status --json` 确认新 manager，`data/isos/log/deployments/default/isos.jsonl` 包含启动及 HTTP debug 事件。两个 manager 的实测也证明 HTTP 与 IPC 来源不一致时会拒绝，而正常 `web` 命令继续成功。五项安装器测试、Rust 格式、脚本语法和中英文链接审计均通过；输出位于 `data/test/20261108-launcher-manager-alignment/`。

安装脚本构建 Rust 工作区，调用 EI 平台构建配方，并构建 Web 与终端 UI，随后将文件暂存到 `releases/`。EI wheel 及其依赖在安装时写入发布目录，运行时不构建 Python 包。根目录 CMake 仅用于开发构建，不再提供产品安装目标。脚本写入 `MANIFEST.sha256`，验证所有暂存文件，最后原子切换 `current`。操作命令为 `bin/isos`；请将其目录加入 `PATH`。已运行的服务和部署会继续使用旧程序，直到明确重启。

`isos install /absolute/source-checkout` 调用该源码目录的根安装脚本。`isos update` 仅列出可用版本，不修改安装。`isos upgrade [version]` 从配置的 GitHub 仓库下载平台包和校验值，验证后通过同一安装脚本暂存。源码安装时可用 `./install.sh --bundle-output /absolute/isos-0.2.1-linux-x86_64.tar.gz` 导出包；旁边的 `.sha256` 是发布校验值。安装本地已验证包可运行 `./install.sh --bundle /absolute/archive.tar.gz`。

安装设置位于 [`config/isos/install.toml`](../../../config/isos/install.toml)。运行时默认值和本地覆盖位于 [`config/isos/`](../../../config/isos)；UI 默认值位于 [`config/isosui.toml`](../../../config/isosui.toml)。`ISOS_CONFIG` 与 `ISOSUI_CONFIG` 可以选择绝对路径的覆盖文件。已安装启动脚本默认使用包内副本。

`config/isos/launch.toml` 中的 `[launch] mode` 为 `./isos` 选择 `dev` 或 `product`；`ISOS_MODE` 可覆盖单次命令。`isos init --mode` 将同一 TOML 设置写入选定主目录。`install.sh` 明确暂存产品版本，从 `install.toml` 读取安装前缀与发布来源，不更改源码目录的启动模式。`config/deployments/` 中生成的节点图是本地状态，已被 Git 忽略。

安装目录的优先级是 `--prefix`、`ISOS_INSTALL_ROOT`、`install.toml`。`isos update` 与 `isos upgrade` 可用 `ISOS_INSTALL_CONFIG` 选择另一个 TOML 文件；`ISOS_RELEASE_REPOSITORY` 和 `ISOS_RELEASE_CHANNEL` 覆盖其中的发布来源和渠道。持久的发布偏好应保存在 TOML。

在 `config/isosui.toml` 中，`ui.default_interface` 为直接执行 `isos` 选择 `web` 或 `tui`，`ui.locale` 选择 `en` 或 `cn`。`connection.startup_timeout_ms` 限制 CLI 等待自动启动服务器的时间（1000–60000 毫秒）；`connection.poll_interval_ms` 控制 TUI 状态刷新（500–10000 毫秒）。IsOS 验证这些值，并通过 `/api/isos/v1/system/ui-config` 提供非秘密 UI 设置。本地 Web 服务通过 `/isosui-runtime.json` 提供当前 API 地址，使安装后的 Web 构建使用配置的 `ISOS_HTTP_PORT`。

验证：`python3 crates/isos/tests/test_product_installer.py` 使用桩程序复现源码安装与安装包导入，并检查损坏的安装包不会切换 `current`。用 `bash -n install.sh` 检查脚本语法，安装后运行 `isos doctor` 检查版本、启动模式、接口支持和本地资源。

2026-11-08，通过 `bash -n isos.sh install.sh`、`cargo fmt --all -- --check`、`cargo test -p isos legacy_json_log`、`cargo build --workspace`、`cargo clippy -p isos --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all` 检查开发启动脚本与日志迁移。本地 `./isos web`、`./isos status --json` 和 `./isos doctor` 复测选择了 `target/debug/isos`，API 正常服务，管理器使用本地构建。

手工重建复测在管理器运行期间替换了 `target/debug/isos`。`./isos web` 拒绝旧管理器，`./isos stop` 成功停止它，随后 `./isos web` 启动新的本地构建。错误输出和停止结果保存在同一测试目录。

TOML 模式后续修改通过脚本语法、Rust 格式与 Clippy、`cli_guide` 测试、桩产品安装器测试和完整的中英文链接审计。

仓库发布流程见[共享发布指南](../../release_cn.md)。

2026-11-13 的 Stop 输出修复使源码启动器等待标准错误输出转发完成后再返回 shell。`bash -n isos.sh`、`python3 -m unittest crates.isos.tests.test_launcher_stop -v` 和双语链接审计均通过。启动器夹具覆盖开发及产品模式、警告顺序，以及成功和失败退出码，不会向正在运行的进程发送信号。完整输出位于忽略的 `data/test/20261113-stop-output/`。

## 可变数据目录

[English](../../../docs/isos/core/config.md) · [IsOS 设计](README_cn.md)

IsOS、EOS 和 EI 各自拥有独立的可变数据目录。源码和便携安装使用 `data/<owner>/`，其中 `<owner>` 为 `isos`、`eos` 或 `ei`。系统安装使用 `/var/lib/island/<owner>/`，并将开机期间有效的套接字和锁放在 `/run/island/<owner>/`。`etc/` 下的配置和 `share/` 下的发行资源不属于可变数据。

| 目录 | 内容 | 运维操作 |
| --- | --- | --- |
| `state/` | 官方持久记录和归属方记忆 | 备份；不要当作缓存清理。 |
| `user/` | 官方记录之外的用户提供或创建的文件 | 备份；仅由用户决定删除。 |
| `attachments/` | 附属于记录或运行的文件 | 与关联记录一起保存。 |
| `cache/` | 可重新生成的下载和派生数据 | 仅在归属方停止后清理。 |
| `log/` | 诊断日志和结构化事件 | 按日志保留策略处理。 |
| `tmp/` | 短期工作文件 | 归属方停止后可清理。 |
| `run/` | 实时套接字、锁和进程文件 | 归属方启动时清理过期条目。 |

仅在需要时创建目录。名为 `cache` 的目录必须可重新生成；保存的图、归属方记忆和用户文件不能放在其中。仓库测试证据保存在被忽略的 `data/test/<run-id>/`；开发工具缓存保存在 `data/cache/tools/`。原先根目录 `logs/test/` 的证据和 `ei/data/ei/` 的运行文件已迁入该目录树。

## 升级源码或便携安装

先停止所有归属方并备份安装目录。在安装根目录运行 `python3 scripts/migrate_var_data.py .`。该工具处理旧 `var/lib/<owner>/` 文件和旧 `ISOS_HOME/data/` 布局。它检查符号链接和特殊文件，仅当现有目标内容一致时才接受它，先复制到临时目标，验证 SHA-256 摘要，再公布副本。旧文件保持原样。中断的复制可能留下 `.migration-pending` 目录；检查并删除该目录后重试。现有缓存、套接字和临时文件不复制；旧 `var/log/` 文件仍保留在旧目录。启动各归属方，确认保存的记录和用户文件可用；确认新安装后再删除旧文件。

如果 IsOS 以前使用 XDG 默认路径而非 `ISOS_HOME`，以实际 XDG 数据根目录运行 `python3 scripts/migrate_var_data.py --xdg "$XDG_DATA_HOME"`；通常默认值是 `~/.local/share`。该命令将旧 `<XDG 数据>/isos/` 记录复制到 `<XDG 数据>/island/isos/`，并将托管 EI 状态复制到 `<XDG 数据>/island/ei/`。

系统部署也应遵循停止、备份、验证和保留旧数据的顺序。确认已安装服务使用标准 `/var/lib/island/<owner>/` 和 `/var/log/island/<owner>/` 根目录后，以有权写入这些目录的身份运行 `python3 scripts/migrate_var_data.py --system /`。该工具把旧归属方文件复制到相应角色目录，把旧归属方日志复制到 `log/`；不会修改 `/var/cache/island` 或 `/run/island`。重试时接受内容一致的副本，遇到内容不同的目标则停止。确认新路径可用后再清理旧文件。使用自定义根目录的安装需按相同映射人工审核复制。
