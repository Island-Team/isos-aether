<a id="dispatcher_arm_cn"></a>
<a id="dispatcher_arm_cn--dispatcherarm-分组"></a>
# dispatcher/arm 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/dispatcher</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/dispatcher/arm.md#dispatcher_arm)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [delta](arm_cn.md#dispatcher_arm_cn--delta)
- [monitor](arm_cn.md#dispatcher_arm_cn--monitor)

<a id="dispatcher_arm_cn--delta"></a>
<a id="delta"></a>

<a id="dispatcher_arm_cn--eosdispatcherarmdeltav02"></a>
## eos.dispatcher.arm.delta@v0.2

当前 IsOS 节点库中的内置节点类型。

<a id="dispatcher_arm_cn--节点作用"></a>
### 节点作用

在模拟机械臂移动前接纳六关节演示命令。

- 输入：`delta`：`arm_joint_delta@v0.2`，进程内。
- 输出：`delta`：`arm_joint_delta@v0.2`，进程内。

<a id="dispatcher_arm_cn--所有者与生命周期"></a>
### 所有者与生命周期

在命令交给图中声明的模拟适配器前，检查运行身份、健康和移动限值。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [arm-control_cn](../../../interfaces_cn.md#arm_control_cn)。

<a id="dispatcher_arm_cn--monitor"></a>
<a id="monitor"></a>

<a id="dispatcher_arm_cn--eosdispatcherarmmonitorv02"></a>
## eos.dispatcher.arm.monitor@v0.2

当前 IsOS 节点库中的内置节点类型。

<a id="dispatcher_arm_cn--节点作用-1"></a>
### 节点作用

检查模拟机械臂的返回观测。

- 输入：`observation`：`arm_monitor_state@v0.2`，进程内。
- 输出：`observation`：`arm_monitor_state@v0.2`，进程内。

<a id="dispatcher_arm_cn--所有者与生命周期-1"></a>
### 所有者与生命周期

拒绝过期或无效样本；监视路径独立于控制结果读取。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [arm-monitor_cn](../../../interfaces_cn.md#arm_monitor_cn)。
