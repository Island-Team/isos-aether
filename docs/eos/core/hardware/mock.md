# hardware/mock group

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
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/hardware/mock_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.hardware.mock.arm@v0.2

Holds six mock joint positions. Input: `delta`: `arm_joint_delta@v0.2`, in process. Output: `observation`: `arm_monitor_state@v0.2`, in process. The `mock_arm` adapter starts at zero in a run, applies admitted distinct deltas, and samples joint positions. Stop closes admission before child exit. Interface: [arm](../../interfaces/node/hardware/mock.md#hardware_mock--arm).

## eos.hardware.mock.arm_observer@v0.2

Reads the sole mock-arm sampler independently. Input: No graph input. Output: `observation`: `arm_monitor_state@v0.2`, in process. The observation source follows the active run; it cannot admit or move an actuator. Interface: [arm-monitor](../../interfaces.md#arm_monitor).

## eos.hardware.mock.arm_output@v0.2

Passes mock-arm observation to monitoring. Input: `observation`: `arm_monitor_state@v0.2`, in process. Output: `observation`: `arm_monitor_state@v0.2`, in process. Compatibility type for saved graphs; it forwards observation and does not create another actuator route. Interface: [arm](../../interfaces/node/hardware/mock.md#hardware_mock--arm).
