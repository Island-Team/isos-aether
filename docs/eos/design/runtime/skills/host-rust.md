# EOS skill host status

[中文](../../../../../docs_cn/eos/design/runtime/skills/host-rust_cn.md) · [EOS architecture](../../architecture.md) · [Arm control](../../../interfaces/arm-control.md)

The current EOS worker executes one built-in demo skill: an admitted button activation becomes the fixed six-joint delta defined in the arm control contract. The skill is in the worker binary; it does not load a package-provided Rust skill. Its output has only one actuator route, through the arm dispatcher to the mock adapter. General skill hosting remains outside the current graph.
