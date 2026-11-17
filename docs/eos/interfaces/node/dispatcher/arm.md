<a id="dispatcher_arm"></a>
<a id="dispatcher_arm--dispatcherarm-group"></a>
# dispatcher/arm group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/dispatcher</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/dispatcher/arm_cn.md#dispatcher_arm_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [delta](arm.md#dispatcher_arm--delta)
- [monitor](arm.md#dispatcher_arm--monitor)

<a id="dispatcher_arm--delta"></a>
<a id="delta"></a>

<a id="dispatcher_arm--eosdispatcherarmdeltav02"></a>
## eos.dispatcher.arm.delta@v0.2

Built-in node type in the current IsOS node library.

<a id="dispatcher_arm--node-behavior"></a>
### Node behavior

Admits the six-joint demo command before mock-arm movement.

- Input: `delta`: `arm_joint_delta@v0.2`, in process.
- Output: `delta`: `arm_joint_delta@v0.2`, in process.

<a id="dispatcher_arm--owner-and-lifecycle"></a>
### Owner and lifecycle

Checks run identity, health and movement limits before the declared mock adapter receives a command.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [arm-control](../../../interfaces.md#arm_control).

<a id="dispatcher_arm--monitor"></a>
<a id="monitor"></a>

<a id="dispatcher_arm--eosdispatcherarmmonitorv02"></a>
## eos.dispatcher.arm.monitor@v0.2

Built-in node type in the current IsOS node library.

<a id="dispatcher_arm--node-behavior-1"></a>
### Node behavior

Checks the mock-arm return observation.

- Input: `observation`: `arm_monitor_state@v0.2`, in process.
- Output: `observation`: `arm_monitor_state@v0.2`, in process.

<a id="dispatcher_arm--owner-and-lifecycle-1"></a>
### Owner and lifecycle

Rejects stale or invalid samples; the monitoring path is independent of control-result reads.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [arm-monitor](../../../interfaces.md#arm_monitor).
