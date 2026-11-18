# skill/demo group

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces/node/design/skill</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../../docs_cn/eos/interfaces/node/design/skill/demo_cn.md)

Current EOS node types and their runtime roles are summarized below. Exact payloads and rejection rules belong to the linked owner interfaces.

## eos.skill.demo.arm_status@v0.2

Passes checked arm status toward EI. Input: `observation`: `arm_monitor_state@v0.2`, in process. Output: `observation`: `arm_monitor_state@v0.2`, in process. Forwards the checked observation to the declared egress; it does not read hardware directly. Interface: [skills](../../skill/demo.md#skill_demo).

## eos.skill.demo.joint_delta@v0.2

Turns activation into the built-in demo delta. Input: `activation`: `button_activation@v0.2`, in process. Output: `delta`: `arm_joint_delta@v0.2`, in process. Emits the registered six-joint demonstration delta; Dispatcher owns admission before movement. Interface: [skills](../../skill/demo.md#skill_demo).
