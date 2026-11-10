# EOS runtime foundation

[中文](../../../../docs_cn/eos/design/runtime/foundation_cn.md) · [Node interface](../../interfaces/hardware.md) · [Process design](process.md) · Dispatcher design

This page describes the current Rust foundation and records dated verification. The later design section includes implemented graph, readiness, routing and stop behavior alongside remaining physical-adapter work; its contracts are owned by [hardware](../../interfaces/hardware.md), [node contributions](../../interfaces/node-contributions.md), [control](../../interfaces/control.md), and [gRPC](../../interfaces/grpc.md).

## Boundaries and failure rules

- A node type owns schema and placement. A node instance owns identity and settings. Typed graph edges declare data flow; the dispatcher alone admits actuator commands and writes durable results.
- IsOS starts EOS, which starts its verified simulator child and waits for the simulator Unix socket before reporting EOS readiness. If the simulator fails, EOS stops; IsOS can restart the EOS component according to its bounded policy.
- EOS acquires the journal before binding its control socket. On restart it may unlink a stale socket only when the path is a Unix socket and no live peer accepts a connection. An uncertain released command is never automatically replayed.
- EOS sends stop to all admitted actuators on graceful shutdown. The simulator enforces a body velocity lease. A measured physical controller stop bound remains necessary before hardware deployment.
- The HTTP bridge and read-only config server bind loopback. The bridge has no adapter access; it forwards bounded requests to EOS.

## 0.x runtime design and remaining work




During operation, the runtime owns health and typed observations; the dispatcher owns deadline, bounds, deduplication, journaling, safety, and the only actuator route. A lost simulator socket or adapter host marks affected nodes unavailable. Released commands with uncertain outcomes are never retried automatically. The R06 owner path now enforces shutdown order under blocked work: stop intake and queued work, obtain independent base/arm stop acknowledgment or record uncertainty, then terminate and reap MuJoCo before closing listeners and the journal. Managed movement primitives are available only when both MuJoCo sockets prove `strict-stop-v1` for the same generation; R05 readiness alone does not prove a later physical safe stop.

R05 proves strict identity and same-child retransmission handling. R06 owner implementation supplies stop admission fencing, a preemptive bounded simulator lane, independent base/arm acknowledgment or durable unknown, and termination/reap after the stop attempt. Exact-head and final-negative cross-owner replays cover selected paths; full R06 qualification still needs the remaining live faults recorded in the [stop design](simulator-stop.md).

The internal adapter registry contains a MuJoCo factory and its socket client. IsOS still installs and pins the external `island/mujoco-sim@0.1.1` executable and assets; EOS receives verified paths and does not install packages or edit the graph. Physical ROS/vendor drivers are later narrow adapters, with measured stop behavior required before admission..
