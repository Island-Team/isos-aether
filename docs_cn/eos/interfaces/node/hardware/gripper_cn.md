<a id="hardware_gripper_cn"></a>
<a id="hardware_gripper_cn--hardwaregripper-分组"></a>
# hardware/gripper 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/base 分组](base_cn.md)
  - [hardware/camera 分组](camera_cn.md)
  - [hardware/manipulator 分组](manipulator_cn.md)
  - [hardware/mock 分组](mock_cn.md)
  - [hardware/torso 分组](torso_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/hardware/gripper.md#hardware_gripper)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [binary](gripper_cn.md#hardware_gripper_cn--binary)

<a id="hardware_gripper_cn--binary"></a>
<a id="binary"></a>

<a id="hardware_gripper_cn--eoshardwaregripperbinaryv02"></a>
## eos.hardware.gripper.binary@v0.2

软件包贡献的候选节点；仅在精确版本的软件包安装并验证后才显示在节点库中。

<a id="hardware_gripper_cn--节点作用"></a>
### 节点作用

软件包提供的开闭夹爪。

- 输入：可选 `command`：`gripper_binary_command@v0.2`。
- 输出：`observation`：`gripper_state@v0.2`。

<a id="hardware_gripper_cn--所有者与生命周期"></a>
### 所有者与生命周期

EOS 通过 Dispatcher 接纳开闭动作，并要求所选服务就绪及运行绑定的 Stop 证明。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [general-hardware_cn](../../../interfaces_cn.md#general_hardware_cn)。
