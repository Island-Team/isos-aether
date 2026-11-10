# R02 gRPC gateway cases

[中文](README_cn.md) · [Owner contract](../../../docs/eos/interfaces/grpc.md#r02-frozen-rust-subset) · [Design](../../../docs/eos/design/runtime/grpc-gateway.md)

`cases.json` specifies 35 gateway success, rejection, stop, and recovery boundaries. The checker validates case structure; the Rust unit and integration suites supply behavior evidence. The cases cover durable Dispatcher command identity, journal recovery, movement admission, health watches, controlled and emergency stop, and result bounds.

Run `python3 tests/fixtures/r02_grpc_gateway/check_cases.py` and the focused Rust tests listed in the [first-use guide](../../../docs/eos/design/runtime/grpc-gateway-guide.md). Save complete output under ignored `data/test/<run-id>/`.
