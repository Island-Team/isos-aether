# EI stage dataflow design

[中文](../../../docs_cn/ei/design/pipeline.md) · [Architecture](architecture.md) · [Current IsOS adapter](../interfaces/setup.md)

## Boundary and status

This page describes EI internal stage dataflow and validation. It does not define public IsOS graph ports or settings. The [IsOS node contract](../../isos/interfaces/node/README.md) is authoritative for registered type, placement, ports, and settings; the [EI node boundary](../interfaces/node.md) explains cross-owner routes. The value names below help trace EI runtime state.

## Shared stage value

```json
{
  "schema_version": "0.1.0",
  "event_id": "01JAZQ3D5Q4N8PK1Y3V6W0M2BS",
  "run_id": "run-20261101-01",
  "backend": "robot-a",
  "observed_at": "2026-11-01T08:00:00Z",
  "kind": "observation",
  "payload": {
    "resource_ref": "camera/front",
    "status": "available"
  }
}
```

| Field | Type / unit | Purpose | Owner |
|---|---|---|---|
| `event_id` / `run_id` | string | Correlate evidence and run | EI event producer |
| `backend` | string | EOS source identity | Link validates |
| `observed_at` | UTC time | Source observation time | Evidence source |
| `kind` / `payload` | enum / typed value | Observation class and content | Perception admission |

## Stage catalog and ports

These seven registered stages come from `ei/src/ei/stages.py`. Port names locate internal dataflow; IsOS remains authoritative for the editable graph contract.

| Registered type | Layer / group | Inputs | Outputs |
| --- | --- | --- | --- |
| `isos.ei_io.gateway@v0.2` | io / link | `gateway`, `admitted`, `emergency_stop` | `discovery`, `resource`, `result`, `health` |
| `isos.ei_evidence.admission@v0.2` | evidence / perception | `resource`, `hearing`, `result` | `observations` |
| `isos.ei_cognition.builder@v0.2` | cognition / context | `observations`, `body`, `memory` | `snapshot` |
| `isos.ei_cognition.queue@v0.2` | cognition / missions | `intent`, `result` | `selected`, `lifecycle` |
| `isos.ei_cognition.planner@v0.2` | cognition / decision | `selected`, `snapshot` | `proposal`, `plan_state` |
| `isos.ei_governance.gate@v0.2` | governance / safety | `proposal`, `registry`, `latch` | `admission`, `rejection` |
| `isos.ei_governance.health@v0.2` | governance / monitor | `health`, `progress`, `bounds` | `status`, `latch` |

The A01 local runtime rejects events from another run, future or invalid timestamps, and events older than 300 seconds at receipt. Resource admission requires nonempty `sample.text` and `sample.eos_robot_id`. Rejection diagnostics identify a stable code, source node, and event ID; a corrected event can reuse its ID after validation failure.

### Runtime values

