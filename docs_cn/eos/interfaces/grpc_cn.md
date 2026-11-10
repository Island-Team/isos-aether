# EOS gRPC 与所有者 IPC

[English](../../../docs/eos/interfaces/grpc.md) · [EOS 接口](README_cn.md)

当前所有者交换使用私有 Unix 套接字。设置、Run、Status 和 Stop 参见[所有者生命周期](owner-health_cn.md)；当前数据路线参见[机械臂控制](arm-control_cn.md)和[机械臂监测](arm-monitor_cn.md)。早期公开 gRPC 接口属于[EOS 架构](../design/architecture_cn.md#未来架构)中的目标设计。

## R02 冻结的 Rust 子集

早期 R02 gRPC 子集不在当前演示图中；其归档设计见[运行时网关设计](../design/runtime/grpc-gateway_cn.md)。

## R03 执行器受治理准入目标契约

当前激活通过受运行身份约束的所有者交换和分发器进入 EOS。早期 R03 传输设计见[准入设计](../design/runtime/admission-revision_cn.md)。
