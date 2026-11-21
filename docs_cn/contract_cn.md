# 共享契约与源码资源

<!-- doc-navigation:start -->
<details>
<summary>目录：docs_cn</summary>

浏览本目录的文档及下级目录。

- [仓库](../README.md)
- 同级文档：
  - [文档](README_cn.md)
  - [文档变更记录](changelog_cn.md)
  - [贡献指南](contributing_cn.md)
  - [代码与文档规范](conventions.v2.0.0_cn.md)
  - [IsOS 统一发布](release_cn.md)
- 下级目录：
  - [ei/](ei/README_cn.md)
  - [eos/](eos/README_cn.md)
  - [isos/](isos/README_cn.md)
  - [node/](node/README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../docs/contract.md) · [文档](README_cn.md)

IsOS 管理仓库根目录的 [`contract/`](../contract)；EI 和 EOS 只读使用这些共享契约。所属[接口页面](isos/interfaces/README_cn.md)通过易读的 JSON 示例解释公开载荷；契约文件是用于校验和生成的机器可读源码。产品和 EI 的出厂配置位于仓库根目录的 [`config/`](../config) 中，包括 [`config/defaults/`](../config/defaults)；安装后的产品保留自己的 `share/isos/` 布局。

| 源码目录 | 所属组件与内容 | 源码状态 |
| --- | --- | --- |
| [`contract/catalog.json`](../contract/catalog.json) | IsOS 契约目录与标签；[接口](isos/interfaces/contract-catalog_cn.md) | 手写分类 |
| [`contract/node/`](../contract/node/README.md) | IsOS 节点类型和实例 JSON Schema | 手写契约 |
| [`contract/proto/`](../contract/proto/README.md) | IsOS 管理的共享 Protobuf 服务和类型化载荷源码 | 手写契约 |
| [`contract/proto/google/api/`](../contract/proto/google/api/annotations.proto) | 为标准 Google API HTTP 注解导入路径提供最小兼容声明 | 手写依赖声明 |
| [`contract/schemas/actions/`](../contract/schemas/actions)、[`body/`](../contract/schemas/body)、[`world/`](../contract/schemas/world) | 共享 EOS 运行时 JSON Schema；[`eos.manifest.json`](../contract/schemas/eos.manifest.json)记录身份和校验和 | 从共享 Protobuf 契约生成 |
| [`contract/schemas/models/`](../contract/schemas/models) | 私有、提供方无关的模型请求与响应校验 | 手写契约 |
| [`contract/schemas/northbound/`](../contract/schemas/northbound) | 共享 EI 北向 JSON Schema | 与 EI 线模型核对 |
| [`contract/schemas/north-http.openapi.json`](../contract/schemas/north-http.openapi.json) | 共享 North HTTP 适配器描述 | 从共享 EOS HTTP 注解生成 |

IsOS 维护此源码目录、共享契约源码和生成的 schema。EI 和 EOS 只读使用这些资源；运行时安全和执行职责仍由各组件承担。EI 设置在 `config/ei/` 中编写；`config/defaults/ei/` 中的出厂默认配置在适用处链接到规范的 EI 设置。源码仓库不再有 `share/` 目录。构建和安装规则保留已安装的 `share/isos/` 路径。教学示例位于 [`examples/`](../examples/README.md)；公共字段含义由各组件的 `interfaces/` 页面定义。

2026-11-16 的源码目录扁平化使用以下命令检查：`ei/.venv/bin/python scripts/ei/generate_eos_proto.py --check`、`ei/.venv/bin/python scripts/generate_eos_contracts.py --check`、`ei/.venv/bin/python scripts/generate_north_http_openapi.py --check`、`ei/.venv/bin/python scripts/ei/check_northbound_schemas.py`、`ei/.venv/bin/python -m pytest ei -q`（744 项通过）、`python3 -m unittest crates/isos/tests/test_product_installer.py -q`（7 项通过）、`cargo test -p isos domain::registry::node_schema --quiet`（4 项通过）、`ei/.venv/bin/python tests/eos/test_contract_interfaces.py` 和 `python3 scripts/check_doc_links.py --all`（232 个文件，无失效链接）。安装后的产品仍以 `share/isos/` 为命名空间。 `cargo check --workspace --quiet` 也通过。`cargo test -p isos --quiet` 在 354 项测试中有 6 项失败：单独复跑的入门图 fixture 预期 1280，但得到 128；其他失败报告缺少预构建的 `aether-profile`。命令和结果记录在被忽略的 `data/test/20261116-share-root/checks.log`。

2026-11-16 的源码配置迁移检查结果：安装器 7 项测试通过；EI 的 744 项测试、Ruff、Pyright 和 4 条导入规则通过；IsOS 生成配置的 23 项 Rust 测试和 Clippy 通过；格式、文档链接及 IsOS 中英页面检查通过。命令和结果记录在被忽略的 `data/test/20261116-config-root/checks.log`。

2026-11-16 的根目录契约迁移已通过契约目录、EI 测试、安装器测试、生成器、工作区检查与 Clippy、相关 Rust 测试、格式和文档检查。`package_observer` 的两项集成测试因来源与输入的数据类别 fixture 不一致而失败。复现命令及结果记录在被忽略的 `data/test/20261116-root-contract/checks.log`。
