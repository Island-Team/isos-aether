# IsOS roadmap

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS core](README.md)
  - [IsOS architecture](architecture.md)
  - [IsOS CLI](cli.md)
  - [IsOS configuration](config.md)
  - [Local core logs](logger.md)
  - [IsOS packages](pkg.md)
  - [IsOS server](server.md)
  - [IslandOS services](service.md)
- Subfolders:
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[Current architecture](architecture.md) · [中文](../../../docs_cn/isos/core/roadmap_cn.md)

## Target architecture (not implemented)

**Target design — not implemented.** The aim is a portable node runtime that can admit third-party capabilities without hiding control authority, while preserving enough evidence to replay and diagnose an experiment. The graph remains a declarative, immutable snapshot for one run. A registered type supplies the owning core, implementation identity, exact version, ports, settings schema, capability requirements, resource limits, and lifecycle. The instance selects values and connections; owner runtime state, health, and measured data stay outside graph bytes. No package or backend can declare a second actuator route around EOS Dispatcher.

### One typed plan from graph to execution

Compile the validated graph and verified package lock into one run plan: node implementation and resource digests, owner projections, port bindings, bridge routes, startup dependencies, readiness checks, and reverse Stop order. Apply records that plan against an exact graph revision. Run and Restart recheck its inputs before launch; a changed package, resource, or owner capability requires a new Apply. Owners attest which declared instances became ready, and failures identify the node and plan stage. A future graph edit creates a new revision and controlled replacement; it never mutates live owner projections in place.

Move all 16 built-in demo instances through the same registered implementation path as package nodes. Bind external UI nodes to verified presentation descriptors naming the exact output port, payload type, run, and revision. A descriptor is display authority only; UI focus loss, disconnect, or control lease expiry must still close command admission and have separate recovery qualification.

Treat each live edge as a typed route with a delivery policy. Commands need bounded queues, deadlines, durable event identity, terminal result lookup, and an explicit `outcome_unknown` state when release cannot be proven. High-rate observations need bounded latest-value or sampled delivery with sequence and sample age; slow EI or UI consumers cannot block EOS's local safety checks. Both endpoint owners must confirm a cross-core route, run ID, revision, payload type, and transport before delivery. A missing implementation rejects Run instead of silently dropping values. Cycles or feedback require an explicit delay/state node and a bounded iteration policy before admission.

### Portability and the open package economy

Put process launch, IPC, paths, clocks, and signals behind host adapters with identical owner lifecycle semantics. Unix sockets are the current local transport; a platform adapter must prove equivalent private endpoint access, cancellation, child supervision, crash recovery, and Stop before support is claimed. Package authors publish typed contributions and immutable resource digests; IsOS resolves exact pins and policy-trusted metadata, while each owner revalidates its payload and capability semantics. Signed and expiring registry metadata, revocation, and an explicit offline policy are future trust gates, not current package behavior. Version graph, node, package, and transport contracts independently so an ecosystem participant can upgrade one boundary deliberately.

### Traceability, experiments, and simulation-to-real transfer

Assign every run a graph revision, plan digest, package and resource digests, owner generation, and trace ID. Propagate node path, port, event ID, causal parent, monotonic sample time, and source sequence across admitted routes. Record validation, Apply, readiness, admission, result, degraded state, recovery, and Stop as bounded structured events. Debug tooling should reconstruct the causal path and distinguish rejection before actuation from an uncertain post-release result; it must not log raw media, prompts, secrets, or unbounded payloads.

Use the same typed graph ports and EOS Dispatcher contract for simulator and physical adapters. An embodiment profile binds device identity, frames, calibration version, timing, limits, and available sensors to a run; switching profiles requires fresh verification and Apply, not a silent backend substitution. Simulation may supply extra ground truth for scoring, but that truth cannot enter the deployed inference route. Store replay inputs and provenance so a failed simulated episode can be compared with a supervised real trial under matched task and risk budgets. Physical cutover additionally needs measured stop latency, watchdog behavior, sensor freshness, and recovery tests; simulation success alone is insufficient.

### Method and acceptance gates

1. Define one typed run-plan schema in the owning interface and prove graph, package, resource, and owner capability rejection before changing the executor.
2. Run a third-party node in each owner and one cross-core typed route through setup → Apply → readiness → control → logs → Stop. Verify duplicate, stale-run, missing bridge, blocked consumer, worker loss, and uncertain result cases.
3. Replay one fixed simulator episode with its plan and trace, then bind a physical profile under supervision. Compare equivalent observations and command outcomes while withholding simulator-only labels from inference.
4. Qualify each host adapter with the same lifecycle and Stop suite. Publish support only for tested host and device combinations.
