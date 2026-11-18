# hardware/gripper group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/base group](base.md)
  - [hardware/camera group](camera.md)
  - [Develop EOS hardware adapters](hardware-guide.md)
  - [EOS hardware runtime design](hardware.md)
  - [hardware/manipulator group](manipulator.md)
  - [hardware/mock group](mock.md)
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/hardware/gripper_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.hardware.gripper.binary@v0.2

Package-backed candidate. Package-backed open/close gripper. Input: Optional `command`: `gripper_binary_command@v0.2`. Output: `observation`: `gripper_state@v0.2`. EOS admits open/close through Dispatcher and requires selected-service readiness and run-bound Stop proof. Interface: [general-hardware](../../interfaces.md#general_hardware).
