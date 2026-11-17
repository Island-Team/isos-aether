# IsOS 统一发布

<!-- doc-navigation:start -->
<details>
<summary>目录：docs_cn</summary>

浏览本目录的文档及下级目录。

- [仓库](../README.md)
- 同级文档：
  - [文档](README_cn.md)
  - [文档变更记录](changelog_cn.md)
  - [共享契约与源码资源](contract_cn.md)
  - [贡献指南](contributing_cn.md)
  - [代码与文档规范](conventions.v1.4.0_cn.md)
- 下级目录：
  - [ei/](ei/README_cn.md)
  - [eos/](eos/README_cn.md)
  - [isos/](isos/README_cn.md)
  - [node/](node/README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../docs/release.md) · [IsOS 设计](isos/core/README_cn.md) · [安装与升级](isos/core/config_cn.md)

本仓库只产出一个带版本的 IsOS 产品发布。公开开发源码位于 `isos-aether`；硬件专属集成保留在私有仓库。取得完整资格的安装包发布包含 IsOS manager、EOS 程序及其实际调用的适配器、EI 程序及依赖，以及 Web 和终端客户端。EI 和 EOS 是该发布的组成部分，不作为独立产品分别取得发布资格。源码版本、图格式、HTTP 模式、protobuf 服务、节点类型 ID 和外部软件包身份各自独立版本化。[IsOS 软件包发行契约](isos/interfaces/package/README_cn.md#release)定义经验证的外部软件包精确锁定，不是本产品的发布流程。

## 准备候选版本

从经过审查的集成提交开始，固定其锁文件及生成绑定。记录源码提交、工具链版本、构建环境、契约版本、支持平台和预定标签。检查受跟踪的源码、测试、文档、清单及 Git 历史，排除凭据、私有端点、设备身份、校准数据、已保存路由、采集内容、运行时状态和无许可材料。公开快照不包含部署设置、原生产品技能、站点脚本和本地数据。只保留 Rust 运行时实际调用的 C++ 适配器。

将文档复制到公开的 `isos-aether` 快照之前，修改 [IsOS](isos/core/architecture_cn.md)、[EI](ei/core/architecture.md) 和 [EOS](eos/core/architecture_cn.md) 架构页及其英文对应页，使其只描述该公开提交已支持的行为。在公开文档中省略私有的**目标设计——尚未实现**章节、研究方法。保留当前归属边界及指向权威接口契约的链接。更新所有指向已移除章节的入站链接，再对准确公开快照运行双语链接与文档检查。Git 忽略规则不能从源码发布中移除已跟踪的架构文本；打标签或发布前须对公开快照进行明确且经过审查的文档修改。

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

启用 `ISLAND_BUILD_TESTS=ON` 后，根目录 CTest 会运行 `aether-runtime` 的 Rust 测试，作为 EOS 运行时检查。它不能替代上述完整工作区测试或受管理部署回放。

## 构建并发布统一产品

手动 [Release IsOS 工作流](../.github/workflows/release-isos.yml)只接受已有的 `vMAJOR.MINOR.PATCH` 标签，并要求标签、检出的提交及远端 `master` 一致。Rust、EI 和 UI 源码版本必须与标签匹配。Linux x86_64 和 aarch64 作业分别使用[根安装脚本](../install.sh)构建发布包，验证相邻 SHA-256 文件，将每个包安装到全新目录，再检查已安装命令。两个平台作业均通过后才开始发布；工作流拒绝覆盖已有 GitHub 发布。公开源码和候选发布经过审查后，工作流在 `isos-aether` 中创建发布。

每个发布包包含记录产品版本、源码提交、操作系统和架构的 `RELEASE.json`，以及覆盖已安装文件和发布身份的 `MANIFEST.sha256`。在发布凭据中记录经过审查的清单与外部校验和。不得为了勾选检查表而给未取得资格的开发快照打标签或发布。

IsosUI 构建本地浏览器包、托管浏览器包和终端客户端。产品归档包含本地浏览器包与终端客户端。IsOS 在 `/en/isos` 和 `/cn/isos` 提供本地包，并通过 `/isosui-runtime.json` 提供 API 地址；两个客户端都连接到 IsOS 服务。托管包供另行管理的网站使用，不是第二个产品安装程序。UI 资源沿用 `RELEASE.json` 中的产品版本和源码提交。从 `ui/` 运行 `npm ci`、`npm test`、`npm run typecheck` 和 `npm run build` 重现检查；在 `isos doctor` 后验证已安装 Web 与 TUI 的连接。

开发时在 `ui/` 运行 `npm run dev`：webpack 监听源码变化并保留本地文件系统缓存。该服务器用于源码迭代。发布工作流在准确的标签源码目录运行 `./install.sh`；其 UI 构建在该作业中生成一次生产版本地、托管和 TUI 输出。安装脚本把构建好的本地及 TUI 文件复制到产品归档，并将其哈希记录在 `MANIFEST.sha256`。通过 `./install.sh --bundle /absolute/archive.tar.gz` 安装归档即可使用已发布文件，无需重新构建前端。开发服务器输出不能替代标签构建。
从同一源码目录重复安装时，`install.sh` 会检查 Rust、EI 和 UI 源码及构建输入指纹，然后才复用现有输出；全新的发布作业没有这些标记，会执行构建。`--no-build` 是使用已取得资格的输出进行暂存且跳过所有构建步骤的明确方式。复用本地构建仅用于开发便利，不能代替发布资格检查；发布工作流的全新检出仍执行构建和要求的检查。

## 安装并验证已发布包

在完整源码目录运行 `./install.sh --prefix "$HOME/.local/isos"`，构建并暂存统一产品。使用 `./install.sh --bundle-output /absolute/isos-<version>-linux-<arch>.tar.gz` 导出归档；旁边的 `.sha256` 是外部校验和。下载一对文件后，先运行 `sha256sum -c isos-<version>-linux-<arch>.tar.gz.sha256`，再运行 `./install.sh --bundle /absolute/archive.tar.gz`。安装脚本在切换 `current` 前检查内部清单和主机架构；损坏的包必须保持现有安装不变。`isos update` 只列出发布版本，`isos upgrade [version]` 验证并暂存一个版本。安装后运行 `isos doctor`，检查就绪状态和日志，再完成 Apply → Run → Stop 回放。现有进程在明确重启前继续使用旧程序。

发布说明按版本和日期、操作者可见变化、支持的接口、安装或升级步骤、已知限制及可复现验证编写。已发布标签和产物保持不可变。
