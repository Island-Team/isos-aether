# EOS hardware runtime design

[中文](../../../../docs_cn/eos/design/runtime/hardware_cn.md) · [Hardware contract](../../interfaces/hardware.md) · [Dispatcher](dispatcher.md) · [MuJoCo](../../interfaces/mujoco.md)

## Ownership and data flow

IsOS owns saved graph revisions, verified package resources, and Apply. The current `0.2.0` graph has separate `isos`, `ei`, and `eos` cores. EOS reads its projection and constructs the declared `eos.hardware.mock.arm@v0.2` adapter once per worker run. The older `isos.eos_hardware.*@v0.2` package path is a separate compatibility design. The registered type sets placement, settings, ports, capabilities, and lifecycle; the instance provides identity and values. EOS owns live status and health, and Dispatcher owns command admission and safety.

The legacy Rust HAL carries typed camera, base, and manipulator samples and commands. The active CXX mock exercises the hardware boundary. Package adapters enter through the versioned C ABI: IsOS verifies the library and sidecar; EOS starts one isolated adapter host per selected node, which rechecks digest and ABI before device creation. MuJoCo is a separate verified process: EOS supervises it as a direct child, connects declared adapters to its owner-only Unix socket, and does not modify the graph.

## Admission and failure

EOS validates node placement, adapter family, settings, graph references, typed edges, health timing, frame IDs, movement modes, and limits before readiness. A malformed or stale sample degrades only its node. A backend cannot create a second actuator route. Every movement passes through Dispatcher with the active revision, binding, typed payload, deadline, and durable command ID.

Stop closes movement admission, requests stop for admitted actuators, records confirmed or unknown outcomes, then terminates and reaps the simulator child. A lost adapter or child degrades dependent nodes and withholds movement. Physical adapters must enforce their own write-boundary stop and watchdog behavior; mock tests do not replace hardware qualification.

## Backend selection

The current Rust runtime admits the CXX mock hardware adapter and the verified MuJoCo v2 package adapter. IsOS installs or links the package, selects its scene and nodes in graph format `0.2.0`, verifies the executable and resources, then starts EOS. EOS reads the applied graph, supervises MuJoCo as its direct child, and connects only the declared camera, base, and manipulator adapters to its owner-only Unix socket.

The registered node type controls placement, capabilities, settings, ports, and lifecycle. The instance supplies identity and selected values. IsOS checks exact package pins and typed edges before Apply; EOS validates adapter settings and health before readiness. Dispatcher admits every actuator command and owns safety and Stop. See [MuJoCo first use](../process/mujoco-operations.md) for setup → Apply → readiness → control → logs → Stop.

The current demo graph uses the registered EOS mock arm adapter and [activation contract](../../interfaces/arm-control.md). Its six-joint state is per worker run; Dispatcher admits the skill-generated delta before any update.

The owner binds a read-only `ArmObserver` instance to the graph's zero-input hardware-layer source path and passes that path to the worker. Monitor requests dispatch through this instance, which reads the sole mock-arm adapter's sampler and publishes a [typed arm observation](../../interfaces/arm-monitor.md) to the monitor dispatcher, status skill, and interface egress nodes. The worker samples at 100 Hz by default and immediately after each admitted movement. The graph may override the 10 ms sample interval and 50 ms freshness timeout through the mock arm’s Advanced settings. Each observation carries the observer path, run fence, monotonic sample time, sequence, graph-ordered joint names, and state. The owner exposes the latest sample through a read-only private route; a stale or invalid sample blocks new movement until a valid sample restores readiness.

## Asynchronous demo execution and monitoring

The EOS worker synchronously records each command reservation in a run-named journal before returning an acceptance receipt. One executor processes accepted commands in order; result lookup uses the event ID to retrieve progress or a terminal result. Repeated event IDs never cause another movement. Stop closes admission and joins the executor; unconfirmed results after worker loss remain uncertain and are not replayed automatically. The arm sampler independently maintains the latest observation and sequence, so multiple readers consume that snapshot without driving sampling. See the [arm control contract](../../interfaces/arm-control.md) for fields.
