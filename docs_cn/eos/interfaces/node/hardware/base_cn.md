<a id="hardware_base_cn"></a>
<a id="hardware_base_cn--hardwarebase-分组"></a>
# hardware/base 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/camera 分组](camera_cn.md)
  - [hardware/gripper 分组](gripper_cn.md)
  - [hardware/manipulator 分组](manipulator_cn.md)
  - [hardware/mock 分组](mock_cn.md)
  - [hardware/torso 分组](torso_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/hardware/base.md#hardware_base)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [device](base_cn.md#hardware_base_cn--device)
- [normalized_axes](base_cn.md#hardware_base_cn--normalized-axes)

<a id="hardware_base_cn--device"></a>
<a id="device"></a>

<a id="hardware_base_cn--eoshardwarebasedevicev02"></a>
## eos.hardware.base.device@v0.2

软件包贡献的候选节点；仅在精确版本的软件包安装并验证后才显示在节点库中。

<a id="hardware_base_cn--节点作用"></a>
### 节点作用

软件包提供的底盘设备。

- 输入：可选 `command`：`base_twist_command@v0.2`。
- 输出：`observation`：`base_state@v0.2`。

<a id="hardware_base_cn--所有者与生命周期"></a>
### 所有者与生命周期

EOS 把声明的执行器端口绑定 Dispatcher，验证设备族及端点就绪，并在服务退出前要求运行绑定的 Stop 证明。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [general-hardware_cn](../../../interfaces_cn.md#general_hardware_cn)。

<a id="hardware_base_cn--normalized-axes"></a>
<a id="normalized-axes"></a>

<a id="hardware_base_cn--eoshardwarebasenormalizedaxesv02"></a>
## eos.hardware.base.normalized_axes@v0.2

软件包贡献的候选节点；仅在精确版本的软件包安装并验证后才显示在节点库中。

<a id="hardware_base_cn--节点作用-1"></a>
### 节点作用

软件包提供的归一化轴底盘。

- 输入：可选 `command`：`normalized_axes_command@v0.2`。
- 输出：`observation`：`pose_state@v0.2`。

<a id="hardware_base_cn--所有者与生命周期-1"></a>
### 所有者与生命周期

EOS 验证所选设备族、坐标系和租期边界；Dispatcher 接纳命令，Stop 使执行器静止。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [general-hardware_cn](../../../interfaces_cn.md#general_hardware_cn)。
