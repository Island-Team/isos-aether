<a id="hardware_camera"></a>
<a id="hardware_camera--hardwarecamera-group"></a>
# hardware/camera group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/base group](base.md)
  - [hardware/gripper group](gripper.md)
  - [hardware/manipulator group](manipulator.md)
  - [hardware/mock group](mock.md)
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/hardware/camera_cn.md#hardware_camera_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [device](camera.md#hardware_camera--device)

<a id="hardware_camera--device"></a>
<a id="device"></a>

<a id="hardware_camera--eoshardwarecameradevicev02"></a>
## eos.hardware.camera.device@v0.2

Package-contributed candidate. It appears in the library only after its exact package is installed and verified.

<a id="hardware_camera--node-behavior"></a>
### Node behavior

Package-backed camera observation device.

- Input: No actuator input.
- Output: `frame`: `image_frame@v0.2`.

<a id="hardware_camera--owner-and-lifecycle"></a>
### Owner and lifecycle

The selected service produces validated frames from its declared source; camera selection stays with the device, not the vision transfer node.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [general-hardware](../../../interfaces.md#general_hardware).
