# 贡献指南

<!-- doc-navigation:start -->
<details>
<summary>目录：docs_cn</summary>

浏览本目录的文档及下级目录。

- [仓库](../README.md)
- 同级文档：
  - [文档](README_cn.md)
  - [文档变更记录](changelog_cn.md)
  - [共享契约与源码资源](contract_cn.md)
  - [代码与文档规范](conventions.v1.4.0_cn.md)
  - [IsOS 统一发布](release_cn.md)
- 下级目录：
  - [ei/](ei/README_cn.md)
  - [eos/](eos/README_cn.md)
  - [isos/](isos/README_cn.md)
  - [node/](node/README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../docs/contributing.md) · [文档索引](README_cn.md) · [仓库约定](conventions.v1.4.0_cn.md) · [契约地图](contract_cn.md)

本指南说明如何修改 IsOS、EI、EOS、Web 客户端与 TUI。[共享约定](conventions.v1.4.0_cn.md)规定源码和文档规则；所属接口定义公开行为。尚未运行本仓库时，先阅读[首次使用指南](isos/start/quick-start_cn.md)。

## 找到归属方与当前边界

| 变更范围 | 先阅读 | 源码 |
| --- | --- | --- |
| 保存的图、Apply、软件包、本地 API、归属服务监督 | [IsOS 接口](isos/interfaces/README_cn.md)、[架构](isos/core/architecture_cn.md) | `crates/isos/` |
| 图类型 ID、节点实例、端口、注册规则 | [共享节点接口](node/interfaces/README_cn.md) | `contract/node/`、归属方注册表 |
| 认知、北向输入、安全门 | [EI 接口](ei/interfaces_cn.md)、[架构](ei/core/architecture.md) | `ei/` |
| 执行、硬件、技能、Dispatcher 准入 | [EOS 接口](eos/interfaces_cn.md)、[架构](eos/core/architecture_cn.md) | `crates/aether-*/` |
| Web 和终端客户端 | [UI 指南](isos/core/ui/webui_cn.md)、[IsOS HTTP 接口](isos/interfaces/api/http_cn.md) | `ui/` |
| 共享线协议载荷与生成的校验资源 | [契约地图](contract_cn.md)、[目录接口](isos/interfaces/contract-catalog_cn.md) | `contract/` |

修改行为前，阅读所属 `interfaces/` 页面、相关 `start/` 任务和专门的 `core/` 设计。IsOS 负责修改共享契约。EI 和 EOS 只读使用应用后的图与共享资源。执行命令必须经过 EOS Dispatcher 准入。当前行为写入当前指南；未来行为写入所属 `core/roadmap.md`，并标注 **目标设计——尚未实现**。

## 准备工作区

从已确认的集成目标创建专注的 `<change-type>/<topic>` 分支。本仓库使用 `fix`、`test`、`docs` 或 `refactor`，不带共享约定中的 `(v2)` 后缀。保留无关改动，编辑前检查 `git status`。IsOS 和 EOS 使用 Rust，EI 使用 Python，UI 使用 Node.js。本地产品路径运行 `./install_dev.sh`、`./isos doctor` 和 `./isos web`；[首次使用指南](isos/start/quick-start_cn.md)解释软件包发现、Apply、Run、日志与 Stop。专门的变更只需构建并运行受影响的边界。

不要提交被忽略的本机选择、凭据、日志、缓存、构建输出或现场证据。设置放在根目录 `config/`；秘密使用环境变量或被忽略的本地文件。安装后的 `share/isos/` 是安装输出，而不是源码目录。[契约地图](contract_cn.md)区分手写、核对与生成文件。

## 修改契约

1. 找到所属接口和当前调用方。移动或删除字段、文件之前，检查安装、运行时、测试和示例调用方。公开图、HTTP、Protobuf、软件包及源码版本相互独立。
2. 修改权威源码：所属接口页面定义公开含义，`contract/proto/` 或 `contract/node/` 定义共享线协议，`contract/schemas/models/` 定义私有模型校验，EI 北向线模型与核对用 schema 应一起更新。不要手工修改生成的 EOS JSON Schema、EOS 清单或 North HTTP OpenAPI。
3. 同步更新实现、夹具、测试、首次使用说明和中英文页面。接口页面应让读者无需打开 schema 或 `.proto` 文件即可理解线协议：说明字段类型、单位、用途、缺省行为、校验与兼容规则，并用真实线协议字段名给出简短有效的 JSON 请求、成功及错误示例。每个定义只有一个权威位置；设计与使用页面链接至该位置。
4. 重新生成或检查受影响的输出。使用 `ei/.venv/bin/python scripts/generate_eos_contracts.py` 生成 EOS 运行时 JSON，`ei/.venv/bin/python scripts/generate_north_http_openapi.py` 生成 North HTTP OpenAPI，`ei/.venv/bin/python scripts/ei/generate_eos_proto.py` 生成 EI 绑定。使用 `ei/.venv/bin/python scripts/ei/check_northbound_schemas.py` 只读核对 EI 北向 schema。使用 `python3 scripts/check_contract_catalog.py` 检查目录覆盖率；`--explain <path>` 可查询文件的来源与用途。

