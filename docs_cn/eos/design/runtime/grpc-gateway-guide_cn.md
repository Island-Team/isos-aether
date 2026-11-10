# EOS gRPC 首次使用与验证

[English](../../../../docs/eos/design/runtime/grpc-gateway-guide.md) · [所有者契约](../../interfaces/grpc_cn.md#r02-冻结的-rust-子集) · [设计](grpc-gateway_cn.md) · [机器用例](../../../../tests/fixtures/r02_grpc_gateway/README_cn.md) · [架构](../architecture_cn.md)

通过 IsOS 使用选定图、已验证 profile、journal、私有控制 socket、直接管理的 MuJoCo 子进程和 loopback gRPC listener 启动 `aether-runtime`。缺少必要适配器或 journal 损坏会在就绪前使启动失败。运行时就绪前 health 保持 `NOT_SERVING`；关闭期间在准入新写入前先变为 `NOT_SERVING`。

listener 读取有界保留的健康与状态快照；HAL 探测不在 tonic 请求线程执行。快照缺失、失败或过期时，对应设备不可用。新鲜资源和帧响应包含样本年龄元数据。十六个运动许可与受控及紧急 Stop 的保留许可分开，排队运动无法占满 Stop 路线。

待处理的 `WatchOperation` 发送当前状态；调用方截止时间前出现终态时再发送终态。到期时返回 `DEADLINE_EXCEEDED`；客户端用同一个 operation ID 调用 `GetOperation`。Submit 响应丢失、watch 取消或到期都不授权再次物理释放。

首次使用时测试健康发现和 mock 操作、过期与阻塞的 HAL 探测、适配器缺失、listener 和子进程故障、变化的 operation 签名、运动容量占满时的 Stop 以及关闭。运行格式化、严格 Clippy、串行工作区测试、构建、[机器用例检查](../../../../tests/fixtures/r02_grpc_gateway/README_cn.md)和双语链接检查。把命令、修订、环境与完整输出保存在 `data/test/<run-id>/`。
