# 验证

本页面向检查当前 IsOS 源码树的贡献者。将完整命令输出保存在忽略的 `data/test/<run-id>/` 下；每次运行记录源码提交和工具链。

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

2026-11-05 的通用节点运行通过格式检查、严格 Clippy、324 个单元测试及 24 个集成测试；1 个依赖归属方的测试被忽略。两个无关包家族的聚焦测试还覆盖注册声明、选择器 ID、类型化边、资源变更、不安全路径和过大元数据。控制授权和读取器功能文档夹具均通过。文档检查发现英文与中文各 55 页，目录、标题、相对链接、锚点及 JSON 示例均无问题。完整输出位于 `data/test/2026-11-05-generic-node/`。临时空白 `XDG_CONFIG_HOME` 让测试不依赖操作者的有效 IsOS 配置。洁净主目录下的归属方回放在 `data/test/20261105-generic-node-bridge/replay-r29.log` 中通过了两个模拟器场景：通用包安装、图 Validate 和 Apply、Run 就绪、经 EOS 传递的 IsOS 按键按下／续租／释放、每场景 20 帧相机图像、完成的原语命令，以及模拟器已回收的确认 Stop。该回放使用的 EOS 与包侧改动仍为本地未提交状态；其余负例和浏览器场景见[待办](remaining-work_cn.md)。

2026-11-06 的配置验证运行了 `cargo test --all-targets --no-fail-fast`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`python3 -m unittest discover -s tests -p 'test_install_user_service.py'`、`python3 tools/check_isos_docs.py` 和 `git diff --check`，全部通过。文档检查发现两种语言各 56 页、零问题。还针对选定的本地主目录运行了已安装的 `isos --help` 和 `isos version check --json`。完整 Cargo 与安装器输出位于 `data/test/2026-11-06-config/`。

配置单元测试验证选定 TOML 优先、示例补充缺失字段。

2026-11-08 的 Cargo 路径修复运行了 `python3 crates/isos/tests/test_product_installer.py`（5 项通过，包含 Cargo 仅位于 `~/.cargo/bin` 的模拟构建）、`bash -n install.sh`、`cargo fmt --all -- --check`、工作区 Clippy，以及 `python3 scripts/check_doc_links.py --all`（462 个 Markdown 文件，零断链）。完整输出位于被忽略的 `data/test/20261108-cargo-path/`。

2026-11-08 的 TOML／环境配置切片位于 `refactor(v2)/toml-env-config`，运行了 `cargo fmt --all -- --check`、`cargo test -p isos --bin isos app::config::tests`（8 项通过）、`cargo test -p isos --bin isos app::isosui::tests`（4 项通过）、`cargo test -p isos --bin isos app::cli::product::tests`（1 项通过）、`cargo test -p isos --test active_home_cli`（5 项通过），以及 `cargo clippy --workspace --all-targets -- -D warnings`（通过）。`PYTHONPATH=ei/src:ei/libexec ei/.venv/bin/python -m pytest ei/tests -q` 通过 751 项测试；EI Ruff 检查通过。双语链接检查覆盖 462 个文件，零断链；IsOS 文档检查发现两种语言各 60 页、零问题。完整输出位于被忽略的 `data/test/20261108-toml-env-config/`。

2026-11-08 的 EI 源码默认值修复仅允许工厂默认值符号链接指向仓库的 `config/ei` 目录。此前的 `ei/etc/ei` 许可路径与实际源码树不符，导致归属方启动前验证失败。运行 `cargo test -p isos source_factory_links_must_resolve_inside_ei_configuration`，再运行 `cargo build -p isos` 和 `python3 crates/isos/tests/unified_mujoco_supervisor_replay.py OUT RELEASE BUNDLE MUJOCO_ROOT pick-place`，使用已发布的软件包及 MuJoCo 3.16.0。真实回放通过包验证、图 Validate 和 Apply、EOS／EI 就绪、相机采集、类型化移动命令完成、Restart、模拟器子进程退出后的确认 Stop，以及强制模拟器故障后拒绝 Restart。首次拒绝和成功回放的完整输出位于 `data/test/20261108-mujoco-owner-replay/`。

