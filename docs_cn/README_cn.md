# 文档

[English](../docs/README.md) · [仓库](../README_cn.md)

IsOS 负责保存图、验证软件包、Apply 和部署运行身份。EI 与 EOS 各自管理持久的进程生命周期接口及工作进程健康状态。当前 Graph `0.2.0` 包含 `isos`、`ei`、`eos` 三个核心，节点类型 ID 为 `<core>.<layer>.<group>.<node>@v0.2`。当前演示图包含独立的按钮到机械臂链与机械臂观测链；参见[节点接口](isos/interfaces/node/README_cn.md)。

| 阅读入口 | 用途 |
| --- | --- |
| [IsOS 入门](isos/start/README_cn.md) | 配置、验证软件包、应用、运行、检查与停止。 |
| [EI](ei/README_cn.md) | 认知、安全及所有者接口。 |
| [EOS 入门](eos/start/README_cn.md) | 执行、硬件、调度器及仿真器操作。 |
| [IsosUI](isosui/README_cn.md) | 图工作台和所有者实时状态。 |
| [共用架构](island/README_cn.md) | 跨所有者契约与数据流。 |
| [统一发布](island/release_cn.md) | 单一产品发布、资格门禁及已发布包的验证。 |
| [v0.2.1 源码 beta 说明](island/release-0.2.1_cn.md) | 候选版本范围、兼容性边界及仅源码验证。 |
| [约定](island/conventions.v1.4.0_cn.md) | 源码、契约和文档规则。 |

公开字段和线协议由所属组件的 `interfaces/` 文档定义。源码版本 `0.2.1` 与图格式 `0.2.0` 独立于 HTTP、protobuf 和软件包标识。

当前指南描述已实现的行为。未来算法与目标行为仅放在明确标记的架构章节，入口为[共用架构](island/architecture_cn.md#目标架构尚未实现)。
