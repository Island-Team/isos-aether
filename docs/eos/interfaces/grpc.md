# EOS gRPC and owner IPC

[中文](../../../docs_cn/eos/interfaces/grpc_cn.md) · [EOS interfaces](README.md)

The current owner exchanges use a private Unix socket. See [owner lifecycle](owner-health.md) for setup, Run, Status, and Stop; [arm control](arm-control.md) and [arm monitor](arm-monitor.md) define the current data routes. The earlier public gRPC surface is a target design in the [EOS architecture](../design/architecture.md#future-architecture).

## R02 frozen Rust subset

The former R02 gRPC subset is outside the current demo graph. Its archived design remains in the [runtime gateway design](../design/runtime/grpc-gateway.md).

## R03 governed actuator admission target contract

The current activation enters EOS through its fenced owner exchange and dispatcher. The earlier R03 transport design remains in the [admission design](../design/runtime/admission-revision.md).
