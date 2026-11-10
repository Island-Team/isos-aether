# EOS 读取器身份

[English](../../../docs/eos/interfaces/reader-features.md) · [设置](setup-v1_cn.md) · [所有者生命周期](owner-health_cn.md)

IsOS 在 Apply 前探测选定的 EOS 可执行文件。`aether-profile --capabilities` 报告源码包版本 `0.2.1`、图格式 `0.2.0`、节点类型版本 `0.2` 和四个内置 EOS 节点类型：

- `eos.interface.ingress.trigger@v0.2`
- `eos.skill.demo.joint_delta@v0.2`
- `eos.dispatcher.arm.delta@v0.2`
- `eos.hardware.mock.arm@v0.2`

`binary` 为 `aether-profile`；功能列表为 `catalog`、`setup-v2` 和 `arm-activation-owner`。目录还提供 `button_activation@v0.2` 与 `arm_joint_delta@v0.2` 数据类型。实时健康状态和激活结果来自[所有者生命周期](owner-health_cn.md)及[机械臂控制](arm-control_cn.md)。
