# EOS 技能宿主验证

[English](../../../../../docs/eos/design/runtime/skills/host-rust-guide.md) · [状态](host-rust_cn.md)

运行 `cargo test -p aether-runtime --bin aether-runtime --test arm_activation`，验证内置技能以及真实所有者与工作进程 socket 边界。回放覆盖就绪、两次不同激活、重复事件回放、过时身份拒绝及 Stop。它还验证工作进程接受第二个已准入的按钮 ID，并拒绝无效来源。完整输出保存在被忽略的 `data/test/20261109-eos-arm-demo/`。