契约源码、生成文件和文档必须一致。本地检查或安装不代表软件包已通过资格验证或已发布；发行规则见[软件包契约](isos/interfaces/package/README_cn.md)。

## 编写与移动文档

公开 HTTP、WebSocket、RPC、IPC、插件、技能、硬件或数据契约放在归属方 `interfaces/` 树。首次使用任务放在 `start/`，当前实现放在 `core/`，未来工作放在 `core/roadmap.md`。在 `docs_cn/` 中镜像标题、字段含义、JSON 示例与相对链接，并在文件名末尾加 `_cn`。使用缩进且有效的 `json` 代码块展示代表性对象；说明文字写在代码块旁。对于 JSON Lines，展示一条格式化 JSON 记录并说明线上以换行分隔。

移动或新增页面时，更新入站链接、[文档索引](README_cn.md)及[英文文件清单](../doc.md)。使用 `rg` 查找旧路径和引用。提交前检查链接与 IsOS 中英页面：

新增、移动或重命名页面后运行 `python3 scripts/update_doc_navigation.py`。该脚本更新所有中英文页面的可折叠上级目录、同级文档和下级目录链接；`--check` 只报告过期导航，不修改文件。

```sh
python3 scripts/update_doc_navigation.py --check
python3 scripts/check_doc_links.py --all
python3 crates/isos/tools/check_isos_docs.py
git diff --check
```

## 验证变更边界

先运行专项检查，再根据实际风险运行需要的较广检查。[代理指南](../AGENTS.md#change-workflow)列出边界检查要求。常用命令如下：

| 边界 | 检查 |
| --- | --- |
| Rust IsOS 或 EOS | `cargo fmt --all`；`cargo fmt --all -- --check`；`cargo clippy --workspace --all-targets -- -D warnings`；相关 `cargo test -p <package>` |
| EI | 在 `ei/` 中运行 `.venv/bin/ruff check .`、`.venv/bin/pyright`、`PYTHONPATH=.:package .venv/bin/lint-imports --cache-dir ../data/cache/tools/import-linter-ei`、`.venv/bin/pytest -q` 或专项测试 |
| UI | 在 `ui/` 中运行 `npm run typecheck`、`npm test`、`npm run build`，覆盖浏览器与 TUI 输出 |
| 共享契约 | 生成器的 `--check` 命令、`python3 scripts/check_contract_catalog.py`、相关 EOS/EI 线协议与安装器测试 |
| 文档 | 链接、中英对应、清单和 `git diff --check` 检查 |

运行时边界应按适用情况验证成功、拒绝、依赖故障与 Stop。将完整命令和输出保存在被忽略的 `data/test/<run-id>/`；在所属指南总结可复现命令与结果。准确记录已知失败，并区分已有夹具失败和新失败。

## 审查与发布

检查 `git diff` 中的意外更改、生成输出、私有数据、错误示例和过期路径。用 `docs:`、`refactor:`、`fix:`、`test:` 或 `chore:` 前缀提交完整的变更。按照[共享变更与发布流程](conventions.v1.4.0_cn.md#变更与发布流程)处理 PR、测试分支、独立审查、合并顺序和清理。发布前确认集成目标。经审查的软件包发行还需遵循[软件包契约](isos/interfaces/package/README_cn.md)的资格验证和不可变资源规则。

2026-11-16 的易读契约文档检查覆盖全部 234 个 Markdown 文件的相对链接（零失效）、28 对 IsOS 中英文页面（零问题）、变更页面中的 140 个 JSON 代码块（全部可解析）及 `doc.md` 对全部英文文档的覆盖。`git diff --check` 通过。可复现的命令和输出保存在被忽略的 `data/test/20261116-readable-contracts/checks.log`。