| Value or setting | Type / unit | Purpose and default | Validation, owner, compatibility |
|---|---|---|---|
| `resource` | EOS typed resource reference + sample / source units | Evidence; no default. | EOS owns payload; link validates source and schema. Unknown references do not become capabilities. |
| `discovery` | complete EOS resource/capability generation / none | Link readiness and exact bindings; absent until a full generation completes. | Link owns atomic publication; partial or conflicting discovery fails closed. EOS owns underlying protobuf fields. |
| `body` | validated EOS body snapshot / source units | Optional hardware context; absent until discovery. | EOS owns physical descriptors; link verifies robot and resource identity before context reads it. |
| `observations` | bounded sequence/snapshot / source units | Evidence available to context; empty when valid but none seen. | Perception owns admission, ordering, coalescing, and omission count. New observation variants require admission policy. |
| `snapshot` | bounded context document / tokens | Model-readable state; empty optional sections allowed. | Context owns total token budget and deterministic ordering; does not contain raw secret or unbounded media. |
| `intent` / `selected` | mission document and revision / none | Operator request and selected work; absent when queue empty. | Missions owns identity, source, priority, version, and checkpoint; stale revision rejected. |
| `lifecycle` | mission ID, revision and state transition / none | Communicates mission changes; absent until a transition occurs. | Missions owns monotonic revisions and durable checkpoint; duplicate transition IDs are idempotent, conflicts fail. |
| `proposal` | typed planned primitive reference / none | Candidate action; no default. | Decision owns model parsing; must reference backend, device, primitive, version and arguments. It has no execution authority. |
| `plan_state` | plan ID, revision, phase and outcome / none | Reports planning progress; absent until a mission is selected. | Decision owns; invalid transitions and stale revision fail, and model timeout is not success. |
| `registry` | complete validated capability generation / none | Safety lookup; absent until discovery succeeds. | Link/registry own exact backend/device/primitive/version bindings; safety rejects missing or stale generations. |
| `admission` / `rejection` | gate decision with reason / none | Release or explain denial; one terminal decision per proposal. | Safety owns; admitted data uses exact current registry binding. Unknown/stale/invalid data fail closed. |
| `result` | EOS operation identity and terminal state / none | Feedback; no default. | EOS owns terminal operation semantics, link validates identity; timeout/cancel is not success. |
| `health` / `latch` | status enum and monotonic episode / none | Report degradation and emergency state; healthy/unlatched only after verified initialization. | Monitor owns; latched stop persists until explicit authorized reset, not process restart. |
| `progress` | mission/operation ID and monotonic elapsed time / seconds | Monitor input; absent when no work runs. | Decision/link own measurements; monitor bounds staleness and never infers completion from lack of updates. |
| `bounds` | finite measured value and configured limit / source unit | Optional monitor safety input; absent when no bound source is configured. | Monitor checks both unit and source identity; missing required limits fail closed. |
| `status` | health state, episode and reason / none | Operator-visible monitor projection; initialized as unavailable. | Monitor owns transitions; stale or partial backend state never maps to healthy. |
| `admitted` / `emergency_stop` | typed command reference / EOS units | Fixed runtime egress, never a user-editable graph edge; no default. | Safety owns workload admission and monitor alone owns emergency stop; link checks caller identity, EOS owns final admission. |

### Stage settings

| Stage / setting | Type / unit | Purpose and installed default | Validation and compatibility |
|---|---|---|---|
| `link.backend_alias` | string / none | Stable EOS route; required for each link. | Nonempty and unique across link instances; legacy single-backend mode normalizes to `default`. |
| `link.gateway` | endpoint reference / none | IsOS-resolved EOS gRPC endpoint; required in managed mode. | TCP host/port or absolute UDS path; rejects HTTP control/catalog endpoints. |
| `perception.capacity` | integer / records | Admission FIFO bound; 64. | `1..10000`; overflow counts and reports omissions. |
| `perception.history_per_kind` | integer / records per kind | Prompt history depth; 8. | `1..10000`; changing it does not reinterpret stored events. |
| `missions.mission_capacity` | integer / pending missions | Queue admission bound; 8. | `1..1024`; full queue rejects a new mission. |
| `decision.controller.model` | selector ID / none | Fast model binding; `gemma4-fast`. | Nonblank, present in `models.toml`, distinct from reasoner binding; unavailable saved ID remains visible but blocks Apply. |
| `decision.reasoner.model` | selector ID / none | Slow model binding; `gemma4-reasoning`. | Same validation as controller and must differ from it. |
| `decision.max_invalid_replies` | integer / completions | Consecutive invalid model replies before mission failure; 3. | At least 1; transport retry does not consume this count. |
| `decision.step_timeout_s` | finite number / seconds | Primitive step egress timeout; 30. | Greater than 0; timeout is not EOS completion. |
| `monitor.heartbeat_interval_s` | finite number / seconds | Backend health poll interval; 5. | Greater than 0; edge frequency does not replace this safety monitor cadence. |
| `monitor.failure_threshold` | integer / polls | Consecutive heartbeat failures before escalation; 3. | At least 1; reset only after verified recovery. |
| `monitor.stall_timeout_s` | finite number / seconds | Running mission without a fresh primitive result before deviation; installed default 150. | Greater than 0; model inference in flight is not treated as a primitive stall. |

## Admission, error, and compatibility behavior

