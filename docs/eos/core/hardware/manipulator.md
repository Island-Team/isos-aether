# hardware/manipulator group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/base group](base.md)
  - [hardware/camera group](camera.md)
  - [hardware/gripper group](gripper.md)
  - [Develop EOS hardware adapters](hardware-guide.md)
  - [EOS hardware runtime design](hardware.md)
  - [hardware/mock group](mock.md)
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/hardware/manipulator_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.hardware.manipulator.device@v0.2

Package-backed candidate. Package-backed joint-position manipulator. Input: Optional `command`: `joint_position_command@v0.2`. Output: `observation`: `joint_state@v0.2`. EOS validates ordered joints and limits; Dispatcher admits movement, and Stop quiesces admitted commands before service exit. Interface: [general-hardware](../../interfaces.md#general_hardware).

## eos.hardware.manipulator.normalized_cartesian@v0.2

Package-backed candidate. Package-backed Cartesian manipulator. Input: Optional `command`: normalized axes; optional `mode`: `control_mode_command@v0.2`. Output: `observation`: `pose_state@v0.2`. EOS checks advertised modes and declared frames; Dispatcher admits commands and Stop requires actuator proof. Interface: [general-hardware](../../interfaces.md#general_hardware).

<a id="manipulator-adapter-node"></a>

## Manipulator adapter node

**Placement:** `isos/eos_hardware/manipulator/<instance-id>` · **Type family:** `isos.eos_hardware.manipulator@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Historical `isos.eos_*` design reference. This type is not a current built-in EOS node-library entry.

### Purpose and boundary

One arm owns ordered joint state, optional tool pose, and its physical stop seam. Node profile joint order is authoritative.

### Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

### Lifecycle and algorithm

Validate joint names, order, finite targets, mode and bounds before Dispatcher admission. The adapter reports fresh observed state; an acknowledgment alone does not prove arrival. Stop or uncertain outcome cannot be hidden by a graph edge. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
