# EOS demo arm activation

[中文](../../../docs_cn/eos/interfaces/arm-control_cn.md) · [EOS interfaces](README.md) · [Hardware design](../design/runtime/hardware.md)

## Overview

EI calls the EOS owner on its private Unix socket with `POST /v1/activate`. The owner verifies the active run and forwards the event to its direct worker child. The EOS interface node passes the activation to the built-in demo skill; the skill emits a six-joint delta to Dispatcher; Dispatcher admits it to the mock arm adapter. No browser or package backend can call the adapter directly.

## Authentication

The owner socket is local and mode `0600`. IsOS controls launch, run ID, revision, and the EI-to-EOS connection. The EOS owner requires the exact active run ID and revision. The worker socket is also local and mode `0600`. Stop closes admission before child exit.

## Interfaces

`button_activation@v0.2` is a JSON object with required `event_id` (1–64 ASCII letters, digits, `_`, `-`, or `.`) and `kind` (`activate`). `arm_joint_delta@v0.2` is a JSON object with `joint_deltas_rad`, exactly six finite numbers in radians. The built-in skill emits `[0.05, -0.04, 0.03, -0.02, 0.01, -0.01]`. Each per-click delta is at most 0.25 rad in magnitude; Dispatcher rejects a resulting mock joint outside the graph-configured `max_joint_position_rad` or ±3.14 rad, whichever is tighter. The mock begins with six zeros and retains positions for the lifetime of one worker run. Its [observation return path](arm-monitor.md) samples continuously and closes movement admission on stale or invalid samples.

The four control graph nodes are `eos.interface.ingress.trigger@v0.2`, `eos.skill.demo.joint_delta@v0.2`, `eos.dispatcher.arm.delta@v0.2`, and `eos.hardware.mock.arm@v0.2`. The ingress input uses the `ei/activation-to-eos/1` Unix socket bridge; its output and skill input use `eos/activation-local/1` in process. The remaining edges use `eos/arm-delta/1` in process. All ports use event delivery; activation is event data, delta is command data. The mock node is a manipulator adapter with a registered `mock_arm` safety adapter and `joint_positions` capability. The setup catalog advertises the compiled `mock-arm` adapter family and `arm.delta@1.0.0` primitive.

## Routes

## POST /v1/activate

EI sends a JSON body with exact fields `schema_version` (`1`), `core` (`eos`), `op` (`activate`), `run_id`, `revision` (64 lowercase hexadecimal characters), `source_node` (`isos/ui/button/<id>`, with a 1–48 character ASCII ID), `event_id`, and `kind` (`activate`). EOS trusts EI to admit the source against the full graph and checks that it remains a canonical IsOS button path. Unknown or malformed fields are rejected. Each event ID is synchronously reserved in the run journal before `accepted` returns; one executor then performs movement. Repeating an ID in the same run returns the same acceptance receipt and does not move again. The response uses HTTP 200 on success and 422 on rejection. Transport uncertainty is reported as `outcome_unknown` and must not trigger an automatic retry with a new event ID.

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "activate",
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "source_node": "isos/ui/button/trigger",
  "event_id": "click-1",
  "kind": "activate"
}
```

```json
{
  "schema_version": 1,
  "core": "eos",
  "ok": true,
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1",
  "command_id": "arm-click-1",
  "status": "accepted"
}
```

Rejected requests have `ok:false` and an `error` code: `invalid_run_identity`, `stale_or_missing_run`, `invalid_activation`, `worker_unavailable`, `outcome_unknown`, `joint_limit`, or `event_capacity`. The resulting joint array is absent on rejection. The worker keeps at most 4096 reserved event IDs per run; when full, it still accepts duplicate known IDs and rejects new IDs without movement. The owner returns `503` for an unavailable worker or uncertain worker outcome, `404` for unknown routes, and `413` for bodies over 1 MiB; the worker limits forwarded messages to 4 KiB. This v1 route and the node/data `@v0.2` identities version independently.

## POST /v1/result

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "result",
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1"
}
```

EI reads a correlated result with exactly `schema_version` (`1`), `core` (`eos`), `op` (`result`), the current `run_id`, `revision`, and `event_id`. A successful reply includes those identities, `command_id`, and `status`: `accepted`, `running`, `succeeded`, `failed`, `cancelled`, or `outcome_unknown`. Only `succeeded` carries `joint_positions_rad`. An unknown ID returns `event_not_found`; Stop or an old run returns `stale_or_missing_run`. EOS monitoring samples independently of result reads, and slow readers get the latest observation.

If the worker exits, the owner reads the run journal for `/v1/result`. A reservation or running record without a terminal record reports `outcome_unknown`. On restart with the same run journal, the worker restores terminal records and positions, marks interrupted commands uncertain, and withholds further movement when any interrupted outcome is uncertain. The executor queue is bounded by the 4096-event run limit; duplicate IDs do not enter it again.
