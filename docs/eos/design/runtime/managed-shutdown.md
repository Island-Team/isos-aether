# Managed routine shutdown and durable safety state

[Owner contract](../../interfaces/simulator-stop.md) · Dispatcher · M5 qualification

## Problem and boundary


The implementation will distinguish an owner-requested **routine managed shutdown** from a genuine `safety.estop`. It adds no public reset operation, graph field, grant, actuator ingress, or new wire identity. A genuine emergency latch remains durable and cannot be cleared by Stop, Restart, ordinary `robot.stop`, or a new simulator generation. A crash, unknown movement release, unknown physical stop, or unknown child reap remains fail-closed.

## State transition

1. Before any actuator stop, EOS closes movement and evidence admission and sets the Dispatcher in-memory inhibit. The managed routine path snapshots the admitted moving targets. The journal adds a `1.1.0` robot-bound `shutdown_pending` marker and selected `shutdown_targets`; pending is true exactly when targets are nonempty. The reader strictly verifies a `1.0.0` control record using its original body shape and checksum before assigning false/empty defaults; only the next successful control mutation upgrades the shape and checksum. A pending marker's targets must match the admitted robot/profile on reopen. This marker is distinct from the existing `robot_inhibited` recovery state and `estop_latched`; a new EOS process with an unresolved marker withholds movement. It never overwrites or treats a preexisting cause as its own.
2. It starts the existing one-request MuJoCo companion-socket stop coordinator within the R06 deadline, before waiting for journal persistence. Concurrent public `robot.stop` or `safety.estop` joins the physical request but retains its own durable identity and safety effect. The routine path atomically persists its own shutdown marker and target set before a positive physical-stop result can be claimed. If this write stalls or fails, the companion stop still runs; the selected outcome is unknown, without claiming that a marker was persisted. If EOS crashes before the write, IsOS's selected unknown-stop gate blocks replacement.
3. A fully matched generation and newer epoch must prove both base and arm stopped. EOS then drains/reaps the selected MuJoCo group, removes both backend sockets and its control socket, shuts down host skills and evidence, and checks every existing R06 receipt predicate and global deadline. Public ingress closes before stop admission ends; a `safety.estop` admitted before closure must durably latch and settle, while later calls are denied. Any in-flight public stop still unresolved at the deadline makes the selected receipt unknown. No subset of these facts permits a confirmed receipt. Internal shutdown's per-part result is evidenced by the selected raw receipt; only public stop operations have separate durable per-part journal records.
5. If any predicate is unknown, the shutdown marker remains durable. A late adapter result cannot clear it. If a genuine `safety.estop` was latched before or during shutdown, the latch remains true even if the routine physical stop and child reap were confirmed. A new EOS run advertises movement unavailable until a separately specified, authorized safety recovery exists.


## Focused owner implementation evidence (2026-11-02)


From this worktree root, the reproducible gates were:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
cargo build --workspace
python3 scripts/check_doc_links.py docs/eos/interfaces/simulator-stop.md docs/eos/design/process/mujoco-operations.md docs_cn/eos/interfaces/simulator-stop_cn.md docs_cn/eos/design/process/mujoco-operations_cn.md
git diff --check
```

All passed; complete output is under `data/test/20261102-managed-shutdown/`. No throughput or latency claim is made.
