# IsOS 统一发布

[English](../../docs/island/release.md) · [共享架构](README_cn.md) · [安装与升级](../isos/design/app/installer_cn.md)

本仓库只产出一个带版本的 IsOS 产品发布。开发源码位于 `isos-p`；`isos-aether` 是预期的公开快照。取得完整资格的安装包发布包含 IsOS manager、EOS 程序及其实际调用的适配器、EI 程序及依赖，以及 Web 和终端客户端。EI 和 EOS 是该发布的组成部分，不作为独立产品分别取得发布资格。源码版本、图格式、HTTP 模式、protobuf 服务、节点类型 ID 和外部软件包身份各自独立版本化。[IsOS 软件包发行契约](../isos/interfaces/packages/release_cn.md)定义经验证的外部软件包精确锁定，不是本产品的发布流程。

## 准备候选版本

从经过审查的集成提交开始，固定其锁文件及生成绑定。记录源码提交、工具链版本、构建环境、契约版本、支持平台和预定标签。检查受跟踪的源码、测试、文档、清单及 Git 历史，排除凭据、私有端点、设备身份、校准数据、已保存路由、采集内容、运行时状态和无许可材料。公开快照不包含部署设置、原生产品技能、站点脚本和本地数据。只保留 Rust 运行时实际调用的 C++ 适配器。

打标签前，运行根目录 Rust 格式检查、严格工作区 Clippy、串行工作区测试、锁定依赖的二进制构建、适用的 CMake/CTest 适配器检查、EI Ruff、Pyright、导入规则、模式检查及完整测试、UI 类型检查／测试／构建，以及双语文档检查。检查 EI wheel，确认不包含测试和私有文件。验证浏览器 Origin 准入仍限制于配置的公开客户端，缺少 EI 配置根目录时明确失败。用精确选定的线协议和生成绑定验证真实 EOS–EI 边界：发现、准入、重复操作 ID、终止确认、超时、取消、技能发现、资源流和不确定的 Stop。对干净安装的候选版本执行 IsOS 软件包验证、Apply、Run、Restart、Stop 和 `isos doctor`。将完整输出和工具版本保存在被忽略的 `data/test/<run-id>/`，并在发布凭据中列明未运行门禁或已知失败。

干净公开源码目录的核心门禁如下：

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets -- --test-threads=1
cargo build --locked --workspace --bins
cmake -S . -B "$BUILD_DIR" -DISLAND_BUILD_TESTS=ON
cmake --build "$BUILD_DIR"
ctest --test-dir "$BUILD_DIR" --output-on-failure
python3 scripts/check_doc_links.py --all
python3 crates/isos/tools/check_isos_docs.py
```

源码所在文件系统空间不足时，将 `BUILD_DIR`、`TMPDIR` 和 `CARGO_TARGET_DIR` 设为合适的绝对路径。候选版本凭据还应包含各归属方测试命令及适用的硬件或模拟器回放；私有准备运行成功不等于已发布标签验证通过。

## v0.2.1 仅源码 beta 版本

[v0.2.1 发布说明](release-0.2.1_cn.md)为以下安装包发布流程规定一个仅源码 beta 例外。在 `isos-aether` 中发布经过审查的公开快照，标记为 GitHub 预发布版本，只提供源码归档；不得附上未经构建及验证的安装程序、二进制安装包或安装包校验和。现有 Release IsOS 工作流用于已取得资格的安装包，不能用于此次仅源码发布。源码版本 `0.2.1` 与标签 `v0.2.1` 保持一致，在发布凭据中记录准确的公开提交和已完成的检查。仅在私有 `isos-p` 中完成测试不足以证明公开快照合格。

v0.2.1 的支持边界从 v0.2.1 开始：不支持 v0.2.1 之前版本的代码、已保存图、软件包和接口，也不承诺旧版迁移路径。这不改变当前图、HTTP、软件包、节点和所有者契约各自独立的身份。v0.2.1 是可测试当前已记录行为的源码，并非第三方自定义节点 SDK。内部 Rust API 和预留的扩展声明尚不稳定。计划中的 v0.2.2 自定义节点加载与 v0.2.3 跨平台支持属于目标工作，并非本次发布已实现的功能，也不保证所有 v0.2.1 扩展无需修改即可运行。

发布前，对准确的公开快照完成源码审查以及上述适用的源码构建、测试和端到端门禁。在发布凭据中写明所有未运行的平台或边界以及已知失败。发布说明必须列出实际测试过的主机平台。GitHub 仅源码预发布版本不能在 `isos upgrade` 中呈现为可安装包；操作者应按已记录的源码检出路径从标签构建。

## 构建并发布统一产品

手动 [Release IsOS 工作流](../../.github/workflows/release-isos.yml)只接受已有的 `vMAJOR.MINOR.PATCH` 标签，并要求标签、检出的提交及远端 `master` 一致。Rust、EI 和 UI 源码版本必须与标签匹配。Linux x86_64 和 aarch64 作业分别使用[根安装脚本](../../install.sh)构建发布包，验证相邻 SHA-256 文件，将每个包安装到全新目录，再检查已安装命令。两个平台作业均通过后才开始发布；工作流拒绝覆盖已有 GitHub 发布。工作流在其运行的仓库中创建发布，不会把源码或产物从 `isos-p` 传到 `isos-aether`。在公开仓库运行工作流前，需另行准备并审查公开快照。

每个发布包包含记录产品版本、源码提交、操作系统和架构的 `RELEASE.json`，以及覆盖已安装文件和发布身份的 `MANIFEST.sha256`。在发布凭据中记录经过审查的清单与外部校验和。不得为了勾选检查表而给未取得资格的开发快照打标签或发布。

IsosUI 构建本地浏览器包、托管浏览器包和终端客户端。产品归档包含本地浏览器包与终端客户端。IsOS 在 `/en/isos` 和 `/cn/isos` 提供本地包，并通过 `/isosui-runtime.json` 提供 API 地址；两个客户端都连接到 IsOS 服务。托管包供另行管理的网站使用，不是第二个产品安装程序。UI 资源沿用 `RELEASE.json` 中的产品版本和源码提交。从 `ui/` 运行 `npm ci`、`npm test`、`npm run typecheck` 和 `npm run build` 重现检查；在 `isos doctor` 后验证已安装 Web 与 TUI 的连接。

## 安装并验证已发布包

在完整源码目录运行 `./install.sh --prefix "$HOME/.local/isos"`，构建并暂存统一产品。使用 `./install.sh --bundle-output /absolute/isos-<version>-linux-<arch>.tar.gz` 导出归档；旁边的 `.sha256` 是外部校验和。下载一对文件后，先运行 `sha256sum -c isos-<version>-linux-<arch>.tar.gz.sha256`，再运行 `./install.sh --bundle /absolute/archive.tar.gz`。安装脚本在切换 `current` 前检查内部清单和主机架构；损坏的包必须保持现有安装不变。`isos update` 只列出发布版本，`isos upgrade [version]` 验证并暂存一个版本。安装后运行 `isos doctor`，检查就绪状态和日志，再完成 Apply → Run → Stop 回放。现有进程在明确重启前继续使用旧程序。

发布说明按版本和日期、操作者可见变化、支持的接口、安装或升级步骤、已知限制及可复现验证编写。已发布标签和产物保持不可变。
