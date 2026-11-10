# ARM64 上的 EOS

[English](../../../../docs/eos/design/runtime/arm64.md) · [构建与运行](../../start/quick-start_cn.md) · [MuJoCo](../process/mujoco-operations_cn.md) · [架构](../architecture_cn.md)

在目标主机上使用受支持的 Rust 工具链和 CXX 硬件适配器边界所需的平台 C/C++ 编译器构建 Rust 工作区。运行[构建指南](../../start/quick-start_cn.md)中的工作区检查。运行时可执行文件为 `aether-runtime`、`aether-profile`、`aether-http-bridge`、`aether-config-server`；IsOS 将它们作为私有归属程序启动。

使用 `isos tui` 检查和控制已应用的部署。

实体适配器在部署前须通过类型化样本、健康、命令边界、取消及 Stop 的硬件集成测试。
