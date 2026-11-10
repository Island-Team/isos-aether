# EI 文档

[English](../../docs/ei/README.md) · 文档索引

EI 拥有认知、任务、证据、本地安全门及其受管工作进程生命周期。IsOS 拥有已保存图和部署编排；EOS 拥有硬件与最终执行器准入。先读[架构](design/architecture.md)，再按边界选择：

| 区域 | 用途 |
| --- | --- |
| [入门](start/README_cn.md) | 准备、启动、检查、恢复和停止 EI。 |
| [接口](interfaces/README.md) | IsOS、EOS、操作员及注册图节点所使用的契约。 |
| [设计](design/architecture.md) | 当前运行路径、图层/组算法与所有权。 |

Python 实现位于 [`ei/`](../../ei/)。正式记忆与重放状态位于 `data/ei/state/`；可重建缓存、诊断、临时文件和实时套接字分别位于 `data/ei/cache/`、`log/`、`tmp/` 与 `run/`。备份状态和用户文件；仅在 EI 停止后清理缓存或临时文件。系统安装使用 `/var/lib/island/ei/` 和 `/run/island/ei/`。`ISLAND_STATE`、`ISLAND_CACHE`、`ISLAND_LOG`、`ISLAND_RUN`、`ISLAND_TMP` 可分别覆盖对应根目录。
