# Governance / monitor

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/design/governance</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [safety 节点设计](safety.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/ei/interfaces/node/design/governance/monitor.md) · [治理层](../../../../core/README.md#governance)

监控节点通过选定的 EI 投影传递只读健康和机械臂状态观测。它们不提交执行器命令，也不改变 EOS 准入。[节点接口](../../governance/monitor_cn.md#governance_monitor_cn)标识已注册的监控阶段；[EI–EOS 接口](../../../../core/runtime/io.md#南向操作处理)定义所有者交换。
