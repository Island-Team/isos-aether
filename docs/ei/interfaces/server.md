<a id="runtime"></a>
<a id="runtime--northbound-websocket-api"></a>
# Northbound WebSocket API

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Subfolders:
  - [node/](node/README.md)

</details>
<!-- doc-navigation:end -->

## Table of contents

- [Routes](#runtime--routes)
- [Stream behavior](#runtime--stream-behavior)
- [Backend status](#runtime--backend-status)
- [Primitive catalog](#runtime--primitive-catalog)
- [Native skills](#runtime--native-skills)
- [Operator primitive invocation](#runtime--operator-primitive-invocation)
- [Hearing, missions, and perception](#runtime--hearing-missions-and-perception)
- [Mission commands](#runtime--mission-commands)


[中文](../../../docs_cn/ei/interfaces/server_cn.md#runtime_cn)


<a id="runtime--routes"></a>
## Routes

The route table lists suffixes under `/api/aether/ei`. For example, `/backend` means the full WebSocket path `/api/aether/ei/backend`. The health endpoint is `/api/aether/ei/health`; sandbox media uses the separate `/v1/sandbox` path.

| Route | Form | Purpose |
|---|---|---|
| `/backend` | stream | EOS connection plus compact robot/device state |
| `/chat` | command | operator hearing ingress |
| `/missions/stream` | stream | mission table projection |
| `/missions/command` | command | create or mutate pending operator missions |
| `/review` | command | review or promote mission history |
| `/context` | stream | latest bounded dynamic model context |
| `/model_io` | stream | model request, activity, and reply events |
| `/prompts` | stream | current static planner prompt |
| `/primitives` | stream | ready typed EOS primitive bindings |
| `/skills` | stream | v2 native-skill catalog and per-robot availability |
| `/skills/command` | command | exact skill lookup, start, execution lookup, and cancellation |
| `/skills/execution` | watch | ordered execution snapshots after a supplied cursor |
| `/primitive/invoke` | command | safety-gated operator primitive invocation |
| `/perceptions` | stream | admitted observation history |
| `/perception/world/state` | stream | fused `world.snapshot@1` |
| `/perception/world/detection` | stream | semantic `detection.snapshot@1` |
| `/perception/world/asr` | stream | compact `asr.snapshot@2` |


<a id="runtime--stream-behavior"></a>
## Stream behavior

Snapshot routes send the current value on connection and then replacements when the store version changes. A source that is not ready sends `{"error":"unavailable"}` and closes with WebSocket code 1013. World snapshots retain their EOS sequence, timestamp, typed identity, and source provenance; a fleet never silently merges two robots' state.

<a id="runtime--backend-status"></a>
## Backend status


```json
{
  "backends": [
    {
      "backend": "robot-a",
      "connection": "up",
      "endpoint": "127.0.0.1:16003",
      "eos_robot_id": "01a...",
      "robot_name": "charlie",
      "eos_started_at": "2026-09-23T00:00:00Z",
      "eos_version": "3.0.0",
      "robot_model": "charlie-v2",
      "robot_description": "mobile manipulator",
      "robot_state": "idle",
      "robot_health": "ready",
      "estop": false,
      "power_pct": 82.0,
      "devices": [
        {
          "device_id": "gripper-0",
          "device_class": "gripper",
          "part_id": "left",
          "description": "left gripper",
          "state": "idle",
          "health": "ready",
          "primitives": [
            "gripper.motion"
          ]
        }
      ],
      "last_heartbeat_at": "2026-09-23T00:00:02Z",
      "consecutive_failures": 0
    }
  ]
}
```

`eos_started_at`, not the stable robot ID, identifies an EOS restart. Only the listed compact body fields are public; driver/provider detail and raw telemetry remain private to EOS.

<a id="runtime--primitive-catalog"></a>
## Primitive catalog

`/primitives` projects each ready executable binding with its portable schema and deployment metadata:

```json
{
  "primitives": [
    {
      "name": "gripper.motion",
      "description": "Release one open or close motion",
      "device_description": "left workcell gripper",
      "version": "1.0.0",
      "robot_id": "robot-a",
      "device_id": "gripper-0",
      "eos_robot_id": "01a...",
      "operator": false,
      "scope": "device",
      "binding_kind": "direct",
      "health": "ready",
      "references": [],
      "absolute_frame": null,
      "dependencies": [],
      "accepted_media_types": [],
      "limits": {
        "force_n": {
          "minimum": 0.0,
          "maximum": 10.0
        }
      },
      "arguments_schema": {
        "type": "object"
      }
    }
  ],
  "count": 1
}
```

`version` is exact SemVer. `robot_id` is the stable configured alias, `eos_robot_id` is diagnostic provenance, and `device_id` is null only for robot-scoped bindings. The registry rejects unknown versions, unavailable bindings, invalid targets, schema violations, and out-of-range numeric arguments before egress.

<a id="runtime--native-skills"></a>
## Native skills

`/skills` publishes schema version 2 and reports availability plus validated skill metadata: configured robot alias, EOS identity, exact name/version, description, parameter/result schemas, exact primitive references, and resource templates. Transient EOS service unavailability replaces stale claims with a degraded snapshot; malformed descriptors reject the EOS generation.

`/skills/command` accepts one strict tagged request per frame. Unknown fields are rejected. `get_skill` requires `robot_id`, `name`, and `version`. `start` additionally requires a non-empty caller-generated `request_id`, object-valued `arguments`, and `timeout_s` greater than zero and no greater than 3600. `get_execution` and `cancel` require `robot_id` and `execution_id`. Successful replies contain either `descriptor` or a complete `execution`; stable failures are `invalid_json`, `invalid_request`, `robot_not_found`, `skill_not_found`, `execution_not_found`, `request_conflict`, `capacity_exhausted`, `unavailable`, `timeout`, and `internal_failure`.

```json
{
  "operation": "start",
  "robot_id": "robot-a",
  "name": "approach",
  "version": "1.2.0",
  "request_id": "ui-42",
  "arguments": {
    "distance_m": 1.5
  },
  "timeout_s": 30
}
```

An accepted start returns the complete initial execution snapshot:

```json
{
  "ok": true,
  "operation": "start",
  "descriptor": null,
  "execution": {
    "robot_id": "robot-a",
    "execution_id": "exec-1",
    "request_id": "ui-42",
    "name": "approach",
    "version": "1.2.0",
    "status": "queued",
    "sequence": 1,
    "cancellation_requested": false,
    "reason_code": null,
    "message": null,
    "result": null
  }
}
```

`/skills/execution` accepts exactly one initial `{"robot_id":"robot-a","execution_id":"...","after_sequence":4}` request. It proxies every retained complete snapshot with a greater sequence, follows live transitions, and closes after EOS closes the terminal watch. Reconnect with the last processed sequence to avoid duplicates. Disconnecting the WebSocket does not cancel execution; cancellation is explicit through `/skills/command`.

<a id="runtime--operator-primitive-invocation"></a>
## Operator primitive invocation

Send one exact target to `/primitive/invoke`:

```json
{
  "primitive": "gripper.motion",
  "robot_id": "robot-a",
  "device_id": "gripper-0",
  "arguments": {
    "action": "open"
  },
  "correlation_id": "ui-42"
}
```


A backend terminal answer is explicit:

```json
{
  "ok": true,
  "accepted": true,
  "command_id": "2aa1d76e-85b7-4fd0-8aef-8a4136b76555",
  "status": "succeeded",
  "sequence": 4,
  "result": {
    "status": "succeeded"
  },
  "reason": null,
  "reason_code": null,
  "correlation_id": "ui-42"
}
```

Only `status == "succeeded"` is success. `rejected`, `failed`, `cancelled`, `timed_out`, and `outcome_unknown` are terminal non-success states. Top-level `reason` and `reason_code` are populated only for backend admission rejection; admitted failure details remain inside `result`. Local request, target, policy, timeout, or egress failures use `ok: false` with `error` and optional `reason`.

Operator ingress uses the same registry, policy, safety gate, command lifecycle, and EOS egress as planner actions. `[primitives].expose_operator = false` hides operator-tier primitives from this route; enabling it never makes them planner-visible.

<a id="runtime--hearing-missions-and-perception"></a>
## Hearing, missions, and perception

`/chat` accepts `{"sentence":"..."}` and appends an attributed hearing observation; it does not directly create a mission. Mission streams expose current documents, priorities, cancellations, and terminal action outcomes. The three world streams expose replaceable typed snapshots without images, audio, prompts, transcripts beyond ASR text, or provider payloads.

A successful `/chat` frame acknowledges observation admission:

```json
{
  "ok": true,
  "observation_id": "observation-1",
  "timestamp": "2026-01-01T00:00:00+00:00",
  "correlation_id": "c-1"
}
```

`/review` promotes a concluded episodic plan into reference memory. For example, send `{"plan_id":"plan-1","correlation_id":"r-1"}`; the reply reports the recorded outcome and review time:

```json
{
  "ok": true,
  "plan_id": "plan-1",
  "outcome": "succeeded",
  "reviewed_at": "2026-01-01T00:00:00+00:00",
  "correlation_id": "r-1"
}
```

An unknown plan returns `{"ok":false,"error":"plan_not_found","correlation_id":"r-1"}`. `outcome` may override the stored judgment with only `effective` or `ineffective`; `note` is an optional operator annotation.

<a id="runtime--mission-commands"></a>
## Mission commands

`/missions/command` accepts strict `create`, `update`, `priority`, and `cancel` operations. The server owns mission UUID, source, origin, planning, and execution state. Mutations require the exact mission UUID and current `expected_revision`; persistence failure is reported before publication. Stable errors include `invalid_json`, `invalid_request`, `queue_full`, `mission_not_found`, `mission_revision_conflict`, `mission_not_pending`, `mission_protected`, `mission_started`, `mission_mutation_failed`, `storage_failure`, and `internal_failure`.

For example, create a pending operator mission with one JSON frame:

```json
{
  "operation": "create",
  "order": "inspect the bay",
  "priority_band": "high",
  "correlation_id": "ui-42"
}
```

The server returns the assigned UUID and first revision:

```json
{
  "ok": true,
  "operation": "create",
  "correlation_id": "ui-42",
  "mission": {
    "mission_id": "2aa1d76e-85b7-4fd0-8aef-8a4136b76555",
    "order": "inspect the bay",
    "state": "pending",
    "priority_band": "high",
    "revision": 1
  }
}
```

To change the priority, send the assigned `mission_id`, `expected_revision:1`, and `priority_band:"urgent"` with `operation:"priority"`. A stale revision returns this error; fetch the current mission before retrying:

```json
{
  "ok": false,
  "operation": "priority",
  "correlation_id": null,
  "error": "mission_revision_conflict",
  "reason": "mission revision is 1, expected 99"
}
```
