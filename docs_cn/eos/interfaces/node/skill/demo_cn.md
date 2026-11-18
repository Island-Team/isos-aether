<a id="skill_demo_cn"></a>
<a id="skill_demo_cn--skilldemo-分组"></a>
# skill/demo 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/skill</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/skill/demo.md#skill_demo)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [arm_status](demo_cn.md#skill_demo_cn--arm-status)
- [joint_delta](demo_cn.md#skill_demo_cn--joint-delta)

<a id="skill_demo_cn--arm-status"></a>
<a id="arm-status"></a>

<a id="skill_demo_cn--eosskilldemoarmstatusv02"></a>
## eos.skill.demo.arm_status@v0.2

当前 IsOS 节点库中的内置节点类型。

<a id="skill_demo_cn--节点作用"></a>
### 节点作用

向 EI 方向转发已检查的机械臂状态。

- 输入：`observation`：`arm_monitor_state@v0.2`，进程内。
- 输出：`observation`：`arm_monitor_state@v0.2`，进程内。

<a id="skill_demo_cn--所有者与生命周期"></a>
### 所有者与生命周期

将已检查观测转给图中声明的出口节点；不直接读取硬件。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [skills_cn](demo_cn.md#skill_demo_cn)。

<a id="skill_demo_cn--joint-delta"></a>
<a id="joint-delta"></a>

<a id="skill_demo_cn--eosskilldemojointdeltav02"></a>
## eos.skill.demo.joint_delta@v0.2

当前 IsOS 节点库中的内置节点类型。

<a id="skill_demo_cn--节点作用-1"></a>
### 节点作用

将激活事件转为内置演示增量。

- 输入：`activation`：`button_activation@v0.2`，进程内。
- 输出：`delta`：`arm_joint_delta@v0.2`，进程内。

<a id="skill_demo_cn--所有者与生命周期-1"></a>
### 所有者与生命周期

产生注册的六关节演示增量；移动前由 Dispatcher 负责接纳。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [skills_cn](demo_cn.md#skill_demo_cn)。
