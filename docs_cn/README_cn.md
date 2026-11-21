# 文档

<!-- doc-navigation:start -->
<details>
<summary>目录：docs_cn</summary>

浏览本目录的文档及下级目录。

- [仓库](../README.md)
- 同级文档：
  - [文档变更记录](changelog_cn.md)
  - [共享契约与源码资源](contract_cn.md)
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

[English](../docs/README.md) · [仓库](../README_cn.md)

IsOS 负责保存图、验证软件包、Apply 和部署运行身份。EI 与 EOS 各自管理持久的进程生命周期接口及工作进程健康状态。当前 Graph `0.2.1` 包含 `isos`、`ei`、`eos` 三个核心，节点类型 ID 为 `<core>.<layer>.<group>.<node>@v0.2`。当前演示图包含独立的按钮到机械臂链与机械臂观测链；参见[节点接口](isos/interfaces/node/README_cn.md)。

| 阅读入口 | 用途 |
| --- | --- |
| [共享节点](node/README_cn.md) | 所有者共用的图、类型、端口、数据和注册表契约。 |
| [IsOS 入门](isos/start/quick-start_cn.md) | 配置、验证软件包、应用、运行、检查与停止。 |
| [EI](ei/README_cn.md) | 认知、安全及所有者接口。 |
| [EOS](eos/README_cn.md) | 执行、硬件、调度器及仿真器契约。 |
| [IsosUI](isos/core/ui/webui_cn.md) | 图工作台和所有者实时状态。 |
| [IsOS 架构](isos/core/architecture_cn.md) | 跨所有者契约、数据流与运行时目标。 |
| [统一发布](release_cn.md) | 单一产品发布、资格门禁及已发布包的验证。 |
| [变更记录](changelog_cn.md) | 已完成的 UI 交付和早期源码 beta 计划。 |
| [贡献指南](contributing_cn.md) | 全仓库贡献入口与文档变更流程。 |
| [约定](conventions.v2.0.0_cn.md) | 源码、契约和文档规则。 |
| [共享契约](contract_cn.md) | 手写、核对及生成的契约文件的归属与位置。 |

公开字段和线协议由所属组件的 `interfaces/` 文档通过易读的 JSON 示例定义。[共享契约地图](contract_cn.md)供贡献者查找机器可读源码与生成资源。源码版本 `0.2.3` 与图格式 `0.2.1` 独立于 HTTP、protobuf 和软件包标识。

## 当前节点边界

| 范围 | 可用行为 | 归属契约 |
| --- | --- | --- |
| 起始图 | 当前 17 节点演示图具有文档所述的归属方路由和运行路径。 | [图节点](isos/interfaces/node/README_cn.md) |
| EI 与 EOS 隔离库 | 可通过类型化的单节点直接调用及测试夹具使用。新增 ID 不属于图支持的身份，普通图设置会拒绝它们。 | [EI 认知](ei/interfaces_cn.md#cognition_isolated_library_cn)及 [EOS 处理器](eos/interfaces_cn.md#isolated_library_cn) |
| EOS 物理场景类型 | 已注册用于验证的元数据与引用描述符；没有随之提供节点、解码器、产物读取器或采集路径。 | [场景数据](eos/interfaces_cn.md#scene_data_cn) |

当前指南描述已实现的行为。未来算法与目标行为放在 [IsOS](isos/core/roadmap_cn.md#目标架构尚未实现)、[EI](ei/core/roadmap.md#目标架构尚未实现) 和 [EOS](eos/core/roadmap_cn.md#目标架构尚未实现) 路线图中。

## 文档布局验证

2026-11-15 的节点与运行时文档布局变更在仓库根目录运行 `python3 scripts/check_doc_links.py --all`、`python3 crates/isos/tools/check_isos_docs.py`、使用 `scripts/check_doc_links.py` 的 `check_file` 进行全仓库 Markdown 链接检查、对照 `doc.md` 核对英文文件清单，以及 `git diff --check`。检查均通过：相对链接无损坏、71 对 IsOS 中英文页面对齐，并且 `doc.md` 覆盖全部 274 个英文 Markdown 文件。完整输出保存在忽略的 `data/test/20261115-node-core-doc-layout/checks.log`。

后续按语义合并的文档使用相同的文档和文件清单检查，另运行 `python3 tests/fixtures/r03_admission_revision/check_cases.py`。全部通过：相对链接无损坏、65 对 IsOS 中英文页面对齐、`doc.md` 覆盖全部 212 个英文 Markdown 文件，且 25 个冻结的 R03 用例对应当前文档锚点。完整输出保存在忽略的 `data/test/20261115-merge-related-docs/checks.log`。

2026-11-15 的 README 精简将 EOS 运行时及外部接口导航移入所属目录索引，并移除三对重复的中英文 README。在仓库根目录运行 `python3 scripts/check_doc_links.py --all`，检查 365 个 Markdown 文件且相对链接错误为零；运行 `python3 crates/isos/tools/check_isos_docs.py`，检查 65 对 IsOS 中英文页面且问题为零；`git diff --check` 通过。完整输出保存在被忽略的 `data/test/20261115-prune-readmes/`。历史清单 `doc.md` 保持原样。