2026-11-08 的启动恢复和图库切片运行了 `./install_dev.sh`、`cargo fmt --all -- --check`、`cargo test -p isos --bin isos`（347 项通过、3 项忽略）、`cargo clippy -p isos --all-targets -- -D warnings`、`npm --prefix ui run typecheck`、`npm --prefix ui test -- --run`（211 项通过），以及 `python3 scripts/check_doc_links.py --all`（466 个 Markdown 文件、零断链）。位于 `data/test/20261108-launch-readiness-recovery/replay.py` 的隔离 HTTP 回放拒绝过时图、以保存字节修订创建空白草稿、单独保存并列出命名图库草稿、拒绝过期图库修订、按准确修订替换活动图，并在 API 诊断和统一事件日志中以 `graph_has_no_processes` 拒绝运行空白图。第二次隔离回放在 Apply 后更改复制的 EOS 程序；就绪状态、Run 和统一事件日志均报告 `generated_launch_input_mismatch`。完整输出位于 `data/test/20261108-launch-readiness-recovery/`。
2026-11-08 的服务启动器和文档合并切片位于 `refactor(v2)/server-launcher-docs`，运行了 `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo clippy -p isos --all-targets -- -D warnings` 和 `cargo test -p isos --test cli_guide`（5 项通过）。`python3 crates/isos/tools/check_isos_docs.py` 检查了英文和中文各 56 页，零问题；`python3 scripts/check_doc_links.py --all` 检查了 454 个 Markdown 文件，零相对断链。`cargo test -p isos --bin isos app::server::tests -- --test-threads=1` 通过 19 项，两个有界工作队列测试失败（`global_type_and_node_set_reads_share_bounded_workers` 和 `selected_catalog_queue_is_bounded_and_does_not_block_runtime`）；并发运行时同两项也失败。本切片没有修改这些处理器。完整输出位于被忽略的 `data/test/20261108-server-launcher-docs/`。

独立审查发现 UI 可能在等待 API／manager 就绪期间退出。共享等待现在会在报告就绪前检查 UI 是否仍在运行；修正后格式检查、严格 Clippy 和 CLI 指南测试再次通过。

2026-11-08 将本切片变基到 `master` 后，`cargo fmt --all -- --check`、`cargo clippy -p isos --all-targets -- -D warnings`、`cargo test -p isos --test cli_guide`（5 项通过）以及 `cargo build --workspace` 均通过。设置 `ISOS_AETHER_BIN_DIR="$PWD/target/debug"` 后执行 `cargo test -p isos --bin isos -- --test-threads=1`，347 项通过、3 项忽略，其中包含 22 项有效服务端测试。独立工作树首次执行服务端测试时未设置 `ISOS_AETHER_BIN_DIR`，由于本工作树缺少选定的 EOS 构建，两项目录测试失败；执行 `cargo build --workspace` 并指定本工作树构建目录后重新运行通过。IsOS 文档检查发现英文和中文各 57 页，零问题；双语链接检查覆盖 456 个 Markdown 文件，零相对断链。完整输出位于被忽略的 `data/test/20261108-server-launcher-rebase/`。

[English](../../../docs/isos/design/verification.md)

## 当前设计文档对齐（2026-11-10）

在 `refactor(v2)/docs-current-design` 分支运行 `python3 scripts/check_doc_links.py --all`，检查 378 个 Markdown 文件，相对链接无错误；`python3 crates/isos/tools/check_isos_docs.py` 检查 61 个英文及 61 个中文 IsOS 页面，无问题；`git diff --check` 通过。完整输出位于忽略目录 `data/test/20261110-docs-current-design/`。解析图模板确认三个核心共 16 个节点、15 个已声明输入。本次仅修改文档，没有更改或重放运行时行为。

## 异步演示 API 与 UI（2026-11-10）

在 `refactor(v2)/async-isos-ui` 分支，`cargo fmt --all` 和 `cargo fmt --all -- --check` 通过。`cargo test -p isos --bin isos app::server::routes::nodes::tests` 的 7 项测试通过。严格的 `cargo clippy -p isos --all-targets -- -D warnings` 因本切片之外现有的 18 项未使用代码诊断而失败；添加 `-A dead_code` 后通过。在 `ui/` 目录中，`npm run typecheck`、`npm test`（189 项通过）及 `npm run build`（本地、托管和 TUI）通过。IsOS 文档检查发现每种语言 61 页，零问题；链接检查覆盖 378 个 Markdown 文件，零断链。完整输出位于被忽略的 `data/test/async-isos-ui-20261110/`。

组合后的 `refactor(v2)/async-graph-test` 重放运行了 `cargo build --workspace`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace`、EI 完整 `pytest`（745 项通过）、Ruff、Pyright、导入契约、UI 测试（189 项通过）及本地／托管／TUI 构建、格式检查和双语文档检查。严格的工作区 Clippy 遇到现有的 18 项未使用代码诊断；`cargo clippy --workspace --all-targets -- -D warnings -A dead_code` 通过。隔离的真实归属方重放验证并 Apply 初始图，确认 EI/EOS 就绪，同时接受三个激活事件并让八个监控读取者工作，读取三个成功结果，确认最终关节位置和日志，最后收到两个归属方确认的 Stop。完整命令、输出及重放脚本位于被忽略的 `data/test/async-graph-20261110/`。

后续演示数据与画布双通道切片重新检查了 `cargo fmt --all -- --check`、`cargo build --workspace`、`cargo test -p isos starter`、`npm test`（190 项通过）、`npm run build` 以及 `python3 scripts/check_doc_links.py --all`（378 页，零断链）。隔离的 `isos config init` 生成新图，`isos config validate` 与带修订围栏的 `isos config apply` 接受了 EI 10 Hz 和 UI 5 Hz 的明确设置。此前真实归属方重放覆盖了未改变的命令和 Stop 路径。聚焦输出和可复现命令位于被忽略的 `data/test/async-demo-visual-20261110/`。
