# EOS mock arm observation

[中文](../../../docs_cn/eos/interfaces/arm-monitor_cn.md) · [EOS interfaces](README.md) · [Arm control](arm-control.md)

The applied EOS projection contains an independent return path from the zero-input observation source `eos.hardware.mock.arm_observer@v0.2`, `eos.dispatcher.arm.monitor@v0.2`, `eos.skill.demo.arm_status@v0.2`, and `eos.interface.egress.arm_status@v0.2`. Each port is named `observation`, uses event delivery, and carries `arm_monitor_state@v0.2` as observation data. The three internal edges use in-process contract `{owner:eos,id:arm-monitor-local,version:1}`. Egress uses the Unix socket contract `{owner:eos,id:arm-monitor-to-ei,version:1}`. EOS requires these nodes and edges before Run.

The observer is a read-only software node in the EOS hardware layer. It has no graph input or actuator binding; it reads the sole mock-arm adapter's periodic sample, which advances independently of activation events. The mock arm remains the only actuator. The earlier input-bound `eos.hardware.mock.arm_output@v0.2` type remains valid for saved graphs using that route; its monitor payload retains the mock arm path in `source_node`.

By default, the mock samples and validates its six joints every 10 ms (100 Hz). Its optional Advanced node settings `sample_interval_ms` (1–1000 ms, default 10) and `freshness_timeout_ms` (2–5000 ms, default 50) must keep freshness at least twice the sampling interval. Its initial sample is six zeros; each admitted movement samples immediately after updating the joints. A sample older than the configured freshness timeout is `stale`; a nonfinite or out-of-bounds joint sample is `degraded`. Both block movement until a fresh valid sample restores `ready`. Stop closes movement admission and reports `stopped`. Only a `ready` observation includes positions.

## Payload

`arm_monitor_state@v0.2` is a JSON object with exactly these fields:

| Field | Type and rule |
| --- | --- |
| `run_id`, `revision` | Active run ID and 64-character lowercase revision. |
| `source_node` | Graph-bound observation source path, such as `eos/hardware/mock/arm_observer`. |
| `sequence` | Positive sample sequence within this worker run. |
| `sampled_at_monotonic_ns` | Nonnegative monotonic sample time in nanoseconds. |
| `joint_names` | Six unique names in graph order. |
| `joint_positions_rad` | Six finite radians when `ready`; `null` otherwise. |
| `state` | `ready`, `degraded`, `stale`, or `stopped`. |
| `reason_code` | `null` when ready; otherwise a reason such as `invalid_joint_sample`, `sample_timeout`, or `stopped`. |

## Private owner read

EI calls `POST /v1/monitor` on the mode `0600` EOS owner Unix socket. The exact request is `{ "schema_version":1, "core":"eos", "op":"monitor", "run_id":"run_1", "revision":"<64 lowercase hex>" }`. EOS requires the active run and revision, reads the latest worker sample without moving the arm, and returns `{ "schema_version":1, "core":"eos", "ok":true, "run_id":"run_1", "revision":"<revision>", "observation":{...} }`. EI may poll at up to 20 Hz; EOS safety sampling stays at 100 Hz independently. An unavailable worker returns `worker_unavailable` with HTTP 503. A stale run returns `stale_or_missing_run` with HTTP 422. Stop invalidates the read fence.
