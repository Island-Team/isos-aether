<a id="hardware_torso_cn"></a>
<a id="hardware_torso_cn--hardwaretorso-分组"></a>
# hardware/torso 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/base 分组](base_cn.md)
  - [hardware/camera 分组](camera_cn.md)
  - [hardware/gripper 分组](gripper_cn.md)
  - [hardware/manipulator 分组](manipulator_cn.md)
  - [hardware/mock 分组](mock_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/hardware/torso.md#hardware_torso)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [normalized_axes](torso_cn.md#hardware_torso_cn--normalized-axes)

<a id="hardware_torso_cn--normalized-axes"></a>
<a id="normalized-axes"></a>

<a id="hardware_torso_cn--eoshardwaretorsonormalizedaxesv02"></a>
## eos.hardware.torso.normalized_axes@v0.2

软件包贡献的候选节点；仅在精确版本的软件包安装并验证后才显示在节点库中。

<a id="hardware_torso_cn--节点作用"></a>
### 节点作用

软件包提供的归一化轴躯干。

- 输入：可选 `command`：`normalized_axes_command@v0.2`。
- 输出：无虚构的位置输出。

<a id="hardware_torso_cn--所有者与生命周期"></a>
### 所有者与生命周期

EOS 验证所选设备族及租期，经 Dispatcher 接纳移动，并要求运行绑定的 Stop 证明。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [general-hardware_cn](../../../interfaces_cn.md#general_hardware_cn)。
