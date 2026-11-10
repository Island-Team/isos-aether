# EOS managed simulator stop contract

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](arm-control.md) for the active data and command path and [owner lifecycle](owner-health.md) for setup, Run, Status, and Stop.


[中文](../../../docs_cn/eos/interfaces/simulator-stop_cn.md) · [MuJoCo binding](mujoco.md) · Dispatcher · [Design](../design/runtime/simulator-stop.md) · [First use](../design/process/mujoco-operations.md#managed-stop-first-use) · R06 task

**Implemented owner contract with scoped cross-owner evidence; the original R06 row and scoped M3 exit are qualified, while broader live proof remains open.** EOS owns its Dispatcher admission, device adapters, direct MuJoCo child, and a terminal stop receipt. IsOS owns the dependent process groups and consumes the receipt from the selected EOS child's stdout pipe. MuJoCo owns the independent base and arm hold proof on its companion owner-only stop socket. This contract adds no graph field and grants no command authority to a graph edge or socket client.

`aether-runtime --capabilities` includes the exact `managed-stop-v1` feature token when this implementation is available. This is a static compatibility announcement for IsOS preflight, not a statement that the current child has stopped or that a selected run has a confirmed receipt. Confirmation still requires this run's strictly validated stdout receipt and successful EOS exit.

## Stop trigger and admission boundary

An IsOS-owned `SIGTERM`, an owner-authenticated private `shutdown`, a fatal backend transition, or normal EOS exit enters one idempotent managed shutdown state. The first transition closes new gRPC and private movement admission, stops skill publication and Vision capture, and makes health `NOT_SERVING`. It records a shutdown fence in Dispatcher before any adapter call. A second signal or shutdown request joins that state; it cannot restart movement or start a second physical stop. `SIGKILL` and process crash cannot produce a receipt. Standalone EOS retains its current best-effort stop semantics and cannot claim a managed receipt.


For a managed graph with any declared base or manipulator, startup retries transient companion stop-health failures only until the existing absolute startup deadline. An invalid proof or generation mismatch fails startup immediately. EOS reports ready only after both sockets prove the same generation for every declared actuator; a camera-only graph may remain read-only without a stop generation. A companion lane that never binds yields `eos_start_failed`, with no ready output or movement admission.

EOS sends one whole-robot stop on the companion socket, with the exact child generation, within a single monotonic shutdown deadline. Full confirmation requires a version- and ID-matched response with the same generation, a canonical newer epoch, `base_stop:"stopped"`, `arm_stop:"stopped"`, `ok:true`, and `accepted:true`. A valid `stop_outcome_unknown` response with the same identity and epoch may record one independently proved `stopped` part while the other remains `unknown`; it cannot confirm the whole stop. A timeout, invalid frame, disconnect, child exit, or absent response marks both unknown unless a separately verified part was already recorded. A stopped part means the MuJoCo engine committed zero base drive or arm hold and fenced every pre-stop movement write; it does not assert mathematical zero residual velocity. EOS cannot infer this from an adapter's local `AtomicBool`, a read-only health probe, a lease expiry, or a process exit. GUI/render failure **before** R05 ready fails startup; a blocked render **after** ready cannot consume the companion stop lane and can still yield confirmed dual stop proof. If the engine itself stops responding to the companion lane, the result is unknown.

One run-scoped MuJoCo stop coordinator owns that physical companion-socket request and its `(generation_id, old_epoch, new_epoch, base_stop, arm_stop)` result. `robot.stop`, `safety.estop` and internal shutdown call this coordinator through Dispatcher; simultaneous public stop IDs join the one in-flight physical request, keep distinct durable journal identities, and cannot each send a stop frame. EOS immediately sets the in-memory robot inhibit and registers a stop writer before the physical request. Public `robot.stop` and `safety.estop` persist their respective stop intents and per-part `attempting`/terminal results. Internal managed shutdown persists its separate shutdown marker; its selected raw terminal receipt carries the classified per-part results after full child cleanup. A stalled journal write cannot delay the companion stop. Public operations require durable intent and per-part results; a managed receipt requires durable marker resolution plus verified physical stop and cleanup facts before the raw receipt is written and IsOS validates it. A single bounded physical result is mapped to each selected graph path; a failed part or lost persistence leaves all movement inhibited and the relevant result unknown. A later stop after any new movement release requires a new physical request. An estop keeps its latch regardless of a positive controlled-stop result. A stop must preempt a movement that holds Dispatcher's normal release permit: the coordinator reserves its safety lane and advances the MuJoCo engine epoch **before** it waits for that movement's adapter return. It may wait for journal bookkeeping afterward, but never delays the physical stop on the movement permit or an ordinary capture/worker. The existing per-path `stop_part` loop and `wait_drained` order are not sufficient for managed MuJoCo.

### Routine shutdown state distinct from emergency stop

For IsOS Stop or Restart with a ready managed MuJoCo child, EOS uses a Dispatcher managed-shutdown stop. It keeps the same admission closure, preemptive physical coordinator, dual-part proof and 15-second receipt deadline, but does not create or clear a `safety.estop` latch. An actual authenticated `safety.estop` always latches, including when it races with shutdown. Fatal or unproved shutdown remains unknown. Standalone and non-MuJoCo fallback stop behavior remains fail-closed.



## Selected-child terminal receipt


```json
{
  "schema_version": 1,
  "type": "eos_terminal_stop",
  "run_id": "r42",
  "eos_pid": 4201,
  "active_revision": "7a8daee6b7db9105f5483506ffc1743625b63089710dfbf936622329c471016b",
  "backend_node": "isos/eos_process/mujoco/sim",
  "backend_pid": 4210,
  "backend_pgid": 4210,
  "generation_id": "00112233445566778899aabbccddeeff",
  "stop_epoch": "1",
  "admission_closed": true,
  "parts": [
    {
      "node_path": "isos/eos_hardware/base/base-0",
      "kind": "base",
      "outcome": "stopped"
    },
    {
      "node_path": "isos/eos_hardware/manipulator/arm-0",
      "kind": "arm",
      "outcome": "stopped"
    }
  ],
  "backend_group_drained": true,
  "backend_reaped": true,
  "sockets_removed": true,
  "outcome": "confirmed",
  "reason_code": null
}
```

The `parts` array contains exactly the selected graph's stop-capable MuJoCo base and manipulator nodes in canonical path order, each once. For a scene without one kind, the absent part is omitted only if no such actuator was admitted; the MuJoCo whole-robot response must still report both base and arm status. An explicit unknown receipt uses `outcome:"unknown"`, a fixed `reason_code`, and the strongest independently proved part statuses. It never substitutes an empty `parts` array to conceal a selected actuator.

| Field | Type / unit | Purpose, absence, validation and compatibility |
| --- | --- | --- |
| `schema_version`, `type` | unsigned integer and literal / — | Required exact `1` and `eos_terminal_stop`; unknown versions and duplicate or extra keys fail closed. This is not a P06 event. |
| `run_id` | ASCII token / — | Required exact managed IsOS run ID, 1–64 letters, digits, `_` or `-`; no default. |
| `eos_pid`, `backend_pid`, `backend_pgid` | positive unsigned integers or nullable backend IDs / process IDs | Required observed EOS PID, its spawned direct MuJoCo child PID and its separate process group ID (`backend_pgid == backend_pid` after `setsid`). IsOS compares EOS PID with its selected unreaped child, not a value from a log. Missing backend IDs are null only for unknown startup rollback before spawn. |
| `active_revision` | lowercase 64-hex string / SHA-256 | Required exact IsOS applied revision admitted by EOS; no fallback to saved graph bytes or caller input. |
| `generation_id` | lowercase 32-hex string or null / child identity | Current MuJoCo generation proved by both sockets; null only when no child was ready and outcome is unknown. Once both sockets prove readiness, EOS retains this verified identity in the terminal receipt even if the stop socket disappears or the later stop response is lost. Never an authorization secret. |
| `stop_epoch` | canonical ASCII decimal string or null / child-local counter | The epoch returned by the classified stop; null if no valid response. Required for confirmed, greater than the epoch captured for the stop. It is not the graph revision. |
| `admission_closed`, `backend_group_drained`, `backend_reaped`, `sockets_removed` | booleans / — | Required exact facts from EOS's shutdown state and owned resources. All must be true for confirmed. `backend_group_drained` means the held leader's original PGID/session and every proven descendant had no remaining live or zombie member other than the leader before EOS reaped it, with no ambiguous newly adopted direct EOS child. No inference from leader PID disappearance or a path's absence before unlink. |
| `parts` | array / records | Required selected actuators, max 64, canonical path order, unique paths and kind; no default. Each record contains only `node_path`, `kind` (`base` or `arm`) and `outcome` (`stopped` or `unknown`). |
| `outcome` | `confirmed` or `unknown` / — | Confirmed only when all selected parts stopped, full MuJoCo dual proof succeeded, admission closed, the child group drained and leader was reaped without forced kill, sockets removed and receipt written before deadline. Unknown is terminal for this run. |
| `reason_code` | fixed code or null / — | Null exactly for confirmed; unknown uses `stop_timeout`, `stop_socket_unavailable`, `stop_response_invalid`, `stop_part_unknown`, `backend_exited`, `backend_forced_kill`, `backend_group_unreaped`, `backend_reap_failed`, or `socket_cleanup_failed`. A failed receipt write produces no usable receipt. No raw path, error or secret. |

An example unknown result after an arm hold fault is:

```json
{
  "schema_version": 1,
  "type": "eos_terminal_stop",
  "run_id": "r42",
  "eos_pid": 4201,
  "active_revision": "7a8daee6b7db9105f5483506ffc1743625b63089710dfbf936622329c471016b",
  "backend_node": "isos/eos_process/mujoco/sim",
  "backend_pid": 4210,
  "backend_pgid": 4210,
  "generation_id": "00112233445566778899aabbccddeeff",
  "stop_epoch": "2",
  "admission_closed": true,
  "parts": [
    {
      "node_path": "isos/eos_hardware/base/base-0",
      "kind": "base",
      "outcome": "stopped"
    },
    {
      "node_path": "isos/eos_hardware/manipulator/arm-0",
      "kind": "arm",
      "outcome": "unknown"
    }
  ],
  "backend_group_drained": true,
  "backend_reaped": true,
  "sockets_removed": true,
  "outcome": "unknown",
  "reason_code": "stop_part_unknown"
}
```

## Deadline and lifecycle

EOS starts one monotonic 15,000 ms shutdown deadline at its first stop transition `t0`. It closes ingress and snapshots targets by `t0+1,000 ms`, classifies the stop-socket exchange by `t0+4,000 ms`, terminates and reaps the child by `t0+10,000 ms`, removes sockets and publishes the receipt by `t0+13,000 ms`, and exits by `t0+15,000 ms`. These are hard **phase deadlines** inside the single global deadline; missing one makes the terminal outcome unknown and starts the next cleanup phase without extending time. The physical stop gets at most 3,000 ms after admission closure; child TERM/reap gets at most 6,000 ms, including at most 3,000 ms before a necessary identity-safe KILL; cleanup and nonblocking receipt get at most 3,000 ms. Normal workers are never joined on the safety path if blocked. A forced KILL, missing reap or cleanup failure yields unknown even after a positive physical response. IsOS's default 20,000 ms EOS grace leaves 5,000 ms for pipe delivery and its own finalization; an explicitly configured shorter grace may produce unknown and must never be interpreted as success. IsOS's 30,000 ms deployment deadline and final 2,000 ms forced cleanup remain its separate outer bound. EOS must complete or classify unknown within its own 15,000 ms regardless of GUI, camera, socket or journal stalls.


**Related:** [IsOS dependent stop](../../isos/interfaces/core/dependent-stop.md) · [MuJoCo stop socket](https://github.com/Island-Team/isos-mujoco-sim/blob/master/docs/mujoco/interfaces/socket.md)
