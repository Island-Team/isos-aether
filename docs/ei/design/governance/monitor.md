# Runtime monitoring and emergency latch

[中文](../../../../docs_cn/ei/design/governance/monitor.md)

中文 · Core overview · Safety


## Heartbeat loop

`monitor_entry` normalizes one client or a backend mapping, creates independent failure counters and escalation episodes, sets readiness, and ticks every `heartbeat_interval_s`. Each tick probes every backend; one slow probe currently delays later probes because checks are sequential.

On success after any failure, monitor forces rediscovery. It then refreshes the typed status aggregate; either failure counts as a failed tick. Successful rediscovery updates the status tracker and, while estop is engaged, attempts restart reconciliation. A fully successful tick records tracker success, emits one `heartbeat recovered` observation if the backend had escalated, and resets its counter. A failed health check or refresh increments the counter. The first failure is logged; reaching `failure_threshold` emits exactly one fault per episode. Continued failure is coalesced. `availability_changed` receives whether all counters are zero after each fleet pass.

## Bounds and rate state

`PhysicalBounds.check_base` computes planar speed with `hypot(x, y)` before checking absolute yaw rate. Defaults are 2 m/s and 3.14159 rad/s; speed wins if both violate. `BoundWatchdog` ignores non-base snapshots. Every violating frame emits a fault with measured value and limit. Five consecutive violations by default engage estop once; one healthy frame resets the streak and re-arms it. The streak is global to this watchdog, not keyed by backend or base device. Non-finite measurements are not explicitly rejected here and therefore rely on typed contract validation upstream.


## Emergency lifecycle

Engagement is idempotent and fsyncs the local barrier with every affected alias and EOS robot identity before transport. For each backend, the latch calls process-wide `safety.estop`; errors or EOS refusal are logged per backend and never abort the fleet loop. Unknown EOS identity is stored as empty and cannot pass authorized reset. `motion_stop()` separately discovers every backend and best-effort calls `motion.stop` for each motion device, continuing after every failure.

## Registered node behavior

**Inputs:** health, progress, optional bounds. **Outputs:** status, latch. The [stage dataflow design](../pipeline.md) describes EI runtime values. IsOS owns the graph port and setting contract, including wire priority and frequency.

**Algorithm and failure:** Watch heartbeat, progress and measured bounds with monotonic deadlines. Escalate a verified fault to a persistent stop latch; only the dedicated emergency route may bypass workload admission. Restart never clears the latch without authorized reset.
