<a id="skill_demo"></a>
<a id="skill_demo--skilldemo-group"></a>
# skill/demo group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/skill</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/eos/interfaces/node/skill/demo_cn.md#skill_demo_cn)

This group contains the following EOS node types. The node contracts follow on this page.

- [arm_status](demo.md#skill_demo--arm-status)
- [joint_delta](demo.md#skill_demo--joint-delta)

<a id="skill_demo--arm-status"></a>
<a id="arm-status"></a>

<a id="skill_demo--eosskilldemoarmstatusv02"></a>
## eos.skill.demo.arm_status@v0.2

Built-in node type in the current IsOS node library.

<a id="skill_demo--node-behavior"></a>
### Node behavior

Passes checked arm status toward EI.

- Input: `observation`: `arm_monitor_state@v0.2`, in process.
- Output: `observation`: `arm_monitor_state@v0.2`, in process.

<a id="skill_demo--owner-and-lifecycle"></a>
### Owner and lifecycle

Forwards the checked observation to the declared egress; it does not read hardware directly.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [arm monitor](../../../interfaces.md#arm_monitor).

<a id="skill_demo--joint-delta"></a>
<a id="joint-delta"></a>

<a id="skill_demo--eosskilldemojointdeltav02"></a>
## eos.skill.demo.joint_delta@v0.2

Built-in node type in the current IsOS node library.

<a id="skill_demo--node-behavior-1"></a>
### Node behavior

Turns activation into the built-in demo delta.

- Input: `activation`: `button_activation@v0.2`, in process.
- Output: `delta`: `arm_joint_delta@v0.2`, in process.

<a id="skill_demo--owner-and-lifecycle-1"></a>
### Owner and lifecycle

Emits the registered six-joint demonstration delta; Dispatcher owns admission before movement.

The linked EOS owner contract defines exact payload fields and rejection rules.

See [arm control](../../../interfaces.md#arm_control).
