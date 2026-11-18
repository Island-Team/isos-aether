<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/core/governance</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Governance / monitor](monitor.md)

</details>
<!-- doc-navigation:end -->


[中文](../../../../docs_cn/ei/core/governance/safety.md)

中文 · Core overview

All planner and operator actions pass through `SafetyGate`. Remote calls resolve against the current discovery catalog by the exact triple `(robot alias, device_id, primitive name)`. Missing, ambiguous, unknown, or stale targets are rejected before transport. The MVP has no Agent-local execution path.

## Implementation map and outputs

`gate.py` owns ordinary admission and returns a verdict with admission, reason and optional EOS acknowledgement. `audit.py` consumes fault observations, coalesces recovered episodes and escalates unrecovered conditions through the safety-owned latch. `action_list.py` is a pure advisory conflict assessment; it does not execute or reserve devices. The gate reserves an owner-only operation ID checkpoint before SubmitOperation and settles it only from a terminal EOS receipt. Startup queries pending IDs with GetOperation; it never resubmits them.

An EOS acknowledgement has two distinct admission levels:

- `primitive.ack.accepted` says EOS admitted the release.

A backend refusal is a normal result. A transport or malformed-ack failure is reported as `egress_failed` without synthesizing an acknowledgement.

## Primitive visibility

- standard primitives discovered from EOS are planner-visible;
- operator primitives require EOS exposure and `primitives.expose_operator = true`;
- privileged `motion.stop` and `safety.estop` are never planner-visible.

The operator WebSocket invokes the same gate and cannot bypass these rules.

## Fleet emergency sequence


1. sends `motion.stop` to every discovered device whose class is `base`, `arm`, or `gimbal`;
2. sends one process-wide `safety.estop` with an empty device target;
3. records the endpoint's current EOS UUID in the fleet latch.

Calls are best effort across the entire fleet: a failed call is recorded and does not prevent attempts against other devices or endpoints. Re-engaging an already engaged latch is idempotent.


## Monitor interaction

The safety layer owns emergency escalation and may call the link directly for stop operations. Its `monitor_entry` loop remains a separately supervised task, so health probes do not block workload admission. Ordinary work still follows:

```mermaid
flowchart LR
  caller["Decision or operator"] --> gate["SafetyGate"] --> client["EosClient"] --> rpc["CapabilityService.SubmitOperation"]
```


## Gate order and conditions

The gate evaluates the emergency latch, caller visibility, primitive and target resolution, canonical argument schema, device limits, configured policy, runtime state constraints, and rate or cooldown limits before transport. The first failed check returns a stable rejection reason and writes an audit observation; later checks and transport do not run. Missing, duplicated, stale, or ambiguous registry entries fail closed. Boolean values do not pass numeric checks, non-finite values are invalid, and a required limit that the device omits makes that invocation unavailable.


Emergency engagement creates a barrier before stop calls begin, preventing new workload from racing the fleet stop. Unknown or vanished targets, partial fleet discovery, backend refusal, timeout, and transport failure are recorded independently and cannot clear the barrier. The action-list conflict checker is currently advisory scaffolding; it does not add a second execution path or weaken the gate.

## Registered node behavior

**Inputs:** proposal, registry, latch. **Outputs:** admission, rejection. The [stage dataflow design](../runtime/pipeline.md) describes EI runtime values. IsOS owns the graph port and setting contract, including wire priority and frequency.

**Algorithm and failure:** Check latch, target identity, exact capability, argument schema, device limits, policy, runtime state and rate window in order; record the first stable denial. Only admitted requests use the fixed internal link route. EOS performs final physical admission.


## Health, bounds, and emergency monitoring

This is an EI Python ownership change. The registered graph node `ei.governance.monitor.arm_status@v0.2` keeps its identity and IsOS contract.



## Supervised health loop

`ei.core.governance.safety.monitor_entry` normalizes one client or a backend mapping, creates independent failure counters and escalation episodes, sets readiness, and ticks every `heartbeat_interval_s`. Each tick probes every backend; one slow probe currently delays later probes because checks are sequential.

On success after any failure, monitor forces rediscovery. It then refreshes the typed status aggregate; either failure counts as a failed tick. Successful rediscovery updates the status tracker and, while estop is engaged, attempts restart reconciliation. A fully successful tick records tracker success, emits one `heartbeat recovered` observation if the backend had escalated, and resets its counter. A failed health check or refresh increments the counter. The first failure is logged; reaching `failure_threshold` emits exactly one fault per episode. Continued failure is coalesced. `availability_changed` receives whether all counters are zero after each fleet pass.

## Bounds and rate state

`PhysicalBounds.check_velocity` computes planar speed with `hypot(x, y)` before checking absolute yaw rate. Defaults are 2 m/s and 3.14159 rad/s; speed wins if both violate. `BoundWatchdog` ignores non-base snapshots. Every violating frame emits a fault with measured value and limit. Five consecutive violations by default engage estop once; one healthy frame resets the streak and re-arms it. The streak is global to this watchdog, not keyed by backend or base device. Non-finite measurements are not explicitly rejected here and therefore rely on typed contract validation upstream.


## Emergency lifecycle

Engagement is idempotent and fsyncs the local barrier with every affected alias and EOS robot identity before transport. For each backend, the latch calls process-wide `safety.estop`; errors or EOS refusal are logged per backend and never abort the fleet loop. Unknown EOS identity is stored as empty and cannot pass authorized reset. `motion_stop()` separately discovers every backend and best-effort calls `motion.stop` for each motion device, continuing after every failure.

## Monitoring node behavior

**Inputs:** health, progress, optional bounds. **Outputs:** status, latch. The [stage dataflow design](../runtime/pipeline.md) describes EI runtime values. IsOS owns the graph port and setting contract, including wire priority and frequency.

**Algorithm and failure:** Watch heartbeat, progress and measured bounds with monotonic deadlines. Escalate a verified fault to a persistent stop latch; only the dedicated emergency route may bypass workload admission. Restart never clears the latch without authorized reset.

## ei.governance.safety.gate@v0.2

Applies the EI safety gate to the activation path before EOS admission. [Node interface](../../interfaces/node/governance/safety.md#governance_safety--gate)
