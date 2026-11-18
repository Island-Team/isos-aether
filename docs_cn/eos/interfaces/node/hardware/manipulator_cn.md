<a id="hardware_manipulator_cn"></a>
<a id="hardware_manipulator_cn--hardwaremanipulator-分组"></a>
# hardware/manipulator 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/base 分组](base_cn.md)
  - [hardware/camera 分组](camera_cn.md)
  - [hardware/gripper 分组](gripper_cn.md)
  - [hardware/mock 分组](mock_cn.md)
  - [hardware/torso 分组](torso_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/hardware/manipulator.md#hardware_manipulator)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [device](manipulator_cn.md#hardware_manipulator_cn--device)
- [normalized_cartesian](manipulator_cn.md#hardware_manipulator_cn--normalized-cartesian)

<a id="hardware_manipulator_cn--device"></a>
<a id="device"></a>

<a id="hardware_manipulator_cn--eoshardwaremanipulatordevicev02"></a>
## eos.hardware.manipulator.device@v0.2

软件包贡献的候选节点；仅在精确版本的软件包安装并验证后才显示在节点库中。

<a id="hardware_manipulator_cn--节点作用"></a>
### 节点作用

软件包提供的关节位置机械臂。

- 输入：可选 `command`：`joint_position_command@v0.2`。
- 输出：`observation`：`joint_state@v0.2`。

<a id="hardware_manipulator_cn--所有者与生命周期"></a>
### 所有者与生命周期

EOS 验证关节顺序及限值；Dispatcher 接纳移动，Stop 在服务退出前使已接纳命令静止。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [general-hardware_cn](../../../interfaces_cn.md#general_hardware_cn)。

<a id="hardware_manipulator_cn--normalized-cartesian"></a>
<a id="normalized-cartesian"></a>

<a id="hardware_manipulator_cn--eoshardwaremanipulatornormalizedcartesianv02"></a>
## eos.hardware.manipulator.normalized_cartesian@v0.2

软件包贡献的候选节点；仅在精确版本的软件包安装并验证后才显示在节点库中。

<a id="hardware_manipulator_cn--节点作用-1"></a>
### 节点作用

软件包提供的笛卡尔机械臂。

- 输入：可选 `command`：归一化轴；可选 `mode`：`control_mode_command@v0.2`。
- 输出：`observation`：`pose_state@v0.2`。

<a id="hardware_manipulator_cn--所有者与生命周期-1"></a>
### 所有者与生命周期

EOS 检查已公布模式及声明的坐标系；Dispatcher 接纳命令，Stop 要求执行器证明。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [general-hardware_cn](../../../interfaces_cn.md#general_hardware_cn)。
