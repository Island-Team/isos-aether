# EI 核心

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EI 架构](architecture.md)
  - [EI 路线图](roadmap.md)
- 下级目录：
  - [cognition/](cognition/)
  - [evidence/](evidence/)
  - [governance/](governance/)
  - [io/](io/)
  - [optional/](optional/)
  - [runtime/](runtime/)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/ei/core/README.md) · [EI 文档](../README_cn.md)

从当前[架构](architecture.md)开始；未来认知工作见[路线图](roadmap.md)。EI 拥有节点图选定行为的认知功能和安全门。节点层说明 IsOS 节点库中可见的已声明阶段；核心运行时页面说明支持多个阶段的监督、交换和可选设施。节点页面解释各自行为，公开输入输出规则则由 [EI 接口](../interfaces_cn.md#readme_cn)定义。

注册节点的层和组：

- [io](README.md#io)
- [evidence](README.md#evidence)
- [cognition](README.md#cognition)
- [governance](README.md#governance)

核心运行时设计：

- [运行时模块](README.md#core)

## 文档布局验证

所有者阅读路径已使用 `python3 scripts/check_doc_links.py --all` 检查（639 个 Markdown 文件、零个损坏的相对链接），`git diff --check` 也已通过。完整输出保存在忽略目录 `data/test/2026-11-14-doc-layout/checks.log`。

## EI 节点目录验证（2026-11-15）

从仓库根目录运行 `python3 scripts/check_doc_links.py --all`，检查了 545 个 Markdown 文件，未发现损坏的相对链接；`git diff --check` 通过。完整命令输出保存在忽略目录 `data/test/2026-11-15-ei-doc-layout/checks.log`。

运行时页面合并检查（2026-11-15）：`python3 scripts/check_doc_links.py --all` 检查了 539 个 Markdown 文件，未发现损坏的相对链接；`git diff --check` 通过。输出保存在忽略目录 `data/test/2026-11-15-ei-doc-layout/runtime-merge.log`。

## io

IO 阶段接收所有者边界输入，并按声明的连接与其他所有者通信。input 节点接纳触发和机械臂状态事件；link 组说明所有者交换网关。硬件控制仍须经过已声明的安全门与 EOS 分发器路径。

本层的节点组：

- [input](../interfaces/node/design/io/input.md)
- [link](io/link.md)

## evidence

证据层在认知处理之前接纳感知数据。perception 组说明接纳阶段及其边界；源载荷与外部契约仍由 EI 接口页面定义。

本层的节点组：

- [perception](evidence/perception.md)

## cognition

认知层将已接纳的观测和任务状态组织为有界上下文、决策与意图。这些阶段节点沿所选 EI 投影传递事件，不负责执行器准入。各组页面分别介绍上下文构建、规划、任务排队和机械臂状态投影。

本层的节点组：

- [context](cognition/context.md)
- [decision](cognition/decision.md)
- [intent](../interfaces/node/design/cognition/intent.md)
- [missions](cognition/missions.md)
- [state](../interfaces/node/design/cognition/state.md)

## governance

治理层在前序阶段之后执行 EI 的安全与观测策略。安全门是激活请求到达 EOS 前的最后一道 EI 检查；monitor 阶段处理只读健康与机械臂状态观测。

本层的节点组：

- [monitor](../interfaces/node/design/governance/monitor.md)
- [safety](governance/safety.md)

## core

这些页面说明服务于多个注册节点组的 EI 运行时机制。内容按运行时执行、认知支持和可选设施组织，节点页面可以引用共同行为而不重复定义。节点位置和各阶段行为仍由层与组页面说明。

- [cognition](README.md#core-cognition)
- [optional](README.md#core-optional)
- [runtime](runtime/runtime.md)

## core cognition

coordination 说明配置执行如何在认知阶段之间共享状态并安排工作。memory 说明这些阶段使用的保留上下文与回忆。两者支撑认知节点，但本身不是注册节点组。

- [coordination](cognition/coordination.md)
- [memory](cognition/memory.md)

## core optional

可选的 memory 与 sandbox 设施可在选择并配置后增强 EI 运行。相关页面说明启用方式、边界与故障处理；基础节点路径仍由层与组设计说明。

- [memory](optional/memory.md)
- [sandbox](runtime/sandbox.md)
