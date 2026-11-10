# R02 gRPC 网关用例

[English](README.md) · [所有者契约](../../../docs_cn/eos/interfaces/grpc_cn.md#r02-冻结的-rust-子集) · [设计](../../../docs_cn/eos/design/runtime/grpc-gateway_cn.md)

`cases.json` 定义 35 条网关成功、拒绝、停止和恢复边界。检查器验证用例结构；Rust 单元和集成测试提供行为证据。用例覆盖 Dispatcher 命令的持久身份、journal 恢复、运动准入、健康 watch、受控和紧急停止以及结果上限。

运行 `python3 tests/fixtures/r02_grpc_gateway/check_cases.py` 和[首次使用指南](../../../docs_cn/eos/design/runtime/grpc-gateway-guide_cn.md)所列的聚焦 Rust 测试。完整输出保存在忽略的 `data/test/<run-id>/` 下。
