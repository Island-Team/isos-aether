# Run MuJoCo with EOS v2

[中文](../../../../docs_cn/eos/design/process/mujoco-operations_cn.md) · [Node IDL](../../interfaces/node-contributions.md) · [Hardware contract](../../interfaces/hardware.md) · [Simulator interface](../../interfaces/mujoco.md) · [R06 stop contract](../../interfaces/simulator-stop.md) · [Architecture](../architecture.md)


## Prepare and run

Build the Aether Rust executables and IsOS, then build a v2 bundle from the [simulator repository](https://github.com/Island-Team/isos-mujoco-sim). Set `MUJOCO_ROOT` to the MuJoCo 3.16.0 installation containing `lib/libmujoco.so.3.16.0`. Link the local bundle and start the IsOS workbench:

```sh
export MUJOCO_ROOT=/absolute/path/to/mujoco-3.16.0
isos init --mode dev
isos pkg link island/mujoco-sim 0.1.1 /absolute/path/to/bundle --manifest /absolute/path/to/bundle/release.json
isos doctor
isos serve
```

Build the root Cargo workspace and `ui` in this repository, then configure their output paths in IsOS before running `isos doctor`. The MuJoCo package remains external.



EOS reads the applied graph without modifying it. IsOS resolves the verified executable and scene; EOS launches MuJoCo and connects only the declared adapters. The R06 stop path fences admitted actuators before child termination and reports unknown when proof fails; the remaining live fault cases are recorded in the [stop design](../runtime/simulator-stop.md). A missing package, changed linked asset, invalid scene, or incompatible typed edge fails admission before hardware starts. The dispatcher remains the only command route for base and manipulator movement.



If a selected graph declares base or arm and the companion stop socket is still starting, EOS waits for its strict proof before reporting ready. If the socket never becomes available by the startup deadline, inspect the current run's `eos_start_failed` and backend log; no movement was admitted. A camera-only graph can be read-only without a stop generation.

## Observe the camera

The preset's EOS camera gate is the declared source for bounded `VisionFrame` output from EOS `CameraFrame` data. When an IsosUI Vision panel is available, select that active compatible gate as its preview source; the preset does not supply a UI node binding. The browser receives JPEG frames over a read-only WebSocket and displays the delivered frame rate. The initial target rate is the camera's declared rate; Vision settings can bound FPS, image size, and JPEG quality. Any later UI-owned source binding must use the typed gate output and cannot bypass EOS command admission.

Use `isos status` to compare saved and active revisions and `isos logs eos` or the IsosUI log viewer to inspect startup and camera faults. Find the current run's first `eos_ready` or `eos_start_failed` by its `run_id`; `eos_ready` confirms local EOS readiness while IsOS status confirms deployment readiness. The Rust EOS producer writes bounded JSONL to stderr. IsOS owns the deployment log store under `data/isos/log/deployments/<deployment>/`; managed MuJoCo output stays in `data/eos/log/<deployment>/capture/simulator.output.txt`, alongside private adapter-host diagnostics, and is not a shared EOS event. A correlated stop uses the same `correlation_id` on `eos_stopping`, any `eos_stop_unconfirmed`, and `eos_stopped`. Use `isos deployment stop` to stop only the graph, or `isos stop` to close the full manager and workbench.

## Managed Stop first use

The [simulator Stop design](../runtime/simulator-stop.md#verification) describes the current replay. After governed base and arm movement, Stop must provide one dual-part physical result, drain the child group, remove sockets, and publish a selected-pipe terminal receipt. A lost stop socket, child loss, or withheld response remains unknown and blocks movement until a verified new run is available.

## Benchmark isolation

The package's benchmark harness uses a separate direct-launch simulator outside the managed EOS child for seeded reset, stepping, ground-truth state and scoring. The managed strict child rejects that socket because Reset and Step would change its engine outside Dispatcher. That protocol is described in the [package repository](https://github.com/Island-Team/isos-mujoco-sim). EOS clients use graph-declared camera capture, observations and dispatcher-admitted movements; they do not receive benchmark state. Run the package's `ctest --test-dir build --output-on-failure` and the IsOS/EOS integration tests when changing the simulator or its node declarations.

**Related:** [IsOS package workflow](../../../isos/start/README.md) · [EOS hardware design](../runtime/hardware.md)
