# EOS roadmap

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EOS core](README.md)
  - [EOS architecture](architecture.md)
- Subfolders:
  - [dispatcher/](dispatcher/)
  - [gateway/](gateway/)
  - [hardware/](hardware/)
  - [process/](process/)
  - [skills/](skills/)

</details>
<!-- doc-navigation:end -->

[Current architecture](architecture.md) · [中文](../../../docs_cn/eos/core/roadmap_cn.md)

## Target architecture (not implemented)

**Target design — not implemented.** EOS should be a HAL-like physical substrate for multiple robot embodiments: typed sensing and actuation, semantic perception with provenance, durable physical evidence, bounded dataset export, device recovery, and independently enforceable emergency stop. It should expose stable capabilities to EI while keeping learned task meaning and planning outside the hardware safety loop.

A public v2 node API and physical ROS 2 or vendor adapters require separate interface contracts and qualification. A position-capable gripper needs an explicit target contract. The reserved `world.snapshot` schema may support a future fusion producer; the current runtime does not publish it.

### Cross-backend graph execution

**Target design — partly implemented.** EOS uses one run-scoped execution plan for typed observation and software links among the mock-arm worker and contributed native nodes. It keeps graph format `0.2.1`, the authored node and port identities, and the existing Dispatcher actuator paths. It does not admit arbitrary cross-package actuator commands or replace the mock-arm safety chain. The current [setup mode guard](../../../crates/aether-system/src/bin/aether_profile/setup.rs) admits a complete mixed demo/native graph with supported routes. IsOS supports multiple selected v2 package services with node-keyed `endpoints.package_services`; its separate legacy v1 EOS process-node handoff remains limited to one process. The v2 service ABI has no typed input callback, so a v2 service can produce an observation for a v1 software consumer but cannot consume a software link in this delivery.

The implemented owner verifies every selected v2 service handoff at Run by deriving its generation from the run identity and package instance and checking the framed IsOS health response for its selected device. Status and health repeat these checks with one 500 ms aggregate deadline and deduplicate shared service devices; a lost service or failed observation route latches failed readiness until Stop. The owner keeps the selected service socket behind its private worker adapter and does not supervise IsOS's service process as its own child.

