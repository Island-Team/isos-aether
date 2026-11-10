# EOS 快速入门

[English](../../../docs/eos/start/quick-start.md) · [EOS](../README_cn.md) · [架构](../design/architecture_cn.md)

初始图包含独立的 `isos`、`ei` 和 `eos` core。按钮激活经过三个 EI 节点，再经过 EOS 入口、内置技能、调度器及模拟机械臂。[机械臂控制契约](../interfaces/arm-control_cn.md)定义控制节点和六关节命令结果。[机械臂监控契约](../interfaces/arm-monitor_cn.md)定义四节点独立观测返回路径。

## 构建和设置

在仓库根目录运行 `cargo build -p aether-runtime --bins` 和 `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`。IsOS 使用 `aether-profile` 所有者 socket 设置，并以同目录的 `aether-runtime` 二进制文件启动工作进程。从 [IsOS 初始图](../../../crates/isos/templates/generic-graph.toml)开始，确认所有者已配置且图通过验证。

## Apply、Run 和控制

Apply 保存带修订版本的图、验证仅含 EOS 的投影并发布渲染后的图。Run 启动 EOS 所有者或重新连接，并启动机械臂工作进程。运行 `isos status --json` 查看 EOS 进程和模拟硬件就绪状态。在 IsosUI 中点击按钮节点上的 **触发**。EOS 先返回已接受的事件 ID 和命令 ID；IsosUI 随后读取关联结果，直到得到六个以弧度计的关节位置或终态故障。EI 独立于 UI 读取者刷新最新的 EOS 机械臂观测；返回路径将新鲜度、健康状态和关节位置传至 IsosUI。第二次使用不同事件 ID 的点击累积相同的小增量；重复同一事件 ID 不会再次运动。读取当前运行的 EOS 日志。IsOS 重启后重新连接，不停止 EOS 所有者。

## Stop 和恢复

运行 `isos stop`，再运行 `isos status --json`。Stop 要求匹配的运行 ID 和修订版本；EOS 在确认子进程退出后报告成功。不确定结果的点击不得自动以新事件 ID 重发。如果子进程已经失败，Stop 仍会回收它。仅在先前的所有者运行停止后再次 Run。
