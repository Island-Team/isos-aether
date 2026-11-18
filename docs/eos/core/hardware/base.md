# hardware/base group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/camera group](camera.md)
  - [hardware/gripper group](gripper.md)
  - [Develop EOS hardware adapters](hardware-guide.md)
  - [EOS hardware runtime design](hardware.md)
  - [hardware/manipulator group](manipulator.md)
  - [hardware/mock group](mock.md)
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/hardware/base_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.hardware.base.device@v0.2

Package-backed candidate. Package-backed base device. Input: Optional `command`: `base_twist_command@v0.2`. Output: `observation`: `base_state@v0.2`. EOS binds the declared actuator port to Dispatcher, validates family and endpoint readiness, and requires run-bound Stop proof before service exit. Interface: [general-hardware](../../interfaces.md#general_hardware).

## eos.hardware.base.normalized_axes@v0.2

Package-backed candidate. Package-backed base with normalized axes control. Input: Optional `command`: `normalized_axes_command@v0.2`. Output: `observation`: `pose_state@v0.2`. EOS validates the selected device family, frame and lease bounds; Dispatcher admits commands and Stop quiesces the actuator. Interface: [general-hardware](../../interfaces.md#general_hardware).

<a id="mobile-base-adapter-node"></a>

## Mobile base adapter node

**Placement:** `isos/eos_hardware/base/<instance-id>` · **Type family:** `isos.eos_hardware.base@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Historical `isos.eos_*` design reference. This type is not a current built-in EOS node-library entry.

### Purpose and boundary

One base owns observations and the physical stop seam. An input edge describes the command route but cannot authorize direct actuation.

### Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

### Lifecycle and algorithm

Resolve the verified adapter, validate command source and typed limits, ask Dispatcher to admit, then release to the one adapter. Lease expiry or stop calls the adapter stop seam; uncertain completion remains `OutcomeUnknown` and is never retried automatically. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
