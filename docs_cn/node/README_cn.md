# 共享节点文档

<!-- doc-navigation:start -->
<details>
<summary>目录：node</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 下级目录：
  - [interfaces/](interfaces/README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../docs/node/README.md) · [文档目录](../README_cn.md)

[共享节点接口](interfaces/README_cn.md)定义 IsOS、EI 和 EOS 共用的图位置、注册类型、端口、数据描述符、连线及注册表准入。[IsOS 节点分组](../isos/interfaces/node/README_cn.md)定义 IsOS 自身的 UI 节点；EI 与 EOS 的执行语义分别由其组件接口定义。

## 文档验证

从仓库根目录运行 `python3 scripts/check_doc_links.py --all`、`python3 crates/isos/tools/check_isos_docs.py`，以及读取器特性、R03、R07、P06 和 UI 的聚焦夹具脚本。2026-11-16 全部通过；完整输出保存在已忽略的 `data/test/20261116-shared-node-interfaces/checks.log`。