**Normal:** A complete discovery generation makes link ready; evidence moves through perception and context; a selected mission produces a proposal; the gate either admits one exact primitive or emits a rejection; EOS terminal result returns as evidence. **Empty:** no mission or optional observation leaves stages idle without inventing data. **Malformed, duplicate, stale:** reject at the owning stage with stable event code, node ID, and correlation ID. **Unavailable and partial:** required discovery blocks readiness, optional model/memory sources degrade. **Timeout and cancellation:** preserve pending or failed state until terminal EOS evidence; never infer physical completion. **Restart:** load checkpoints, create a new run ID, rediscover EOS, and keep the emergency latch until authorized reset.

The source-edge scheduler may prioritize and cap evidence delivery, but gate admission always uses the current validated state. Overflow of a bounded observation or context queue is counted and logged; safety and terminal result events are never silently discarded. A graph cannot connect a model proposal directly to link or configure an arbitrary emergency sender. The only emergency bypass is monitor's dedicated stop egress.

## Interface verification

The Q03 owner qualification design fixes a measurement of the two existing A04 evidence routes. Its workload and metrics are test evidence, not additional stage ports, QoS settings, or safety routes. The selected installed-wheel and cross-owner replay remain release gates.

The [Q01 Vision evidence ingress](../interfaces/vision-evidence.md) is a separate process-owned, read-only inspection path. It adds no eighth stage, graph port, A04 `resource` image interpretation, prompt input, or actuator route. A gate frame never enters this stage scheduler in Q01.


## Activation data pass MVP

The IsOS-triggered demo path enters EI at the private owner `/v1/activate` route. The owner evaluates the declared input → intent → safety nodes in order against the selected graph and run. These stages carry the activation envelope unchanged; no EI stage chooses joint values. The existing managed worker continues to provide process readiness and lifecycle. The safety stage forwards only while that worker is live and the EOS owner socket for this Run is available. EOS then owns command admission and mock arm motion. The owner socket serializes Activate and Stop, so Stop closes new event admission before reaping the worker. An uncertain EOS transport outcome is surfaced to IsOS and is never retried by EI.

The 2026-11-09 EI activation boundary check used `uv sync --project ei --extra dev --extra vec --frozen`, `ei/.venv/bin/pytest ei/tests -q` (736 passed, 6 skipped), `ei/.venv/bin/ruff check ei/src/ei ei/tests` (passed), `.venv/bin/pyright src/ei` from `ei/` (0 errors), and `PYTHONPATH=src:libexec .venv/bin/lint-imports` from `ei/` (6 contracts kept). Complete output is in ignored `data/test/ei-arm-activation-20261109/`.

The bilingual documentation link audit for this change used `python3 scripts/check_doc_links.py --all` and checked 370 Markdown files with zero broken relative links. Complete output is in `data/test/ei-arm-activation-20261109/links.log`.

## Arm monitor return MVP

The EI owner handles `POST /v1/monitor` as a read-only pass through three registered nodes. The path check binds EOS egress `observation` to EI input, state, and monitor. The owner polls EOS no faster than the monitor node’s `forward_rate_hz` (default 20, range 1–20) and caches the last validated observation for that interval; it never invents a new sample or joint value. Run/revision mismatch, worker loss, missing graph route, invalid EOS data or configured joint-name mismatch, or sequence regression fail closed. Stop clears the cache. The final observation retains EOS identity, sequence, sample timestamp, six joint names, readiness/fault state, and positions only when ready.

The 2026-11-09 EI monitor check used `uv sync --project ei --extra dev --extra vec --frozen`, `ei/.venv/bin/pytest ei/tests -q` (743 passed), `ei/.venv/bin/ruff check ei/src/ei ei/tests` (passed), `.venv/bin/pyright src/ei` from `ei/` (0 errors), `PYTHONPATH=src:libexec .venv/bin/lint-imports` from `ei/` (6 contracts kept), and `python3 scripts/check_doc_links.py --all` (372 files, no broken links). Complete outputs are in ignored `data/test/ei-arm-monitor-20261109/`.

Configured monitor check, 2026-11-09: a custom six-name arm observation passes EI only when its ordered names match Run’s `endpoints.arm_monitor.joint_names`; the `forward_rate_hz` setting changes the cache interval and values above 20 are rejected. `ei/.venv/bin/pytest ei/tests -q` passed 745 tests; Ruff and Pyright passed. Complete outputs are in ignored `data/test/ei-arm-monitor-20261109/configured-monitor-*.log`.
