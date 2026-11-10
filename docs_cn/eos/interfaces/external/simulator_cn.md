# 外部模拟器接口

本页描述可选 C++ 模拟器客户端及其 `eos.simulator.v1` 协议。默认 Rust v2 路径使用 [MuJoCo v2 接口](../mujoco_cn.md)：IsOS 验证外部软件包，EOS 将仿真器作为直接子进程启动。智能体使用 EOS 公共服务，不会直接连接私有模拟器套接字。

## 边界

| 传输 | 端点 | 调用方 |
| --- | --- | --- |
| 私有 Unix socket 上的 gRPC | `<run>/simulator.sock` | EOS MuJoCo 后端适配器 |


## 契约

`eos.simulator.v1.SimulatorService` 定义于 [`simulator.proto`](../../../../share/eos/contracts/eos/external/simulator/simulator.proto)。EOS 通过仅所有者可访问的 socket 发送有界 `Health`、`Execute`、`ReadTelemetry` 和 `ReadCameraFrame` 请求。调用具有有限 deadline；服务恢复后客户端会重新连接。Socket 缺失时，后端健康状态为不可用、命令会拒绝、相机不会返回画面，但其他 EOS 服务继续运行。

MuJoCo 软件包的实现、启动参数、就绪状态和评测行为见其[使用指南](../../design/process/mujoco-operations_cn.md)、设计文档及[接口副本](https://github.com/Island-Team/isos-mujoco-sim/blob/master/docs/mujoco/interfaces/simulator.md)。