At setup, EOS compiles the verified EOS projection and accepted `port_routes` into a candidate route plan bound to the saved revision; Run recompiles it from verified bytes and binds it to the admitted run ID. Every node is assigned to exactly one worker according to its registered implementation and selected verified package; every delivered edge remains an exact authored source node/port → target node/port link. A route record identifies the source and target worker, registered data type, data class, transport, and bounded delivery policy. A worker can publish only a declared output; the owner validates its envelope and delivers only to declared inputs. No worker may add edges, infer a route from similar type names, mutate the applied graph, or send an actuator write around Dispatcher. The existing EOS [edge contract](../interfaces/node/hardware/mock.md#hardware_mock--arm) and [node-contribution contract](../../isos/interfaces/server/eos.md#node_contributions) remain authoritative for current registrations; the implementation slice must extend the owning interfaces before changing their wire fields.

Setup validation checks static package references, implementation availability, exact type IDs and port transports, supported bridge adapters, complete demo safety chains, and the absence of a second actuator path; it returns an edge- or node-addressed diagnostic before Apply. Run recompiles the same routes from the saved revision and freshly verified package bytes, then checks concrete run-bound service endpoints, socket paths, generations, health, and run identity before EOS readiness. It never silently drops an unsupported link. The registered source output `frequency_hz` declares its production frequency. The destination input may request a `desired_rate_hz` range; EOS derives the edge `frequency_hz` cap from its maximum and rejects authored edge frequency. The admitted edge `priority` and derived cap use the existing [bounded scheduler](dispatcher/scheduler.md) where applicable; cross-worker delivery also requires explicit per-route and aggregate byte limits, finite queue depth, cancellation deadlines, and observable loss or backpressure counters. Observation or software overload must not block command admission or Stop.

One persistent EOS owner holds the plan and run fence, supervises its direct mock and native workers, and routes typed observation/software outputs between them and selected services. IsOS supervises v2 package service children behind `endpoints.package_services`; EOS receives their existing node-keyed, run-bound endpoints and does not claim them as direct children. The [IsOS service design](../../isos/core/service.md) and EOS [node-contribution contract](../../isos/interfaces/server/eos.md#node_contributions) describe that current handoff. Legacy v1 EOS process-node children remain under the existing single-process owner path; this target does not expand that path or its [package-process contract](../../isos/interfaces/server/eos.md#package_process).

Worker exchange must bind each input/output envelope to the run ID, saved revision, canonical node path, port, exact registered data type, and bounded payload. The owner rejects stale or foreign publications before enqueueing and fences queued and in-flight work when the run ends. Local in-process delivery may avoid serialization only after the same identity and type checks. A package service endpoint is a source or sink only when its registered lifecycle and route adapter explicitly support that direction; endpoint presence alone grants no graph route. The implementation must document the final private exchange fields and error codes under EOS interfaces, while keeping the owner API envelope and graph format unchanged.

Readiness requires every selected EOS worker and child to prove the exact run and revision and pass its health probe; required package-service endpoints must match IsOS's run-fenced service receipts and pass their health checks. Partial startup rolls back the new run without replacing a healthy previous run. Loss of a required producer, consumer, worker, or service degrades dependent routes and inhibits affected commands; it never synthesizes success or silently replays an uncertain command. Stop first seals graph and command admission, invokes Dispatcher stop for admitted actuators and records its run-bound adapter proof, cancels bounded observation/software deliveries, then stops and reaps EOS-owned workers and any legacy process child. IsOS then terminates and reaps its v2 package-service children and verifies their exit. The EOS owner reports confirmed Stop only when its actuator and direct-child proofs are complete; IsOS reports the overall run stopped only after EOS Stop proof and its own service-child exit checks agree. Uncertain proof remains visible and retryable.

Acceptance covers demo-only and package-only regressions, a mixed mock-arm plus camera/vision graph, and an observation/software edge from a selected v2 service producer to a separately pinned v1 software consumer package. A v2 consumer requires a future typed input ABI. Unit and integration cases reject wrong type or transport, missing implementation, undeclared edge, stale run or revision, unsafe command route, changed package bytes, queue saturation, failed readiness, worker or service loss, and incomplete Stop proof. A real-boundary replay must show setup → Apply → readiness → control and observation → logs → Stop, with run-fenced diagnostics and no second actuator release.

### HAL and device lifecycle

A registered adapter profile declares device identity, embodiment, frames, units, calibration and firmware versions, capabilities, timing, limits, health probes, write-boundary Stop, and recovery procedure. A simulator or physical driver implements the same typed port behavior, while each has an explicit provenance class. EOS binds only verified resources from IsOS and revalidates its projection. A backend supplies samples and executes admitted commands; it cannot create routes, change graph settings, or claim a capability absent from its profile. Dispatcher remains the sole actuator admission path, and a physical controller retains an independent watchdog and stop latch.

Separate hot-path sampling and control from slower perception and persistence. Every sample has source, device time, monotonic host time, sequence, frame, calibration version, quality, and bounded age. Invalid or stale samples degrade the affected node and dependent capabilities. Recovery requires a fresh probe and explicit state transition; losing a sensor cannot silently reuse its last good value. Hardware reconnection never repeats an uncertain command.

### Perception catalog and semantic evidence

Publish a catalog of observation producers and semantic outputs: object or human detections, contact onset, slip evidence, obstruction, joint limit, motion completion, and health. Each entry names its producer, model or rule version, supported frame, rate, uncertainty, retention, and permitted consumers. A semantic event points back to bounded source evidence and states unknown when a cause is not supported. EOS performs physical signal interpretation and source provenance; EI decides what those events mean for a task. Raw media or embeddings remain behind a governed read boundary rather than being placed in every event or log. World fusion, where needed, must declare its temporal and source assumptions and publish a new derived resource instead of overwriting detections.

### Memory writes and dataset generation

EOS writes append-only, run-fenced physical episodes: sampled state references, admitted proposal and executed command IDs, contact and safety interventions, device faults, terminal outcomes, clock alignment, and calibration and adapter versions. Its physical evidence store is distinct from EI's cognitive memory and IsOS's deployment logs. Writes use bounded queues and an explicit loss or backpressure policy that cannot stall the servo or Stop path. Stop flushes what can be proven and marks gaps or uncertain outcomes; restart never invents missing samples.

A dataset exporter selects consented or otherwise authorized episodes, resolves their evidence references, checks schema and time alignment, and records a manifest with source digests, label provenance, split assignment, and transformation versions. Preserve failures and interrupted attempts. Simulator privileged truth may supply training labels and scoring, but never an inference input or an unmarked real-world observation. Separate simulation and physical results and record teacher time, human corrections, model calls, and compute so a learning claim can be audited.

### Safety, e-stop, and recovery

Use layered admission: profile and graph checks at setup, health and freshness checks at command time, a latched emergency stop at Dispatcher, and independent stop/watchdog enforcement in each physical adapter. E-stop preempts queues and learned skills, persists a visible reason, and blocks new commands. A reset requires an operator-authorized procedure, physical cause removal, fresh probes, and revalidation of limits; a model cannot clear the latch. On adapter or worker loss, deadline miss, sensor disagreement, or uncertain post-release result, hold or stop the affected actuators and expose the exact degraded state. Measure stop latency and watchdog expiry on each physical device before cutover; a mock proof is insufficient.

### Method and acceptance gates

1. Define one typed HAL profile and semantic observation contract under EOS interfaces; test rejected units, stale frames, unsupported capability, and mismatched calibration before admitting a physical adapter.
2. Replay one simulated episode through Dispatcher and the evidence store, including a fault, Stop, restart, and dataset export. Verify exact command and sample provenance and explicit gaps.
3. Compare semantic detectors with audited labels under noise, delay, occlusion, and missing sensors. Report false positives, false negatives, latency, and uncertainty rather than treating model output as truth.
4. Qualify a physical adapter with real measured stop latency, independent watchdog, blocked write, fault recovery, and reset tests. Re-run setup → Apply → readiness → control → logs → Stop using the same typed ports as simulation.



### Earlier service and resource targets

- **Primitive calls:** The earlier primitive catalog and payload examples are target designs. The current graph admits `button_activation@v0.2` and `arm_joint_delta@v0.2` through the EOS owner and dispatcher. See [arm control](../interfaces.md#arm_control), [arm monitor](../interfaces.md#arm_monitor), and [owner lifecycle](../../isos/interfaces/server/eos.md#owner_health) for the implemented contracts. Future direction belongs in the [EOS architecture](roadmap.md#target-architecture-not-implemented).
- **Body discovery:** The earlier general device-discovery model is a target design. The current mock arm exposes its declared graph node, six joint positions, health, and read-only observation. See [arm control](../interfaces.md#arm_control), [arm monitor](../interfaces.md#arm_monitor), and [owner lifecycle](../../isos/interfaces/server/eos.md#owner_health) for the implemented contracts. Future direction belongs in the [EOS architecture](roadmap.md#target-architecture-not-implemented).
- **CapabilityService:** The earlier public native gRPC capability catalog is a target design. The current graph admits only its declared button, mock arm, and monitor routes. See [arm control](../interfaces.md#arm_control), [arm monitor](../interfaces.md#arm_monitor), and [owner lifecycle](../../isos/interfaces/server/eos.md#owner_health) for the implemented contracts. Future direction belongs in the [EOS architecture](roadmap.md#target-architecture-not-implemented).
- **SkillService:** The earlier public skill service is a target design. The current EOS demo skill is an in-process data-pass node on the declared graph route. See [arm control](../interfaces.md#arm_control), [arm monitor](../interfaces.md#arm_monitor), and [owner lifecycle](../../isos/interfaces/server/eos.md#owner_health) for the implemented contracts. Future direction belongs in the [EOS architecture](roadmap.md#target-architecture-not-implemented).
- **World resources:** The earlier world-resource and fusion schemas are target designs. The current graph does not publish a world resource; its read-only return payload is `arm_monitor_state@v0.2`. See [arm control](../interfaces.md#arm_control), [arm monitor](../interfaces.md#arm_monitor), and [owner lifecycle](../../isos/interfaces/server/eos.md#owner_health) for the implemented contracts. Future direction belongs in the [EOS architecture](roadmap.md#target-architecture-not-implemented).

## Design verification

On 2026-11-12, the cross-backend target-design slice ran `python3 scripts/check_doc_links.py --all`, `python3 crates/isos/tools/check_isos_docs.py`, an English/Chinese architecture heading-level parity check, and `git diff --check`. Complete output is under ignored `data/test/20261112-eos-cross-backend-design/`. This verifies documentation only; mixed execution and cross-backend Stop proof remain implementation work.

On 2026-11-11, the documentation-only slice ran the bilingual link audit, IsOS documentation checker, `git diff --check HEAD`, and architecture heading-level parity check. Commands and complete outputs are recorded in [IsOS verification](../../isos/core/server.md); no EOS runtime or physical safety qualification is claimed by this design.
