# External simulator interface


## Boundary

| Transport | Endpoint | Caller |
| --- | --- | --- |
| Private gRPC over Unix socket | `<run>/simulator.sock` | EOS MuJoCo backend adapter |


## Contract

The `eos.simulator.v1.SimulatorService` contract is defined in [`simulator.proto`](../../../../share/eos/contracts/eos/external/simulator/simulator.proto). EOS sends bounded `Health`, `Execute`, `ReadTelemetry`, and `ReadCameraFrame` calls over the owner-only socket. Calls use finite deadlines; the client reconnects when the service returns. If the socket is missing, backend health is unavailable, commands reject, and camera reads return no frame while other EOS services continue.

MuJoCo package implementation, launch arguments, readiness, and benchmark behavior are documented in its [usage guide](../../design/process/mujoco-operations.md), design, and [interface copy](https://github.com/Island-Team/isos-mujoco-sim/blob/master/docs/mujoco/interfaces/simulator.md).
