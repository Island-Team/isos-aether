# Rust v2 process spine

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/process</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)

</details>
<!-- doc-navigation:end -->



The process owns admission, command history, node health, and shutdown for its mock nodes. The unified profile supplies identity, location, capabilities, limits, and adapter selection. The dispatcher is the sole command route; the socket handler does not call adapters directly. An invalid profile, unsupported adapter, incomplete admission, unavailable journal, or socket bind error fails startup without a READY announcement. A signal or shutdown request stops admitted actuators and removes the socket.

`aether-runtime` is the private Rust worker started by the persistent EOS owner. It accepts one validated node profile, constructs only selected adapters, opens a durable command journal, and binds its owner-only control socket after readiness checks. Dispatcher remains the sole command route.

## Startup and local control fields

| Field | Type | Purpose and rule |
| --- | --- | --- |
| `--profile` | UTF-8 file path | Supplies one bounded unified node profile; its root `version` must be `"1.0.0"`. |
| `--journal-dir` | directory path | Holds durable command reservations and outcomes; one process holds its exclusive lock. Keep it on persistent storage for restart testing. |
| `--socket` | Unix socket path | Local control endpoint; bind failure prevents readiness. Access to this path must be limited by local filesystem permissions. |
| READY `version` | string | Identifies the process protocol as `"1.0.0"`. |
| READY `status` | string | `"ready"` only after profile, adapters, journal, and socket have been admitted. |
| READY `socket` | string | Bound control socket path. |
| READY `hardware_nodes` | unsigned integer | Count of EOS hardware nodes registered for dispatch; software nodes remain in the profile but do not receive adapters. |
| request `op` | string | Selects `discover`, `health`, `time`, `dispatch`, `result`, `stop`, or `shutdown`. |
| request `node_path` | string, required for `health`, `dispatch`, and `stop` | Full `cores/.../node/...` path. Short IDs and software paths cannot control hardware. |
| request `command_id` | string, required for `dispatch` and `result` | Durable idempotency key; replay an identical request to retrieve its prior outcome. |
| request `deadline_monotonic_ns` | unsigned 64-bit integer, required for `dispatch` | Absolute process-local deadline. Obtain current time with `time` and submit before expiry. Replay the exact original value after restart. |
| request `movement` | tagged object, required for `dispatch` | `body_velocity`, `relative_pose`, `joint_positions`, or `end_effector_pose`, with [typed hardware fields](../../interfaces/node/hardware/mock.md#hardware_mock--arm). The profile must admit its mode and limits. |
| response `ok` | boolean | Indicates whether the operation completed. |
| response `nodes` | array, on `discover` success | Hardware node descriptors derived from the admitted profile. |
| response `health` | object, on `health` success | Effective adapter health, including Rust-side observation faults. |
| response `monotonic_ns` | unsigned 64-bit integer, on `time` success | Current process-local time for calculating a command deadline; do not compare it with another process's clock. |
| response `result` | object, on `dispatch` or `result` success | Durable command ID, outcome status, and optional reason. A rejected command is still a recorded dispatch result. |
| response `error` | string, on failure | Human-readable validation or operation failure. Do not parse it as a stable error code. |

The socket uses one bounded newline-delimited JSON request and response per connection. The limit is 64 KiB per request. This surface is for local integration and lifecycle testing. `dispatch` uses the same full path, typed movement, deadline, and durable journal as the dispatcher. Replay the same semantic fields, including the absolute deadline; the journal prevents a second actuator release after restart.

## Verified package handoff

Use `isos packages` to install or link an exact verified release. The saved graph pins the package on the selected node and names stable resources; installation alone does not select a runtime process. IsOS checks package metadata, payload digests, node contributions, and resources before Apply and again before Run or Restart. EOS receives the verified process and resource handoff, starts its selected direct service child, and reports readiness and Stop through its owner contract.

For the older package-contributed topology, a node type must match its `isos.eos_process.<name>@v0.2` placement and exact package pin. Hardware nodes refer to the selected adapter package through their own settings. See the [IsOS release contract](../../../isos/interfaces/package/README.md#release) and [verification contract](../../../isos/interfaces/package/README.md#verification).

<a id="eos-runtime-foundation"></a>

## EOS runtime foundation

This page describes the current Rust foundation and records dated verification. The later design section includes implemented graph, readiness, routing and stop behavior alongside remaining physical-adapter work; its contracts are owned by [hardware](../../interfaces/node/hardware/mock.md#hardware_mock--arm), [node contributions](../../../isos/interfaces/server/eos.md#node_contributions), [control](../../../isos/interfaces/server/eos.md#control), and [gRPC](../../../isos/interfaces/server/eos.md#grpc).

### Boundaries and failure rules

- A node type owns schema and placement. A node instance owns identity and settings. Typed graph edges declare data flow; the dispatcher alone admits actuator commands and writes durable results.
- IsOS starts EOS, which starts its selected verified workers and waits for their private readiness proof before reporting EOS readiness. If a required worker fails, EOS stops; IsOS can restart the EOS component according to its bounded policy.
- EOS acquires the journal before binding its control socket. On restart it may unlink a stale socket only when the path is a Unix socket and no live peer accepts a connection. An uncertain released command is never automatically replayed.
- EOS sends stop to all admitted actuators on graceful shutdown. A measured physical controller stop bound remains necessary before hardware deployment.
- The HTTP bridge and read-only config server bind loopback. The bridge has no adapter access; it forwards bounded requests to EOS.

### 0.x runtime design and remaining work





R05 proves strict identity and same-child retransmission handling. R06 owner implementation supplies stop admission fencing, a bounded direct-worker stop lane, independent base/arm acknowledgment or durable unknown, and termination/reap after the stop attempt. Exact-head and final-negative cross-owner replays cover selected paths; full R06 qualification still needs the remaining live faults recorded in the [stop design](process.md#managed-routine-shutdown-and-durable-safety-state).

<a id="rust-eos-services"></a>

## Rust EOS services

The persistent EOS owner supervises its runtime worker. IsOS supplies a verified graph projection and selected package handoffs; EOS constructs declared adapters, admits commands through Dispatcher, and reports health and Stop for the active run.



| Executable | Invocation | Role |
| --- | --- | --- |
| `aether-profile` | `validate --file <graph>` | Validate the graph and registered node types before admission. |
| `aether-server` | `--socket <path> --listen <control-loopback:port> --profile <graph> --catalog-listen <catalog-loopback:port>` | Serve bounded control, health, vision, and read-only graph catalog routes from one process on two loopback listeners. |

`aether-system::io::control` supplies the bounded private client used by the server. Control requests are limited to 64 KiB, responses to 16 MiB, and socket reads have two-second deadlines. `capture` returns RGB8 as `rgb8_base64` and optional depth as `depth_u16_le_base64`; `observe` returns typed base or manipulator data. The bridge validates request size and identity before forwarding. Dispatcher remains the sole actuator admission and release path.

### Rust skill workers

The runtime loads only IsOS-selected, verified native Rust `.skill` executables. It stages verified bytes privately and starts a fresh confined `aether-runtime --skill-worker` child for inspection or invocation. The worker receives a private protocol socket and staged executable descriptor, then applies resource limits, Landlock, and seccomp. Primitive calls return through the host broker and Dispatcher; the worker has no direct adapter handle. EOS owns cancellation, bounded reap, and live execution status. See the [Rust skill contract](../../interfaces.md#skills_rust).

### First use and checks

From the repository root, run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `RUST_TEST_THREADS=1 cargo test --workspace --no-fail-fast`. Start the IsOS manager with `isos serve`, then run an applied deployment with `isos run`; inspect it with `isos status --json` and `isos logs eos --lines 200`. Use `isos deployment stop` for the deployment or `isos stop` for the manager. Direct service invocations are useful for focused boundary tests; production graph ownership and package resolution belong to IsOS.

For the 2026-11-16 crate layout change, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 scripts/check_doc_links.py --all`, `git diff --check`, and `python3 -m unittest crates.isos.tests.test_product_installer`, and `python3 tests/reader_features_cli.py` passed. The new server's 12 tests cover both listeners and SIGTERM, and the 72 IsOS process tests pass with `ISOS_AETHER_BIN_DIR="$PWD/target/debug"`. The full `ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --no-fail-fast` run failed in two unchanged fixture boundaries: two `package_observer` cases reject a frame data-class mismatch before runtime execution, and the repository demo graph's camera width is 128 where its IsOS starter test expects 1280. Complete output is under ignored `data/test/20261116-eos-crate-layout/`.

<a id="managed-routine-shutdown-and-durable-safety-state"></a>

## Managed routine shutdown and durable safety state

### Problem and boundary


The implementation will distinguish an owner-requested **routine managed shutdown** from a genuine `safety.estop`. It adds no public reset operation, graph field, grant, actuator ingress, or new wire identity. A genuine emergency latch remains durable and cannot be cleared by Stop, Restart, ordinary `robot.stop`, or a new selected service generation. A crash, unknown movement release, unknown physical stop, or unknown child reap remains fail-closed.

### State transition

1. Before any actuator stop, EOS closes movement and evidence admission and sets the Dispatcher in-memory inhibit. The managed routine path snapshots the admitted moving targets. The journal adds a `1.1.0` robot-bound `shutdown_pending` marker and selected `shutdown_targets`; pending is true exactly when targets are nonempty. The reader strictly verifies a `1.0.0` control record using its original body shape and checksum before assigning false/empty defaults; only the next successful control mutation upgrades the shape and checksum. A pending marker's targets must match the admitted robot/profile on reopen. This marker is distinct from the existing `robot_inhibited` recovery state and `estop_latched`; a new EOS process with an unresolved marker withholds movement. It never overwrites or treats a preexisting cause as its own.
5. If any predicate is unknown, the shutdown marker remains durable. A late adapter result cannot clear it. If a genuine `safety.estop` was latched before or during shutdown, the latch remains true even if the routine physical stop and child reap were confirmed. A new EOS run advertises movement unavailable until a separately specified, authorized safety recovery exists.


### Focused owner implementation evidence (2026-11-02)


From this worktree root, the reproducible gates were:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
cargo build --workspace
git diff --check
```

All passed; complete output is under `data/test/20261102-managed-shutdown/`. No throughput or latency claim is made.
