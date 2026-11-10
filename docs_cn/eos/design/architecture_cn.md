# EOS 架构

[English](../../../docs/eos/design/architecture.md) · [EOS](../README_cn.md) · [外部接口](../interfaces/README_cn.md)

EOS 是独立的执行 core。其持久的 `aether-profile owner` 通过私有 Unix socket 提供设置、Run、Status、Stop、激活和监控读取。IsOS 负责保存图修订，并启动 EOS 所有者或重新连接；IsOS 重启不会停止 EOS。所有者为已准入的运行启动一个 `aether-runtime` 工作进程。

## 当前图边界

图格式 `0.2.0` 包含独立的 `isos`、`ei` 和 `eos` core。EOS 只接收 EOS 投影。控制路径为接口入口 → 内置演示技能 → 机械臂调度器 → 六关节模拟机械臂适配器。独立返回路径始于机械臂输出观测源 → 监控调度器 → 状态技能 → 接口出口。图声明六条 EOS 进程内边、一条 EI 到 EOS 的激活桥接，以及一条 EOS 到 EI 的观测桥接。EOS 在 Run 前验证全部八个节点及其连接。[机械臂控制契约](../interfaces/arm-control_cn.md)定义事件和增量准入；[机械臂监控契约](../interfaces/arm-monitor_cn.md)定义观测载荷及读取路由。

## 进程生命周期

[所有者生命周期](../interfaces/owner-health_cn.md)定义运行隔离和工作进程监管。EOS 在启动前验证图投影和运行时可执行文件的精确摘要。Status 检查工作进程；Stop 关闭准入并回收进程。已应用图始终只读。模拟适配器在一次工作进程运行中保留六个关节位置，不连接实体硬件。

## 未来架构

公开的 v2 节点 API 以及实体 ROS 2 或厂商适配器仍是独立目标。实体适配器在准入前需要测量停止延迟，并提供独立速度看门狗。可定位的夹爪需要明确的目标契约。未来的 world 融合生产者可以使用保留的 `world.snapshot` schema 结合多次检测；当前运行时不发布该资源。
