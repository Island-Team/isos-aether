# skill/demo 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/design/skill</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/eos/interfaces/node/design/skill/demo.md)

以下概述当前 EOS 节点类型及其运行时职责。准确载荷与拒绝规则见所链接的所有者接口。

## eos.skill.demo.arm_status@v0.2

向 EI 方向转发已检查的机械臂状态。 输入：`observation`：`arm_monitor_state@v0.2`，进程内。 输出：`observation`：`arm_monitor_state@v0.2`，进程内。 将已检查观测转给图中声明的出口节点；不直接读取硬件。 接口：[skills_cn](../../skill/demo_cn.md#skill_demo_cn)。

## eos.skill.demo.joint_delta@v0.2

将激活事件转为内置演示增量。 输入：`activation`：`button_activation@v0.2`，进程内。 输出：`delta`：`arm_joint_delta@v0.2`，进程内。 产生注册的六关节演示增量；移动前由 Dispatcher 负责接纳。 接口：[skills_cn](../../skill/demo_cn.md#skill_demo_cn)。
