# 文档变更记录

<!-- doc-navigation:start -->
<details>
<summary>目录：docs_cn</summary>

浏览本目录的文档及下级目录。

- [仓库](../README.md)
- 同级文档：
  - [文档](README_cn.md)
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

[English](../docs/changelog.md) · [当前文档](README_cn.md)

本页记录已完成的工作和早期发布计划。当前行为以各所有者指南和接口为准。

## IsosUI 交付

1. **已完成——图与目录读取。** 当前图与所选目录修订限制编辑和 Invoke。见 U01 #40 和所选目录 #44。
2. **已完成——U01 表单与选择器。** 元数据表单及可搜索场景／设备选项保留不可用 ID，并阻止无效 Apply。见 U01 #40。
3. **已完成——U02 连线。** 精确 IsOS 诊断、已路由优先级／频率编辑，以及仅设计／固定路由的无障碍标签由 U02 #41 覆盖。
4. **已完成——U04 节点信息卡。** 悬停、焦点和触摸展示类型与实例描述、端口、单位及所属方报告的健康，不妨碍图编辑。见 U04 #42。
5. **已完成——U03/U04 工作台。** 独立控制、监视、Vision 和日志面板保存有界布局及 Stop 权限；每个受支持活动原语绑定有一个图解析目标、类型化 `primitive_typed` 控件和关联结果跟进。图连线不执行命令。见 U04 #42、U03 #43 和刷新 #45。
6. **已完成——本次运行日志。** Home 使用 IsOS JSONL 游标、固定首事件锚点、缺口提示及来源筛选，并将命令结果与日志区分。见 [P07 UI 契约](isos/interfaces/api/client_cn.md#log_cursors)。
7. **所有者检查已记录。** 中英文首次使用、无障碍交互、Node 22/npm 10 测试及本地／托管构建见 Q03 所有者证据。封存的 Q03 final-17 选定回放和 Q04 final-5 合并后回放通过各自精确输入验证。Final-5 在回环地址模拟托管 Origin；这不证明真实公网 HTTPS 首次使用。当前统一产品发布方式见[产品发布指南](release_cn.md)。

## v0.2.1 仅源码 beta 候选

2026-11-10 的候选说明计划发布源码版本 `0.2.1`、图格式 `0.2.0`、不含预编译安装程序的仅源码 beta。它要求在准确的公开快照上完成检查，并未声称检查已经通过。当时稳定的第三方节点 SDK 和跨平台支持不在候选范围内。当前支持的行为见[发布指南](release_cn.md)和[接口](isos/interfaces/README_cn.md)。
