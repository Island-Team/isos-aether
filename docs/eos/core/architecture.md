# EOS architecture

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EOS core](README.md)
  - [EOS roadmap](roadmap.md)
- Subfolders:
  - [dispatcher/](dispatcher/)
  - [gateway/](gateway/)
  - [hardware/](hardware/)
  - [process/](process/)
  - [skills/](skills/)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/eos/core/architecture_cn.md) · [EOS](../README.md) · [Interfaces](../interfaces.md#readme)

EOS is the execution and physical safety core. It reads its applied IsOS graph projection, constructs only declared adapters, admits commands through Dispatcher, publishes observed health, and supervises its direct worker children. Its persistent owner serves Run, Status, Stop, activation, and monitor reads over a private Unix socket. The [hardware](../interfaces/node/hardware/mock.md#hardware_mock--arm), [arm control](../interfaces.md#arm_control), [arm monitor](../interfaces.md#arm_monitor), and [owner health](../../isos/interfaces/server/eos.md#owner_health) pages own current fields and behavior.

## Current runtime boundary

In graph `0.2.1`, the control chain is interface ingress → demo skill → arm Dispatcher → six-joint mock adapter. A separate read-only chain starts at the arm observation source and returns through monitor Dispatcher, status skill, and interface egress. EOS validates its eight nodes and links before Run. The owner verifies the projection and exact worker digest, starts the required worker or workers, checks them on Status, and closes admission and reaps them on Stop. The applied graph is read-only. The mock adapter has no physical connection.

Dispatcher checks command identity, deadlines, capability, health, sample freshness, limits, and safety before a physical write. It durably reserves commands and does not automatically replay an uncertain release. Local observation faults withhold movement until a valid fresh sample recovers it. Stop closes admission and stops admitted actuators before terminating selected direct children. The [hardware design](hardware/hardware.md) and [dispatcher design](dispatcher/dispatcher.md) describe these current mechanisms.

The owner can also run the complete mock chain alongside selected native and package nodes. Setup admits supported typed routes; Run binds them to the saved revision and run ID. The owner supervises its mock and native workers, checks selected v2 service health through IsOS, and delivers admitted observation outputs between workers. The current v2 service ABI can produce observations but has no typed input callback. See the [owner exchange](../../isos/interfaces/server/eos.md#node_contributions) and [cross-backend roadmap](roadmap.md#cross-backend-graph-execution) for the remaining qualification gates.

Future direction: [roadmap](roadmap.md).

## Rust crate ownership

`aether-system` owns the EOS owner and worker runtime, including Dispatcher admission and private process I/O. `aether-node` owns built-in node implementations under `src/<layer>/<group>/`; the `aether-runtime` library keeps the installed native ABI artifact used by the current worker. `aether-node-profile` owns graph parsing, type registration, and edge validation, so setup and runtime read the same node contracts. `aether-hal` defines the adapter boundary, and `aether-lib` holds small shared time, event, and bounded-I/O primitives. The server and runtime depend on these libraries without moving graph or wire identities between owners.
