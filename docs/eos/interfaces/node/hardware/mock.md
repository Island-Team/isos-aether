<a id="hardware_mock"></a>
<a id="hardware_mock--hardwaremock-group"></a>
# hardware/mock group

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
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/hardware/mock_cn.md#hardware_mock_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [arm](mock.md#hardware_mock--arm)
- [arm_observer](mock.md#hardware_mock--arm-observer)
- [arm_output](mock.md#hardware_mock--arm-output)

<a id="hardware_mock--arm"></a>
<a id="arm"></a>

<a id="hardware_mock--eos-mock-arm-hardware-boundary"></a>
## EOS mock arm hardware boundary

The graph `0.2.1` registers `eos.hardware.mock.arm@v0.2` as a six-joint Manipulator/Adapter node. Its input carries `arm_joint_delta@v0.2` from the EOS dispatcher. The node declares six ordered joint names, the `mock-arm` adapter family, `joint_positions` capability, health timing, observation bounds, and movement limits. The worker initializes joint positions to zero for each run and accumulates each admitted distinct delta once.

IsOS admits the full graph before Apply; EOS validates its four-node projection before Run. The dispatcher checks event identity, per-click bounds, and resulting joint positions before the mock adapter updates state. EOS rejects a graph with `movement.limits.max_joint_position_rad` above the 3.14 rad mock safety bound; an admitted value is passed unchanged to the worker. Stop closes admission before the child exits. The [arm control contract](../../../interfaces.md#arm_control) defines payloads, owner route, terminal results, and errors. No camera, base, C++ adapter, or physical arm is constructed for this demo.

<a id="hardware_mock--arm-observer"></a>
<a id="arm-observer"></a>

<a id="hardware_mock--eoshardwaremockarmobserverv02"></a>
## eos.hardware.mock.arm_observer@v0.2

Built-in node type in the current IsOS node library.

<a id="hardware_mock--node-behavior"></a>
### Node behavior

Reads the sole mock-arm sampler independently.

- Input: No graph input.
- Output: `observation`: `arm_monitor_state@v0.2`, in process.

<a id="hardware_mock--owner-and-lifecycle"></a>
### Owner and lifecycle

The observation source follows the active run; it cannot admit or move an actuator.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [arm-monitor](../../../interfaces.md#arm_monitor).

<a id="hardware_mock--arm-output"></a>
<a id="arm-output"></a>

<a id="hardware_mock--eoshardwaremockarmoutputv02"></a>
## eos.hardware.mock.arm_output@v0.2

Built-in node type in the current IsOS node library.

<a id="hardware_mock--node-behavior-1"></a>
### Node behavior

Passes mock-arm observation to monitoring.

- Input: `observation`: `arm_monitor_state@v0.2`, in process.
- Output: `observation`: `arm_monitor_state@v0.2`, in process.

<a id="hardware_mock--owner-and-lifecycle-1"></a>
### Owner and lifecycle

Compatibility type for saved graphs; it forwards observation and does not create another actuator route.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [arm](mock.md#hardware_mock--arm).
