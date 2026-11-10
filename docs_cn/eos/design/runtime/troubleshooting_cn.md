# 调试 EOS

[English](../../../../docs/eos/design/runtime/troubleshooting.md) · [所有者生命周期](../../interfaces/owner-health_cn.md) · [架构](../architecture_cn.md)

对于当前图 `0.2.0`，先查看 `./isos.sh status --json` 和活动图修订。EOS 有一个持久的 `aether-profile` 所有者及一个 `aether-runtime` 机械臂工作进程。IsOS 读取 `data/eos/log/<deployment>/eos.jsonl` 更新日志面板；独立运行的 EOS 使用 `data/eos/log/local/eos.jsonl`。绝对路径 `EOS_LOG_DIR` 可选择其他目录。所有者接口传递 Run、Status、激活和 Stop 结果；日志单独保存。

若 **触发** 失败，请比较 UI 事件 ID 和运行修订版本与所有者状态。过时的运行或修订版本在移动前被拒绝。`invalid_eos_graph` 表示投影缺少四节点链或声明的连接。`joint_limit` 表示下一增量会超过配置的模拟机械臂限值或 ±3.14 rad。`outcome_unknown` 表示工作进程可能已执行动作；不得以新的事件 ID 自动重试。成功的激活结果报告六个模拟关节位置。

每条 JSONL 记录包含 UTC `timestamp`、小写 `level`、`source` 和 `message`。系统来源包括 `eos.aether_profile.owner@aether-v0.2.1` 与 `eos.aether_runtime.worker@aether-v0.2.1`；模拟节点使用 `eos.hardware.mock.arm@aether-v0.2.1`。工作进程不可用时，所有者在观察到后只报告一次。应比较同一次运行的日志和状态；单条日志不能证明就绪。

Stop 关闭准入并等待工作进程退出。未确认的 Stop 仍表现为失败的所有者回复。模拟机械臂不连接实体硬件。将可复现的测试输出保存在忽略的 `data/test/<run-id>/`。
