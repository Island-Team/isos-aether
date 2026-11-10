# EOS 技能宿主状态

[English](../../../../../docs/eos/design/runtime/skills/host-rust.md) · [EOS 架构](../../architecture_cn.md) · [机械臂控制](../../../interfaces/arm-control_cn.md)

当前 EOS 工作进程执行一个内置演示技能：已准入的按钮激活转换为机械臂控制契约定义的固定六关节增量。该技能编译在工作进程中，不加载软件包提供的 Rust 技能。其输出只有一条执行器路径，经过机械臂调度器到模拟适配器。通用技能宿主仍不属于当前图。
