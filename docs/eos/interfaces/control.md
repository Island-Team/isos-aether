# EOS control admission

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](arm-control.md) for the active data and command path and [owner lifecycle](owner-health.md) for setup, Run, Status, and Stop.


[中文](../../../docs_cn/eos/interfaces/control_cn.md) · [Primitive bindings](primitive-bindings.md) · [Rust skill contract](skills/rust.md)

The current graph has separate `isos`, `ei`, and `eos` cores and uses the owner activation route in [arm control](arm-control.md). The older loopback primitive-binding route admits movement through `isos.eos_dispatcher.primitive_base@v0.2` and `isos.eos_dispatcher.primitive_manipulator@v0.2` bindings. Each binding targets one compatible hardware node through a typed edge. Dispatcher is the only actuator route. Rust skill workers request primitives through the same broker; graph-authored action sequences are not part of this contract.

## Loopback API


The HTTP bridge listens at `127.0.0.1:16020` and forwards bounded JSON to EOS over its private Unix socket. Requests use `POST /api/eos/v2/control` with `Content-Type: application/json`; bodies are limited to 64 KiB. Browser calls require an approved `Origin`: `http://127.0.0.1:<ISOSUI_PORT>` and `http://localhost:<ISOSUI_PORT>` (default 16001), `http://127.0.0.1:<ISOS_HTTP_PORT>` (default 16000), `https://www.os-island.com`, or `https://os-island.com`. The bridge answers approved CORS/private-network preflights. The bridge allowlist excludes raw actuator, result and shutdown routes for browser and local HTTP callers. The EOS R03 private socket also rejects raw `manual`, `primitive`, `skill_execute`, `skill_result`, `dispatch`, `stop`, `manual_stop`, `cancel`, `result` and `skill_cancel` routes; internal owner-only `shutdown` remains. Every request must use the loopback bridge address.

For public `primitive_typed`, the browser supplies `header.payload_length_bytes` as a positive 1–16,384 byte compact UTF-8 payload length. The bridge parses the bounded public JSON, requires that field and a payload, rejects a declared or parsed payload length outside 1–16,384, then replaces the private header length with `serde_json`'s compact serialization length of the parsed payload before Unix IPC. The public declaration is a bounded hint rather than byte-exact proof across serializers; the private header is authoritative. The private runtime still requires an exact match with its own parsed-payload serialization and rejects malformed typed commands before Dispatcher. This bridge normalization is necessary because JavaScript and Rust render some finite JSON numbers differently, such as `0.000001` versus `1e-6`; the browser cannot use a wall-clock or string-character count to infer the private bytes. A missing, noninteger or out-of-range public length, or an absent payload, returns HTTP 400 with `{"ok":false,"error":"invalid typed request"}` before reading the grant. The bridge does not change typed payload values, graph binding, target, revision, command ID, deadline or grant checks.

## R07 managed browser submission

**R07 scoped implementation merged and qualified; evidence is recorded in the shared TODO.** For a managed, authorized `primitive_typed` request, EOS returns a correlated `DispatchResult` with `InProgress` only after the Dispatcher has durably reserved that exact command and accepted one bounded execution slot. No new public field or operation is added. The bridge's existing two-second private I/O bound and the browser's four-second timeout apply to this acknowledgement, not to the later physical call. EOS may return a terminal result instead if validation, admission or execution finishes before the reply; the browser must honor that exact status. A transport timeout or lost acknowledgement is **submission outcome unavailable**, never an invitation to resend movement. The browser retains the generated ID and may make only the authorized, read-only `result` request under the same current IsOS run and active revision. An `InProgress` acknowledgement proves durable reservation, not actuator release or physical completion.

EOS admits at most eight managed browser movement submissions that have returned or may return `InProgress`, counting workers waiting for the release gate and workers inside an adapter call. This limit is separate from private-socket client and safety-stop capacity; a busy movement lane cannot consume the safety path. For a new ID, EOS checks the exact active binding, target, typed payload limits, grant, run, revision and current deadline before reservation, obtains a slot, and durably writes `Prepared` before acknowledging `InProgress`. It rechecks live admission at the Dispatcher release gate, durably writes `Releasing` before one adapter call, and persists one terminal `Completed`, `Failed`, `Cancelled`, `Rejected` or `OutcomeUnknown` result before releasing the slot. If all eight slots are occupied, EOS records a terminal `Rejected` result for a valid new ID **in one atomic journal insert** containing the full command signature and managed identity, `phase:Terminal`, and `accepted:false`; there is no intermediate `Prepared` record, no worker and no adapter call. The Dispatcher implements this distinct crash-safe transition as `Journal::insert_terminal_rejection`, separate from the `Prepared` insert. If it cannot persist that rejection, EOS returns `{"ok":false,"error":"movement capacity unavailable"}` without acknowledging or releasing the command. An expired deadline or invalid admission is rejected before physical release. This asynchronous path still enters the existing Dispatcher; it does not create a second actuator route or change gRPC movement semantics.




If a queued worker decides **before** `Releasing` that its deadline expired, health failed or `Closing` began, but writing the terminal result fails, the durable record remains `Prepared`. EOS fences that ID against `Releasing` and every adapter call, latches movement inhibit for the live generation, keeps its slot until a durable seal or generation exit, and makes authorized result reads and identical duplicate submissions return `{"ok":false,"error":"result unavailable"}` instead of stale `InProgress`; a duplicate never creates a worker. It retries a durable terminal `Rejected` for expiry or health rejection, or `Cancelled` for Stop, under the journal lock, with Stop's `Cancelled` winning a race. A successful seal releases the slot and makes that terminal result readable. If the generation exits before sealing, normal journal recovery converts durable `Prepared` to terminal `Cancelled` without release before readiness; failure to persist recovery fails startup. Inject the pre-release terminal-write fault at deadline, health and Stop rejection, then test stale-read opacity, duplicate nonrelease, slot retention, durable retry and restart recovery.

## R07 authorized browser result



| Request field | Type | Unit | Purpose, default and validation |
| --- | --- | --- | --- |
| `op` | string | — | Required exact `result`; no default. |
| `command_id` | string | command identity | Required 1–128 UTF-8 bytes, matching the submitted typed header; no default. |
| `active_revision` | string | saved graph SHA-256 | Required 64 lowercase hex digits equal to EOS's current IsOS active revision; no default. |

```json
{
  "op": "result",
  "command_id": "move-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```


```json
{
  "ok": true,
  "result": {
    "command_id": "move-1",
    "status": "InProgress",
    "reason": null
  }
}
```


The public result route returns HTTP `400` with `{"ok":false,"error":"invalid result request"}` for malformed or extra public keys, and HTTP `403` with the existing bridge error for a forged `authority_grant`, invalid Host or disallowed Origin. Missing or unreadable bridge grant returns HTTP `503` with `{"ok":false,"error":"operator control unavailable"}` before IPC. A private grant rejected by EOS returns HTTP `200` with `{"ok":false,"error":"operator control grant required"}`; a stale revision returns HTTP `200` with `{"ok":false,"error":"stale active revision"}`; opaque IDs return HTTP `200` with `{"ok":false,"error":"unknown command ID"}`. The bridge returns HTTP `502` with `{"ok":false,"error":"runtime unavailable"}` for private IPC transport failure. These result-route errors are bounded fixed strings; no private grant or journal data is echoed.
