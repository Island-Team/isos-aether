# EOS loopback HTTP bridge

[中文](../../../../docs_cn/eos/interfaces/external/http_cn.md) · [Control admission](../control.md) · [Vision](../vision.md)

`aether-http-bridge` is the Rust client-facing process. It binds loopback, validates Host and Origin, bounds request bodies and private IPC, and forwards only admitted public operations to the selected `aether-runtime` Unix socket. It does not release an actuator directly. IsOS selects and supervises the bridge as part of the applied deployment.

## Control route

`POST /api/eos/v2/control` accepts bounded JSON for admitted `primitive_typed`, authorized result lookup, and read-only status, observation, and health. The [control contract](../control.md) defines request fields, grant handling, deadlines, errors, and Dispatcher reservation. Browser callers cannot supply a private owner grant or invoke raw dispatch and shutdown operations.

## Vision route

The read-only Vision WebSocket streams JPEG frames from a selected active camera gate. The bridge verifies the owner attestation, run and graph revision on each bounded poll. It does not create a camera route or change the applied graph. See the [vision contract](../vision.md).

## Failure and Stop

Invalid Host or Origin, malformed JSON, excessive payload, missing grant, stale revision, unavailable runtime, and failed owner attestation produce bounded errors without exposing private paths or tokens. IsOS owns deployment Stop. EOS closes command admission and stops admitted actuators before terminating its direct simulator child.
