# IsosUI 文档

[English](../../docs/isosui/README.md) · [文档](../README_cn.md)

IsosUI 是 IsOS 部署的浏览器工作台。它编辑包含 `isos`、`ei`、`eos` 三个核心的 `0.2.0` 格式图，并呈现各核心声明层中的已注册节点。IsOS 负责图持久化、软件包准入和进程状态。EOS 负责实时控制与观测。工作台呈现这些所有者接口，不定义另一种图格式。

## 阅读地图

- [首次使用](usage_cn.md)介绍如何打开和操作工作台。
- [架构](architecture_cn.md)区分当前客户端边界与未来 UI 目标。
- [目录投影](design_cn.md)解释注册类型如何出现在选择器中。
- [工作区设计](design_cn.md)说明客户端状态和所有者边界；[浏览器结果](design_cn.md#r07-浏览器结果状态)、[日志游标](design_cn.md#p07-日志游标设计)和[原语控制](design_cn.md#u04-原语控制验证)介绍具体流程。
- [接口概览](interfaces/README_cn.md)汇总节点图加载、控制、日志与面板契约。
- [交付记录](todo_cn.md)记录已完成的 UI 阶段与证据。

源码位于 [`ui/`](../../ui/)。本地与托管构建均由同一 TypeScript 项目生成。

## 客户端配置

IsOS 从 [`config/isosui.toml`](../../config/isosui.toml) 读取 Web 和终端客户端默认值。`ISOSUI_CONFIG` 可指定另一个绝对路径的 TOML 文件。进程环境变量覆盖 TOML：`ISOSUI_DEFAULT_INTERFACE`（`web` 或 `tui`）、`ISOSUI_LOCALE`（`en` 或 `cn`）、`ISOSUI_STARTUP_TIMEOUT_MS`（1000–60000）及 `ISOSUI_POLL_INTERVAL_MS`（500–10000）。IsOS 校验解析后的设置，并向两个客户端提供非秘密值。持久偏好写入 TOML；临时覆盖使用环境变量。
