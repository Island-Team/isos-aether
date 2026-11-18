# V2 dispatcher tree

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/dispatcher</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Routed connection scheduling](scheduler.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/dispatcher/dispatcher_cn.md) · [Hardware design](../hardware/hardware.md) · [Field-level interface](../../interfaces/node/hardware/mock.md#hardware_mock--arm)

## Why the tree exists


The dispatcher is a Rust module separate from the HAL. Discovery reads an immutable registry snapshot built from the validated node profile. A movement request names one node and one typed mode. The dispatcher resolves that node once, then checks the profile's declared capability, observed health, required sample freshness, command deadline, numerical limits, and safety state before handing the command to its adapter. The adapter is the final physical authority. Navigation planning, trajectory generation, and inverse kinematics are outside this layer.

Package-contributed node types use the same Dispatcher admission rules as built-in types. A package safety declaration cannot grant an actuator route without an accepted adapter and Stop implementation.


## Command lifecycle

Every command has a caller-supplied `command_id` and monotonic deadline. `Dispatcher::new_durable` opens an exclusive, bounded journal directory. It syncs a versioned reservation and then a `releasing` record before calling an adapter. A duplicate ID returns its stored outcome; conflicting reuse is rejected. On restart, an unfinished record becomes `outcome_unknown` and is never automatically released again. Corrupt or oversized records prevent startup. The in-memory constructor remains for tests and in-process prototypes; the managed Rust runtime uses the durable constructor. A failed journal write before release rejects the command; a failed terminal write after release leaves its outcome uncertain.

Emergency stop latches process safety before calling every actuator's independent stop method. The dispatcher no longer holds a release lock across a potentially blocking adapter call. The adapter must latch stop at its physical write boundary so a delayed submission cannot actuate after stop; the CXX mocks implement and test this rule. The mock test proves stop returns while a submission is blocked, but a real controller's stop latency and independent velocity watchdog still require measurement before cutover. An observation or validation failure degrades that node's Rust-side health; `refresh_node` clears the fault only after a valid, fresh probe. Health, safety, and physical stop errors remain visible separately.

## Verification

Use a fake HAL registry to prove exact path resolution, facet discovery, and isolation between nodes. Integration tests cover healthy movement, stale observation rejection, limits, expired deadline, duplicate ID replay across restart, corrupt journal rejection, health degradation and recovery, cancellation before release, and stop during a blocked mock release. Physical stop latency, controller-enforced velocity lease expiry, and a deployed ROS 2 graph remain cutover acceptance cases. Record reproducible test and benchmark commands in the [development guide](../hardware/hardware-guide.md), with full output in `data/test/<run-id>/`.

The button demo admits `arm_joint_delta@v0.2` through the built-in `arm.delta` primitive. It rejects a delta greater than 0.25 rad per joint or a resulting joint outside ±3.14 rad before the mock adapter updates state. See [arm control](../../interfaces.md#arm_control).

The demo monitor return path checks the latest six-joint observation independently of command responses. The configured sample timeout (50 ms by default), a nonfinite joint, or a position beyond the configured limit removes movement readiness. The 100 Hz sampler can restore readiness after a fresh valid sample. The private [monitor read](../../interfaces.md#arm_monitor) remains read-only and never bypasses Dispatcher admission.

<a id="r03-governed-actuator-admission-design"></a>

## R03 governed actuator admission design

### Admission and durable identity

Each public motion ingress builds a canonical command with its command ID, IsOS deployment `run_id`, caller role, server-proven ingress, active saved revision, exact primitive/version and binding source, hardware target, typed arguments, and a separately checked monotonic deadline. Dispatcher durably reserves this identity before possible physical release. Credentials and transport request IDs are excluded from the durable signature; role, scope, ingress, run ID and revision are included. A repeated ID with the same canonical request returns its retained state. A changed revision, role, binding, target or argument conflicts and never releases. A new IsOS deployment rotates its run ID and grant; an old result cannot be claimed as success for the new run. A released `OUTCOME_UNKNOWN` is terminal for that ID and is never retried automatically.

Stop records retain role, run ID, stop kind (`MotionStop`, `RobotStop`, or `SafetyEstop`), exact target, revision where required, and recovery marker. Get, Watch, and Cancel enforce the journaled caller scope and run. After reservation, Dispatcher checks the release barrier, safety latch and inhibit, exact binding and adapter kind, active revision, authorization, node lifecycle and fresh health, required typed observations, finite values and configured units, and remaining deadline. It writes `Releasing` durably before calling an adapter. Queue expiry or a journal failure before release results in zero adapter calls. Ambiguous physical release, terminal persistence loss, or a crash after `Releasing` becomes durable `OUTCOME_UNKNOWN` and inhibits motion until explicit stop recovery proves a safe state. A transport timeout or disconnect never proves completion or cancellation.

### Verification boundary

The [frozen cases](../../../../tests/fixtures/r03_admission_revision/cases.json) cover managed revision provenance, missing and stale revisions, caller grants, exact bindings, typed limits, fake-clock health and deadlines, blocked-queue rechecks, duplicate IDs, journal failures, restart, concurrent Stop, and former bypass paths. Unit tests use fake clocks and one-call adapters; integration tests use the real Rust listener and private socket with a stub adapter. Physical controller deduplication and selected service child stop/reap timing belong to R05/R06 and are not inferred from these mocks.

### Governed actuator admission: development and verification

IsOS issues a run-scoped grant for the selected graph revision. EOS validates the grant, active binding, health, deadline, and caller identity before Dispatcher releases an actuator command. A request admitted under a prior run or revision is rejected. The journal stores one result for each operation ID; a lost response or uncertain outcome never permits automatic physical replay.

#### First-use replay

1. Build the Rust workspace and start IsOS with a verified graph containing separate `isos`, `ei`, and `eos` cores with registered `<core>.<layer>.<group>.<node>@v0.2` types.
2. Start EOS through IsOS and confirm the managed grant and active graph revision are delivered to the selected child only.
3. Submit a valid base or manipulator command through the public EOS interface. Read the same operation ID until a terminal result; confirm one journal record and one adapter release.
4. Repeat with an invalid grant, caller, target, revision, payload, and expired deadline. Each request must fail before adapter release and must not reveal another caller's result.
5. Exercise cancellation, process failure, a journal fault, and Stop while a command is pending. Confirm the release fence, durable result classification, and physical Stop order described in the [owner stop contract](../../../isos/interfaces/server/eos.md#owner_health).

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, serial workspace tests, the [case checker](../../../../tests/fixtures/r03_admission_revision/README.md), and `python3 scripts/check_doc_links.py --all`. Save complete output under `data/test/<run-id>/` and record the exact revision, commands, environment, and case-to-test mapping with the run.

<a id="eos-local-logs"></a>

## EOS local logs

The EOS owner and arm worker append to one local JSONL file: `data/eos/log/<deployment>/eos.jsonl` for a managed deployment or `data/eos/log/local/eos.jsonl` standalone. IsOS reads the file for its log panel. The owner API returns lifecycle and activation results; log records remain local.

Each record has a UTC RFC 3339 timestamp, lowercase level, source, and message. Run-scoped records include the run ID. The mock arm source is `eos.hardware.mock.arm@aether-v0.2.3`; worker system events use `eos.aether_runtime.worker@aether-v0.2.3`. The owner records a worker loss once when observed and records failed Run or Stop outcomes. An activation reply, rather than a log line, proves the command's terminal result.

Older bridge and config-server binaries retained for callers write bounded records into this same file through `LocalCoreLog`. The current demo graph does not launch them. Verify local logs and owner/worker lifecycle with `cargo test -p aether-runtime --all-targets`; retain complete output under ignored `data/test/<run-id>/`.
