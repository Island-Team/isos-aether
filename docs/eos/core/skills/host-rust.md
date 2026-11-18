# EOS skill host status

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/skills</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/skills/host-rust_cn.md) · [EOS architecture](../architecture.md) · [Arm control](../../interfaces.md#arm_control)

The current EOS worker executes one built-in demo skill: an admitted button activation becomes the fixed six-joint delta defined in the arm control contract. The skill is in the worker binary; it does not load a package-provided Rust skill. Its output has only one actuator route, through the arm dispatcher to the mock adapter. General skill hosting remains outside the current graph.

## EOS skill host verification

Run `cargo test -p aether-runtime --bin aether-runtime --test arm_activation` to verify the built-in skill and real owner/worker socket boundary. The replay covers readiness, two distinct activations, duplicate replay, stale identity rejection, and Stop. It also verifies that the worker accepts a second admitted button ID while rejecting a malformed source. Complete output is saved under ignored `data/test/20261109-eos-arm-demo/`.
