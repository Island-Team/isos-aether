<a id="hardware_torso"></a>
<a id="hardware_torso--hardwaretorso-group"></a>
# hardware/torso group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/base group](base.md)
  - [hardware/camera group](camera.md)
  - [hardware/gripper group](gripper.md)
  - [hardware/manipulator group](manipulator.md)
  - [hardware/mock group](mock.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/hardware/torso_cn.md#hardware_torso_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [normalized_axes](torso.md#hardware_torso--normalized-axes)

<a id="hardware_torso--normalized-axes"></a>
<a id="normalized-axes"></a>

<a id="hardware_torso--eoshardwaretorsonormalizedaxesv02"></a>
## eos.hardware.torso.normalized_axes@v0.2

Package-contributed candidate. It appears in the library only after its exact package is installed and verified.

<a id="hardware_torso--node-behavior"></a>
### Node behavior

Package-backed torso with normalized axes control.

- Input: Optional `command`: `normalized_axes_command@v0.2`.
- Output: No fabricated position output.

<a id="hardware_torso--owner-and-lifecycle"></a>
### Owner and lifecycle

EOS validates the selected family and lease, admits movement through Dispatcher, and requires run-bound Stop proof.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [general-hardware](../../../interfaces.md#general_hardware).
