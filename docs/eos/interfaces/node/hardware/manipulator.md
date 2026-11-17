<a id="hardware_manipulator"></a>
<a id="hardware_manipulator--hardwaremanipulator-group"></a>
# hardware/manipulator group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/base group](base.md)
  - [hardware/camera group](camera.md)
  - [hardware/gripper group](gripper.md)
  - [hardware/mock group](mock.md)
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/hardware/manipulator_cn.md#hardware_manipulator_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [device](manipulator.md#hardware_manipulator--device)
- [normalized_cartesian](manipulator.md#hardware_manipulator--normalized-cartesian)

<a id="hardware_manipulator--device"></a>
<a id="device"></a>

<a id="hardware_manipulator--eoshardwaremanipulatordevicev02"></a>
## eos.hardware.manipulator.device@v0.2

Package-contributed candidate. It appears in the library only after its exact package is installed and verified.

<a id="hardware_manipulator--node-behavior"></a>
### Node behavior

Package-backed joint-position manipulator.

- Input: Optional `command`: `joint_position_command@v0.2`.
- Output: `observation`: `joint_state@v0.2`.

<a id="hardware_manipulator--owner-and-lifecycle"></a>
### Owner and lifecycle

EOS validates ordered joints and limits; Dispatcher admits movement, and Stop quiesces admitted commands before service exit.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [general-hardware](../../../interfaces.md#general_hardware).

<a id="hardware_manipulator--normalized-cartesian"></a>
<a id="normalized-cartesian"></a>

<a id="hardware_manipulator--eoshardwaremanipulatornormalizedcartesianv02"></a>
## eos.hardware.manipulator.normalized_cartesian@v0.2

Package-contributed candidate. It appears in the library only after its exact package is installed and verified.

<a id="hardware_manipulator--node-behavior-1"></a>
### Node behavior

Package-backed Cartesian manipulator.

- Input: Optional `command`: normalized axes; optional `mode`: `control_mode_command@v0.2`.
- Output: `observation`: `pose_state@v0.2`.

<a id="hardware_manipulator--owner-and-lifecycle-1"></a>
### Owner and lifecycle

EOS checks advertised modes and declared frames; Dispatcher admits commands and Stop requires actuator proof.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [general-hardware](../../../interfaces.md#general_hardware).
