# IsosUI 接口

[English](../../../docs/isosui/interfaces/README.md) · [IsosUI 概览](../README_cn.md)

这些契约说明浏览器通过 IsOS 显示和发送的内容。IsOS 负责节点图修订、运行、软件包与 Stop；EOS 负责命令准入、健康状态与结果。浏览器保存本地展示状态，在控制前重新核验所有者证据。

| 契约 | 范围 |
| --- | --- |
| [控制与结果](control_cn.md) | 节点图绑定、类型化原语发送、关联结果读取与 Stop 展示。 |
| [节点图加载](graph-loading_cn.md) | 修订一致的节点图与目录选择、重新加载及轮询。 |
| [当前运行日志](log-cursors_cn.md) | 锚点、筛选、游标、缺口与运行边界。 |
| [工作面板与 Vision](workspace-panels_cn.md) | 停靠、持久化、可见性、Vision 来源验证与画面显示。 |

操作顺序见[首次使用指南](../usage_cn.md)，客户端状态与验证证据见[设计指南](../design_cn.md)。
