# EOS gRPC first use and verification

[中文](../../../../docs_cn/eos/design/runtime/grpc-gateway-guide_cn.md) · [Owner contract](../../interfaces/grpc.md#r02-frozen-rust-subset) · [Design](grpc-gateway.md) · [Machine cases](../../../../tests/fixtures/r02_grpc_gateway/README.md) · [Architecture](../architecture.md)

Start `aether-runtime` through IsOS with the selected graph, verified profile, journal, private control socket, direct MuJoCo child, and loopback gRPC listener. Missing required adapters or a corrupt journal fail startup before readiness. Health remains `NOT_SERVING` until the runtime is ready and becomes `NOT_SERVING` before new writes are admitted during shutdown.

The listener reads bounded retained health and status snapshots; HAL probes do not run on tonic request threads. A missing, failed, or stale snapshot makes the corresponding device unavailable. Fresh resource and frame responses include sample age metadata. Sixteen movement permits are separate from reserved controlled and emergency Stop permits, so queued movement cannot consume the Stop route.

A pending `WatchOperation` sends current state and a terminal update if one arrives before the caller deadline. At the deadline it returns `DEADLINE_EXCEEDED`; clients use the same operation ID with `GetOperation`. A lost Submit response, cancelled watch, or deadline does not authorize another physical release.

For first use, test healthy discovery and a mock operation, stale and blocked HAL probes, missing adapters, listener and child failure, changed operation signatures, Stop under full movement capacity, and shutdown. Run formatting, strict Clippy, serial workspace tests, build, the [machine case checker](../../../../tests/fixtures/r02_grpc_gateway/README.md), and the bilingual link audit. Save commands, revision, environment, and complete output under `data/test/<run-id>/`.
