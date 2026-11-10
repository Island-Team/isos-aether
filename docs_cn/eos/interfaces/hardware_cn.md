# EOS 模拟机械臂硬件边界

[English](../../../docs/eos/interfaces/hardware.md) · [EOS 接口](README_cn.md) · [机械臂控制](arm-control_cn.md)

图 `0.2.0` 将 `eos.hardware.mock.arm@v0.2` 注册为六关节 Manipulator/Adapter 节点。其输入从 EOS 调度器接收 `arm_joint_delta@v0.2`。节点声明六个有序关节名称、`mock-arm` 适配器族、`joint_positions` 能力、健康时序、观测边界和运动限值。工作进程在每次运行开始时把关节位置初始化为零，并为每个已准入且不同的增量执行一次累积。

IsOS 在 Apply 前准入完整图；EOS 在 Run 前验证四节点投影。调度器在模拟适配器更新状态前检查事件身份、单次增量限值和结果关节位置。有效位置限值为 `movement.limits.max_joint_position_rad` 与 3.14 rad 中较小者。Stop 在子进程退出前关闭准入。[机械臂控制契约](arm-control_cn.md)定义载荷、所有者路由、终态结果和错误。此演示不构建摄像头、底盘、C++ 适配器或实体机械臂。
