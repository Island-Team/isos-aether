# dispatcher/arm group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/dispatcher</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Base primitive binding node](base.md)
  - [Shared control dispatcher node](control.md)
  - [V2 dispatcher tree](dispatcher.md)
  - [Manipulator primitive binding node](manipulator.md)
  - [Routed connection scheduling](scheduler.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/dispatcher/arm_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.dispatcher.arm.delta@v0.2

Admits the six-joint demo command before mock-arm movement. Input: `delta`: `arm_joint_delta@v0.2`, in process. Output: `delta`: `arm_joint_delta@v0.2`, in process. Checks run identity, health and movement limits before the declared mock adapter receives a command. Interface: [arm-control](../../interfaces.md#arm_control).

## eos.dispatcher.arm.monitor@v0.2

Checks the mock-arm return observation. Input: `observation`: `arm_monitor_state@v0.2`, in process. Output: `observation`: `arm_monitor_state@v0.2`, in process. Rejects stale or invalid samples; the monitoring path is independent of control-result reads. Interface: [arm-monitor](../../interfaces.md#arm_monitor).
