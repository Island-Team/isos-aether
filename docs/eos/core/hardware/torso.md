# hardware/torso group

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
  - [hardware/manipulator group](manipulator.md)
  - [hardware/mock group](mock.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/hardware/torso_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.hardware.torso.normalized_axes@v0.2

Package-backed candidate. Package-backed torso with normalized axes control. Input: Optional `command`: `normalized_axes_command@v0.2`. Output: No fabricated position output. EOS validates the selected family and lease, admits movement through Dispatcher, and requires run-bound Stop proof. Interface: [general-hardware](../../interfaces.md#general_hardware).
