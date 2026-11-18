# EI 文档

<!-- doc-navigation:start -->
<details>
<summary>目录：ei</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EI interfaces](interfaces_cn.md)
- 下级目录：
  - [core/](core/README.md)
  - [interfaces/](interfaces/)

</details>
<!-- doc-navigation:end -->

[English](../../docs/ei/README.md) · 文档索引

EI 拥有认知、任务、证据、本地安全门及其受管工作进程生命周期。IsOS 拥有已保存图和部署编排；EOS 拥有硬件与最终执行器准入。先读[架构](core/architecture.md)，再按边界选择：

| 区域 | 用途 |
| --- | --- |
| [入门](../isos/start/quick-start_cn.md) | 准备、启动、检查、恢复和停止 EI。 |
| [服务端接口](interfaces/server_cn.md) | 北向 WebSocket 路由与负载。 |
| [节点分组](interfaces/node/README_cn.md) | 已注册的 `<layer>/<group>` 节点契约。 |
| [接口](interfaces_cn.md#readme_cn) | IsOS、EOS、操作员及注册图节点所使用的契约。 |
| [设计](core/README.md) | 当前运行路径、图层/组算法与所有权。 |

Python 实现位于 [`ei/`](../../ei)。正式记忆与重放状态位于 `data/ei/state/`；可重建缓存、诊断、临时文件和实时套接字分别位于 `data/ei/cache/`、`log/`、`tmp/` 与 `run/`。备份状态和用户文件；仅在 EI 停止后清理缓存或临时文件。系统安装使用 `/var/lib/island/ei/` 和 `/run/island/ei/`。`ISLAND_STATE`、`ISLAND_CACHE`、`ISLAND_LOG`、`ISLAND_RUN`、`ISLAND_TMP` 可分别覆盖对应根目录。

源码边界：`ei/core/` 负责认知与生命周期，`ei/io/` 负责外部传输与 wire 模型，`ei/contracts/` 只读消费 IsOS 所拥有的 schema，`ei/generated/` 存放生成的 EOS 绑定，`ei/lib/` 存放共享实现辅助代码，`ei/package/` 存放私有外部程序。EI 仓库级工具位于 `scripts/ei/`。共享 schema 由 IsOS 在 `contract/` 中维护；EI 据此校验，运行时不修改这些类型。
