<a id="hardware_gripper"></a>
<a id="hardware_gripper--hardwaregripper-group"></a>
# hardware/gripper group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/base group](base.md)
  - [hardware/camera group](camera.md)
  - [hardware/manipulator group](manipulator.md)
  - [hardware/mock group](mock.md)
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/hardware/gripper_cn.md#hardware_gripper_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [binary](gripper.md#hardware_gripper--binary)

<a id="hardware_gripper--binary"></a>
<a id="binary"></a>

<a id="hardware_gripper--eoshardwaregripperbinaryv02"></a>
## eos.hardware.gripper.binary@v0.2

Package-contributed candidate. It appears in the library only after its exact package is installed and verified.

<a id="hardware_gripper--node-behavior"></a>
### Node behavior

Package-backed open/close gripper.

- Input: Optional `command`: `gripper_binary_command@v0.2`.
- Output: `observation`: `gripper_state@v0.2`.

<a id="hardware_gripper--owner-and-lifecycle"></a>
### Owner and lifecycle

EOS admits open/close through Dispatcher and requires selected-service readiness and run-bound Stop proof.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [general-hardware](../../../interfaces.md#general_hardware).
