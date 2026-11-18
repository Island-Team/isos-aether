<a id="hardware_camera_cn"></a>
<a id="hardware_camera_cn--hardwarecamera-分组"></a>
# hardware/camera 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/base 分组](base_cn.md)
  - [hardware/gripper 分组](gripper_cn.md)
  - [hardware/manipulator 分组](manipulator_cn.md)
  - [hardware/mock 分组](mock_cn.md)
  - [hardware/torso 分组](torso_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/hardware/camera.md#hardware_camera)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [device](camera_cn.md#hardware_camera_cn--device)

<a id="hardware_camera_cn--device"></a>
<a id="device"></a>

<a id="hardware_camera_cn--eoshardwarecameradevicev02"></a>
## eos.hardware.camera.device@v0.2

软件包贡献的候选节点；仅在精确版本的软件包安装并验证后才显示在节点库中。

<a id="hardware_camera_cn--节点作用"></a>
### 节点作用

软件包提供的相机观测设备。

- 输入：无执行器输入。
- 输出：`frame`：`image_frame@v0.2`。

<a id="hardware_camera_cn--所有者与生命周期"></a>
### 所有者与生命周期

所选服务从声明的来源产生已验证帧；相机选择属于设备，不属于 Vision 传输节点。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [general-hardware_cn](../../../interfaces_cn.md#general_hardware_cn)。
