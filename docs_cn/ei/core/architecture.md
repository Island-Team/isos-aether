# EI 架构

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EI 核心](README.md)
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

[English](../../../docs/ei/core/architecture.md) · [EI](../README_cn.md) · [接口](../interfaces_cn.md#readme_cn)

EI 负责认知、任务规划、自身安全门和认知记忆。它消费获准观测并提出动作；只有 EOS 准入执行器命令并负责硬件安全。IsOS 负责已保存图、软件包验证和部署运行身份。当前边界由 [EI setup](../../isos/interfaces/server/ei_cn.md#setup_cn)、[节点](../../isos/interfaces/server/ei_cn.md#node_cn)和 [EOS 交换](runtime/io.md#南向操作处理)定义。

## 当前运行边界

EI 的持久归属服务验证独立的 `cores.ei` 投影，选择一个运行，一次监管一个受管工作进程。IsOS 调用 Run、Status、Stop 和激活；manager 退出不会终止 EI。在图 `0.2.1` 中，三个激活节点将同一个 `button_activation@v0.2` 从输入经意图传至安全门。EI 不选择机械臂关节值，也不创建执行器通道。安全门检查运行、修订、声明路径、工作进程存活及 EOS 端点；下游结果不确定时原样报告，不自动重试。Stop 与激活串行，并关闭准入。

独立的只读监测链将 EOS 最新已检查机械臂采样经 EI 输入、状态和监测送往 IsOS。EI 保留来源序号、采样时间、状态、原因、关节名称及有效位置；过期或故障采样仍是观测。当前受管健康工作进程与归属方的数据传递分离。当前路径尚未实现统一张量空间、学习式 VLA 技能路由或世界模型控制环。

归属服务在启动前验证工作程序摘要与运行身份。仅报告健康的工作进程没有安全状态或执行器权限，不占用完整 EI 运行时的全局安全状态锁，因此不同部署可在同一状态根目录下维持健康工作进程。2026-11-09 并行归属服务检查运行两个这样的子进程，随后 Stop 回收两者；完整 EI 测试、Ruff、Pyright、导入规则和双语链接检查通过。完整输出位于忽略的 `data/test/ei-arm-monitor-20261109/`。

未来方向见[路线图](roadmap.md)。
