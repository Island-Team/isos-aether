<a id="hardware_base"></a>
<a id="hardware_base--hardwarebase-group"></a>
# hardware/base group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/camera group](camera.md)
  - [hardware/gripper group](gripper.md)
  - [hardware/manipulator group](manipulator.md)
  - [hardware/mock group](mock.md)
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/hardware/base_cn.md#hardware_base_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [device](base.md#hardware_base--device)
- [normalized_axes](base.md#hardware_base--normalized-axes)

<a id="hardware_base--device"></a>
<a id="device"></a>

<a id="hardware_base--eoshardwarebasedevicev02"></a>
## eos.hardware.base.device@v0.2

Package-contributed candidate. It appears in the library only after its exact package is installed and verified.

<a id="hardware_base--node-behavior"></a>
### Node behavior

Package-backed base device.

- Input: Optional `command`: `base_twist_command@v0.2`.
- Output: `observation`: `base_state@v0.2`.

<a id="hardware_base--owner-and-lifecycle"></a>
### Owner and lifecycle

EOS binds the declared actuator port to Dispatcher, validates family and endpoint readiness, and requires run-bound Stop proof before service exit.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [general-hardware](../../../interfaces.md#general_hardware).

<a id="hardware_base--normalized-axes"></a>
<a id="normalized-axes"></a>

<a id="hardware_base--eoshardwarebasenormalizedaxesv02"></a>
## eos.hardware.base.normalized_axes@v0.2

Package-contributed candidate. It appears in the library only after its exact package is installed and verified.

<a id="hardware_base--node-behavior-1"></a>
### Node behavior

Package-backed base with normalized axes control.

- Input: Optional `command`: `normalized_axes_command@v0.2`.
- Output: `observation`: `pose_state@v0.2`.

<a id="hardware_base--owner-and-lifecycle-1"></a>
### Owner and lifecycle

EOS validates the selected device family, frame and lease bounds; Dispatcher admits commands and Stop quiesces the actuator.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [general-hardware](../../../interfaces.md#general_hardware).
