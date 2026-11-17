# 共享契约目录

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [节点、软件包、核心与 IsosUI 接口](README_cn.md)
  - [核心归属方契约](core_cn.md)
  - [归属方实现说明](manager_cn.md)
- 下级目录：
  - [api/](api/README_cn.md)
  - [node/](node/README_cn.md)
  - [package/](package/README_cn.md)
  - [server/](server/README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/isos/interfaces/contract-catalog.md) · [IsOS 接口](README_cn.md) · [共享契约](../../contract_cn.md)

IsOS 维护 [`contract/catalog.json`](../../../contract/catalog.json)。这是共享契约资源的发现与归属目录，独立于图的 `<core>/<layer>/<group>/<node>` 类型和实例身份。每条目录记录包含稳定的 `id`、`purpose`、`origin`、`kind`、`owner`、`consumers`、`domains` 和 `paths`。`owner` 指源码维护方，`consumers` 指只读使用方；`origin` 说明文件是手写、与其他模型核对、生成或引入的依赖。这些标签不授予运行时权限，也不替代拥有方的线协议接口。

| 要查找 | 位置 | 应修改 |
| --- | --- | --- |
| 图节点规则和原生 ABI | `node/` | 对应的 schema 或头文件 |
| 跨归属方服务或载荷 | `proto/` | 对应的 Protobuf 源码 |
| EOS 运行时 JSON 校验 | `schemas/actions/`、`schemas/body/`、`schemas/world/` | 对应的 `<domain>/<name>.proto` 源码 |
| EI 北向校验 | `schemas/northbound/` | 共享 schema 与 EI 线模型 |
| 私有模型提供方校验 | `schemas/models/` | 对应的私有 JSON Schema |
| North HTTP API 描述 | `schemas/north-http.openapi.json` | EOS HTTP Protobuf 标注 |

每个资源恰好属于一个目录。`paths` 是相对于 `contract/` 的 glob 模式。该目录中的每个 `.proto`、`.h` 和 `.json` 资源都必须分类；Markdown 指南和目录文件除外。每个模式至少匹配一个文件。支持的类别为 `graph-schema`、`native-abi`、`protobuf-service`、`protobuf-message`、`protobuf-dependency`、`json-schema`、`generated-json-schema` 和 `generated-openapi`。目录 ID 与领域标签使用小写连字符名称。维护方和使用方使用 `isos`、`ei` 或 `eos`。

首批目录区分节点接口、原生 ABI、拥有方服务、类型化载荷、外部接口、私有模型 schema、Google API 兼容声明、生成的 EOS 运行时 schema、EI 北向 schema 和生成的 North HTTP OpenAPI。项目导入路径相对于 `proto/`；`island.eos.*` 包名保持不变。此调整会改变 Protobuf 描述符文件名，因此需要重新生成消费方代码。`proto/google/api/` 保存采用标准导入路径的最小兼容 HTTP 注解声明。新增契约时，按用途和使用方选择目录，更新路径模式或新增目录，再运行校验器。同时维护拥有方接口页面与生成输出。

在仓库根目录运行 `python3 scripts/check_contract_catalog.py`，查看用途和数量摘要，同时检查元数据、覆盖率和重复归类。使用 `--explain schemas/actions/arm_movej.v1.json` 可查看单个文件的用途和生成它的 Protobuf 源码。添加 `--list` 可列出归类结果；`--catalog`、`--domain` 和 `--consumer` 按标签筛选（例如 `--domain perception --consumer ei`）。`python3 -m unittest tests/test_contract_catalog.py` 覆盖通过、未分类、重复分类和单文件解释情况。目录随契约一起复制到已安装的 `share/isos/contract/`；目录中的路径仍相对于该契约根目录。单个 North HTTP 描述文件直接位于 `schemas/` 下；Protobuf 包名保持稳定；使用者需要根据扁平化后的导入路径重新生成绑定。

更新后的目录于 2026-11-16 完成检查：校验器将 155 个资源分为 21 组，3 项目录测试和 7 项产品安装器测试通过，North HTTP 生成器检查与 Ruff 通过；文档检查覆盖 234 个文件且没有失效链接，28 对 IsOS 中英文页面一致。可复现的命令与结果保存在被忽略的 `data/test/20261116-contract-catalog-usability/checks.log`。

扁平化的 `proto/` 布局于 2026-11-16 使用 `python3 scripts/check_contract_catalog.py`、`python3 -m unittest tests/test_contract_catalog.py -q`、三个契约生成器的 `--check` 命令、`cargo fmt --all -- --check`、`cargo check --workspace --quiet`、`cargo clippy --workspace --all-targets -- -D warnings`、`ei/.venv/bin/python -m pytest ei -q`、Ruff、Pyright、导入规则、产品安装器测试、EOS 线协议检查、R02 用例以及两项文档检查完成验证。全部通过；EI 共 744 项测试通过。命令与输出保存在被忽略的 `data/test/20261116-flat-proto/checks.log`。

扁平化的 `schemas/` 布局于 2026-11-16 使用 EOS 与 North HTTP 生成器检查、EI 北向 schema 检查、目录校验及测试、EOS 契约与清单测试、744 项 EI 测试、产品安装器测试、R02 用例、两项文档检查、Ruff、Pyright、导入规则、Rust 格式检查、`cargo check --workspace --quiet` 和 `cargo clippy --workspace --all-targets -- -D warnings` 完成验证。全部通过。命令与结果保存在被忽略的 `data/test/20261116-flat-schemas/checks.log`。
