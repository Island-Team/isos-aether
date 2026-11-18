# Routed connection scheduling

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/dispatcher</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [V2 dispatcher tree](dispatcher.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/dispatcher/scheduler_cn.md) · Graph contract · [Runtime foundation](../process/process.md#eos-runtime-foundation)

**R01 implementation.** A graph edge names one producer output and one consumer input. IsOS owns `priority` and the derived `frequency_hz` in saved bytes; EOS owns delivery after both endpoints are admitted. IsOS derives the edge cap from the destination instance input `desired_rate_hz.max`. The registered source output `frequency_hz` declares its expected production frequency and does not itself cap delivery. The [hardware interface](../../interfaces/node/hardware/mock.md#hardware_mock--arm) owns the target status fields and bounds.

## Delivery algorithm

An optional finite positive `frequency_hz` starts with one delivery token. After consuming it, a new token needs positive elapsed monotonic nanoseconds. For `rate <= 1e9`, compare `(elapsed_ns as f64) * rate >= 1e9`; this product is bounded below `1.85e28` for `u64` nanoseconds. For `rate > 1e9`, permit one token at the next distinct nanosecond tick. Do not calculate `1 / rate` or convert its period into an integer duration: the G06 wire admits positive subnormal values and the largest finite `f64`. A backward observation leaves the last-consumption timestamp unchanged. Consume the token on each consumer attempt, including a failed attempt, and reset the consumption timestamp to that attempt; never bank more than one token or burst after a stall. Without the field, there is no edge cap. Waiting samples remain queued under the class policy; a scheduler visit with queued work and no token increments `cap_wait_count`, not `gap_count`. The producer's own ceiling and adapter safety limits remain independent. Aggregate diagnostics instead of logging each sample.

Ready edges share weighted deficit round robin in bytewise route-key order. A **service slice** visits at most 64 distinct routes and makes at most 64 delivery attempts; the next slice resumes at a persistent cursor pointing after the last visited route. A budget exhaustion never resets the cursor to the first key. A complete round is a traversal of every ready route once, possibly spanning many slices. Nice-like `priority` in `[-20,19]` maps to positive `weight = 20 - priority` (`40..1`). On its visit a ready route gains `weight` units of credit, capped at 1024; one unit is 64 KiB of serialized payload, and a delivery costs `max(1,ceil(bytes/65536))` units, capped at 1024. It can make multiple attempts while it has credit and the slice has attempt budget; after the visit the cursor advances even if credit remains or the attempt budget was exhausted. Unused credit carries to the next round; an empty queue discards credit. A capped route with no token gets one `cap_wait_count` visit, consumes no credit, and is skipped until its next round visit. With 65 or 256 continuously ready one-unit routes at weight one, all are visited within two or four slices respectively; with arbitrary positive weights, every continuously ready route is visited within at most 256 slices because each nonempty slice advances over at least one distinct route. A weight-one maximal payload accumulates enough credit after at most 1024 visits, while higher weights can make proportionally more attempts. The loop never uses OS thread priority or promises real-time scheduling. Synthetic state/event tests exercise fairness without registering new graph routes.


R01 stop atomically closes evidence admission and fences in-flight publication. Before draining a potentially large queue or waiting on any evidence worker, the runtime invokes Dispatcher `emergency_stop` for admitted actuators and records any stop failure. It then cancels queued records. Each route has at most one in-flight consumer acceptance. That in-flight operation receives a cancellation token and must finish or expire within 100 ms; acceptance before cancellation counts as delivered, while cancellation or deadline expiry counts exactly once as cancelled and a gap. A generation fence rejects late publication after cancellation, including an image that would otherwise appear in `VisionService::capture`. `queue_depth` excludes in-flight work. Joining evidence workers has a 1 s deadline; expiry is a visible stop failure and cannot be reported as a completed join. R01 tests check this local ordering and accounting. The enclosing direct-worker stop, termination, and reap path has its own verification; R01 alone does not prove physical safe stop.

The 100 ms acceptance deadline applies to the gate consumer, which uses a cancellable slot lock. A camera adapter capture is a separate synchronous producer call and may block; shutdown fences its later publication, waits up to 1 s for evidence workers, and exits with failure if a worker remains blocked. Rust cannot kill that blocked thread or claim that it joined. The existing adapter host timeout and R06 direct-worker stop/reap remain separate owner boundaries.

A gate consumer prepares a frame without changing its visible slot. Under the same scheduler lock used by shutdown admission sealing, the final deadline and cancellation check commits the frame, delivered generation, counter, and acceptance timestamp together. Shutdown seals the scheduler before Dispatcher `emergency_stop` and drains queues only afterward; a paused producer cannot enqueue after the seal. Camera sampling computes a saturating `Duration` for every admitted finite positive `max_rate_hz`, including subnormal values, and polls stop at 100 ms intervals while waiting.

## VisionService integration seam

`crates/aether-runtime/src/bin/aether-runtime/vision.rs` retains one capture worker per camera source. The admitted route table sits between capture and each `@v0.2` gate slot. It serializes the frame once outside the scheduler lock, then shares one immutable `Arc<CameraFrame>` across the exact declared gate routes; each route applies its own cap and queue. The locked fanout clones only `Arc` handles, including when the 64 MiB quota rejects later routes. Only a successful route consumer acceptance publishes to a gate-specific latest slot and increments that gate's delivered generation. `VisionService::capture` for an R01 gate reads that slot and generation, so a waiting, replaced, dropped or consumer-failed frame cannot appear through the browser preview. Two gates sharing one camera may have different caps and generations without multiplying camera captures. Gate health still comes from Dispatcher status and health, not route counters.


## Diagnostics and acceptance

The owner-only runtime socket target `route_status` returns validated EOS-local declarations, ordered by key, with no payload. Active rows expose configured QoS, queue depth, delivered/wait/loss counters and last successful monotonic delivery; local design-only and fixed rows have null scheduling values, `routed:false`, and no inferred health. Cross-core declarations are omitted. IsOS remains the deployment-status authority. Tests cover exact-route admission, similar but unrouted types, fake-clock caps including extreme finite values and backward time, 65/256-route cursor progress, latest replacement, FIFO/global saturation, consumer error, bounded in-flight stop and emergency-stop ordering, and the invariant that no edge creates a second actuator route. [First use](scheduler.md#inspect-eos-evidence-routes) describe the implemented behavior and remaining qualification work.

The G06 profile admission baseline is reproducible from the EOS repository root with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo build --workspace`. Full output for the 2026-11-01 run is in local `data/test/20261101-g06-edge-contract/{fmt,clippy,tests,build}.log`. The profile edge fixture tests passed; R01 delivery and queue behavior still require their own integration evidence.


## R01 implementation evidence


Independent R01 review corrections were retested on 2026-11-01. The gate slot is now published only after the final locked deadline/cancellation decision; `begin_shutdown` seals admission under that lock before Dispatcher stop, and queue draining follows stop. `last_delivery_monotonic_ns` is sampled at the successful commit. The camera capture wait saturates for positive subnormal rates and remains stop-pollable. Deterministic tests pause a consumer and producer across the seal, hold preparation past its 100 ms deadline, check the later commit clock, and start/stop a process with a subnormal camera rate. The same full fmt, workspace Clippy/test/build, fixture, link, and diff gates passed; complete output is in local `data/test/20261101-r01-review-corrections/`.

A later R01 review found full frame clones under the producer's scheduler lock. The producer now serializes once outside that lock and fans out shared immutable `Arc<CameraFrame>` handles. A deterministic 256-route regression uses an 8 MiB declared payload and a saturated 64 MiB quota while a stop-seal request waits for the producer lock; it checks eight retained pointers, 248 overflow counters, and bounded lock handoff. The full fmt, workspace Clippy/test/build, 28-case fixture, 302-document link and diff gates passed again. Complete output is in local `data/test/20261101-r01-shared-fanout/`.

<a id="inspect-eos-evidence-routes"></a>

## Inspect EOS evidence routes

IsOS owns graph admission and Apply. EOS admits the declared camera frame → `eos.interface.egress.vision_flow@v0.2` route. The owner socket exposes `route_status` for the current runtime generation.

Install and verify the camera package, select its frame output and the Vision egress input in the IsOS graph, then Validate and Apply as described in [first use](../../../isos/start/quick-start.md). The camera output may declare an official `frequency_hz`; the Vision input may declare `desired_rate_hz.max` as its delivery cap. A v1 or v2 native Vision source needs a `node.adapter.settings.frequency_hz` within `(0, 1000]` Hz on its graph instance for EOS to capture frames into the source pool. The official package output frequency alone does not schedule capture; without the instance setting, that native source produces no pooled frames. Authored edge `frequency_hz` is rejected. A successful Apply admits the declaration; camera health and frame delivery are checked at Run.


To inspect an overload, compare `delivered_count`, `cap_wait_count`, `replaced_count`, `overflow_count`, `consumer_error_count`, `cancelled_count`, and `gap_count` within one runtime generation. A rate cap can increase waiting without loss. A new image replacing a retained image increases replacement and gap counts. Stop closes evidence admission, invokes Dispatcher emergency stop before waiting on evidence workers, and cancels pending evidence with a gap. The owner socket no longer serves `route_status` during or after shutdown; inspect counters before stopping and use IsOS status and run events afterward. Command results, health, and stop come from their existing owners and remain outside these queues. Use IsOS run-bound logs to find the first ready or failure event, then correlate with owner status; do not infer health from a route row. A restarted runtime resets counters; use IsOS run and process identity to distinguish generations even if the projection digest is unchanged.

If the route is `not_routed`, check the exact source and target type IDs, ports, data type and transport against the [contract](../../interfaces/node/hardware/mock.md#hardware_mock--arm). Similar or contributed identities do not gain delivery. If startup rejects a missing compiled endpoint or too many live routes, use IsOS's owner-reported failure and keep the saved graph unchanged while correcting it. [Scheduler design](scheduler.md) explains the bounded queue and fairness rules.

The mock-arm worker may remain available for observation when a declared activation edge is absent. The owner records whether the command and observation bindings are complete at Run. It rejects activation or monitor reads before contacting the worker when the corresponding chain is disconnected. The applied graph stays read-only; reconnecting requires another Apply and Run.
