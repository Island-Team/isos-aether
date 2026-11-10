# EOS runtime topics

[中文](../../../../docs_cn/eos/design/runtime/README_cn.md) · [Architecture](../architecture.md) · [External interfaces](../../interfaces/README.md)

The [architecture](../architecture.md) is the EOS design overview. This directory explains shared runtime mechanisms: [process selection and readiness](foundation.md), [hardware adapters](hardware.md), [Dispatcher admission](dispatcher.md), [MuJoCo child supervision](process.md), [managed Stop](simulator-stop.md), [bounded edge scheduling](scheduler.md), [selected reader features](reader-features.md), and [runtime events](events.md). Focused registered node behavior is linked from the architecture under `design/<layer>/<group>.md`.

For implementation and diagnosis, see [backend selection](hardware.md), [configuration](configure.md), [ARM64 builds](arm64.md), [package handoff](process.md), [browser result inspection](browser-control.md), [gRPC replay](grpc-gateway-guide.md), [route inspection](routed-edges.md), and [troubleshooting](troubleshooting.md). The [quick start](../../start/quick-start.md) gives the shortest operator path.
