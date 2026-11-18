# hardware/camera group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/design/hardware</summary>

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

[中文](../../../../../../docs_cn/eos/interfaces/node/design/hardware/camera_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.hardware.camera.device@v0.2

Package-backed candidate. Package-backed camera observation device. Input: No actuator input. Output: `frame`: `image_frame@v0.2`. The selected service produces validated frames from its declared source; camera selection stays with the device, not the vision transfer node. Interface: [general-hardware](../../../../interfaces.md#general_hardware).

<a id="camera-adapter-node"></a>

## Camera adapter node

**Placement:** `isos/eos_hardware/camera/<instance-id>` · **Type family:** `isos.eos_hardware.camera@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Historical `isos.eos_*` design reference. This type is not a current built-in EOS node-library entry.

### Purpose and boundary

One configured camera owns one adapter and produces typed frames, health, and lifecycle status. RGB is required when advertised; aligned depth is optional and must be reported unavailable when absent.

### Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

### Lifecycle and algorithm

Validate the node profile and adapter family, construct exactly one device adapter, sample into the bounded latest-value hub, then publish only validated fresh frames. A bad frame degrades this camera alone; it never fabricates a replacement frame. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
