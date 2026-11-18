# EOS 技能宿主状态

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core/skills</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/core/skills/host-rust.md) · [EOS 架构](../architecture_cn.md) · [机械臂控制](../../interfaces_cn.md#arm_control_cn)

当前 EOS 工作进程执行一个内置演示技能：已准入的按钮激活转换为机械臂控制契约定义的固定六关节增量。该技能编译在工作进程中，不加载软件包提供的 Rust 技能。其输出只有一条执行器路径，经过机械臂调度器到模拟适配器。通用技能宿主仍不属于当前图。

## EOS 技能宿主验证

运行 `cargo test -p aether-runtime --bin aether-runtime --test arm_activation`，验证内置技能以及真实所有者与工作进程 socket 边界。回放覆盖就绪、两次不同激活、重复事件回放、过时身份拒绝及 Stop。它还验证工作进程接受第二个已准入的按钮 ID，并拒绝无效来源。完整输出保存在被忽略的 `data/test/20261109-eos-arm-demo/`。
