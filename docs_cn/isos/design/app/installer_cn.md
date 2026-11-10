# 安装和升级 IsOS

根目录 `install.sh` 构建并安装统一产品版本，包含 IsOS 管理器、EOS 程序、EI 文件、Web 和终端客户端。需要 Rust Cargo、系统 `python3.12` 可执行文件、uv、Node 22 和 npm 10。安装脚本从 `PATH` 或 `${CARGO_HOME:-$HOME/.cargo}/bin/cargo` 查找 Cargo。在完整源码目录执行：

```sh
./install.sh --prefix "$HOME/.local/isos"
```

缺少 `config/isos/launch.toml` 时默认使用产品模式。先运行 `./install_dev.sh` 构建 Rust 工作区、同步 EI 依赖、构建 UI，并选择本地开发模式，再从仓库根目录执行 `./isos <命令> [参数]`。`./isos.sh` 是等价的显式脚本。开发模式直接选用 `target/debug/isos`、`target/debug/aether-*`、`ei/bin` 和 `ui/dist/local`，无需安装版本。即使从其他工作目录调用，脚本仍选择本仓库的配置文件（除非显式设置 `ISOS_CONFIG` 或 `ISOSUI_CONFIG`）和 `data/`（除非显式设置 `ISOS_HOME`）。修改源码后先重建相关组件，再重启管理器。设置 `ISOS_MODE=product` 可运行已安装版本；`install.sh` 用于产品发布暂存。`install_dev.sh` 只写入被忽略的本地启动选择和构建产物，不创建系统安装。

如果 `./isos.sh web` 报告 manager IPC 不可用或 HTTP 对应另一 manager，表示 API 端口由不同的 IsOS manager 服务。先在仓库外执行 `cd /tmp && ~/.local/isos/bin/isos status --json` 检查该服务；若有运行中的部署，先停止部署，再根据状态中的 PID 执行 `kill -TERM <manager.pid>`。也可以为 `ISOS_HTTP_PORT` 和 `ISOSUI_PORT` 选择未占用端口。重新运行 `./isos.sh web`，再用 `./isos.sh status --json` 和 `./isos.sh logs all --level debug --lines 200` 检查。持久事件位于 `data/isos/log/deployments/default/isos.jsonl`。

2026-11-08，在端口 16000 上运行的旧 IsOS 空闲服务中重现了此冲突。确认没有活动部署后，停止旧服务，执行 `./install.sh --no-build`，随后 `./isos.sh web` 在 `http://127.0.0.1:16001/en/isos` 成功启动。`./isos.sh status --json` 确认新 manager，`data/isos/log/deployments/default/isos.jsonl` 包含启动及 HTTP debug 事件。两个 manager 的实测也证明 HTTP 与 IPC 来源不一致时会拒绝，而正常 `web` 命令继续成功。五项安装器测试、Rust 格式、脚本语法和中英文链接审计均通过；输出位于 `data/test/20261108-launcher-manager-alignment/`。

安装脚本构建 Rust 工作区，调用 EI 平台构建配方，并构建 Web 与终端 UI，随后将文件暂存到 `releases/`。EI wheel 及其依赖在安装时写入发布目录，运行时不构建 Python 包。根目录 CMake 仅用于开发构建，不再提供产品安装目标。脚本写入 `MANIFEST.sha256`，验证所有暂存文件，最后原子切换 `current`。操作命令为 `bin/isos`；请将其目录加入 `PATH`。已运行的服务和部署会继续使用旧程序，直到明确重启。

`isos install /absolute/source-checkout` 调用该源码目录的根安装脚本。`isos update` 仅列出可用版本，不修改安装。`isos upgrade [version]` 从配置的 GitHub 仓库下载平台包和校验值，验证后通过同一安装脚本暂存。源码安装时可用 `./install.sh --bundle-output /absolute/isos-0.2.1-linux-x86_64.tar.gz` 导出包；旁边的 `.sha256` 是发布校验值。安装本地已验证包可运行 `./install.sh --bundle /absolute/archive.tar.gz`。

安装设置位于 [`config/isos/install.toml`](../../../../config/isos/install.toml)。运行时默认值和本地覆盖位于 [`config/isos/`](../../../../config/isos)；UI 默认值位于 [`config/isosui.toml`](../../../../config/isosui.toml)。`ISOS_CONFIG` 与 `ISOSUI_CONFIG` 可以选择绝对路径的覆盖文件。已安装启动脚本默认使用包内副本。

`config/isos/launch.toml` 中的 `[launch] mode` 为 `./isos` 选择 `dev` 或 `product`；`ISOS_MODE` 可覆盖单次命令。`isos init --mode` 将同一 TOML 设置写入选定主目录。`install.sh` 明确暂存产品版本，从 `install.toml` 读取安装前缀与发布来源，不更改源码目录的启动模式。`config/deployments/` 中生成的节点图是本地状态，已被 Git 忽略。

安装目录的优先级是 `--prefix`、`ISOS_INSTALL_ROOT`、`install.toml`。`isos update` 与 `isos upgrade` 可用 `ISOS_INSTALL_CONFIG` 选择另一个 TOML 文件；`ISOS_RELEASE_REPOSITORY` 和 `ISOS_RELEASE_CHANNEL` 覆盖其中的发布来源和渠道。持久的发布偏好应保存在 TOML。

在 `config/isosui.toml` 中，`ui.default_interface` 为直接执行 `isos` 选择 `web` 或 `tui`，`ui.locale` 选择 `en` 或 `cn`。`connection.startup_timeout_ms` 限制 CLI 等待自动启动服务器的时间（1000–60000 毫秒）；`connection.poll_interval_ms` 控制 TUI 状态刷新（500–10000 毫秒）。IsOS 验证这些值，并通过 `/api/isos/v1/system/ui-config` 提供非秘密 UI 设置。本地 Web 服务通过 `/isosui-runtime.json` 提供当前 API 地址，使安装后的 Web 构建使用配置的 `ISOS_HTTP_PORT`。

验证：`python3 crates/isos/tests/test_product_installer.py` 使用桩程序复现源码安装与安装包导入，并检查损坏的安装包不会切换 `current`。用 `bash -n install.sh` 检查脚本语法，安装后运行 `isos doctor` 检查版本、启动模式、接口支持和本地资源。

2026-11-08，通过 `bash -n isos.sh install.sh`、`cargo fmt --all -- --check`、`cargo test -p isos legacy_json_log`、`cargo build --workspace`、`cargo clippy -p isos --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all` 检查开发启动脚本与日志迁移。本地 `./isos web`、`./isos status --json` 和 `./isos doctor` 复测选择了 `target/debug/isos`，API 正常服务，管理器使用本地构建。

手工重建复测在管理器运行期间替换了 `target/debug/isos`。`./isos web` 拒绝旧管理器，`./isos stop` 成功停止它，随后 `./isos web` 启动新的本地构建。错误输出和停止结果保存在同一测试目录。

TOML 模式后续修改通过脚本语法、Rust 格式与 Clippy、`cli_guide` 测试、桩产品安装器测试和完整的中英文链接审计。

仓库发布流程见[共享发布指南](../../../island/release_cn.md)。
