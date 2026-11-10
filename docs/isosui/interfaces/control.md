# Browser primitive control and result contract

[中文](../../../docs_cn/isosui/interfaces/control_cn.md) · [Design](../design.md) · [First use](../usage.md) · [EOS owner contract](../../eos/interfaces/control.md)

IsOS owns the saved graph, active revision, deployment run identity and Stop. EOS owns the active binding, admission, journal result and live status. IsosUI displays a browser-local send state; a correlated EOS `DispatchResult` establishes the command outcome. A graph edge declares one binding target. The current runtime admits MuJoCo movement only with its physical stop boundary.

## Typed boundary

The `src/services/eos/` client sends only `POST /api/isos/v1/owners/eos/control` through the validated IsOS loopback origin configured for local or hosted builds. IsOS forwards the bounded public request to the selected EOS bridge. It uses `credentials: omit`, JSON, no Authorization header, and the existing four-second client timeout. The browser sends no `authority_grant`, private `primitive_result` or raw Dispatcher operation. IsOS applies Host and approved-Origin checks; the EOS bridge retains its own admission and supplies its own run-scoped operator grant. The client provides an operator-clicked `primitive_typed` sender and a correlated result reader for supported active bindings. A browser `op: "result"` is read-only; the bridge rewrites it to its private operation.

The following fields describe the `base.velocity` request; the active binding supplies the corresponding registered arm identity and typed payload for an arm command.

| Send field | JSON type / unit | Purpose, default and validation |
| --- | --- | --- |
| `op` | literal `primitive_typed` / — | Required exact operation; no default. |
| `header.data_type_id`, `data_type_version`, `codec` | strings / registered identities | Required `base_movement`, `1.0`, `json_v1` for this base slice; no defaults. |
| `header.primitive_name`, `primitive_version` | strings / primitive identity | Required exact values from the active binding; this slice supports `base.velocity@1.0.0`. |
| `header.source_path`, `target_path` | strings / canonical graph node paths | Required exact active binding and its one graph-edge base target; never read from an editable target input. |
| `header.active_revision` | string / saved graph SHA-256 | Required current IsOS saved and active revision, 64 lowercase hexadecimal digits; no default. |
| `header.command_id` | string / command identity | Fresh per invocation, 1–128 UTF-8 bytes and EOS Dispatcher grammar; generate before send and retain after timeout. |
| `header.sequence` | unsigned integer / command order | Positive safe integer generated for this selected binding in the current run; no default. |
| `header.deadline_monotonic_ns` | unsigned integer / nanoseconds | EOS `time` reply plus bounded deadline duration, never browser wall time; reject a non-safe JSON integer or overflow before send. |
| `header.payload_length_bytes` | unsigned integer / UTF-8 bytes | Exact length of compact JSON serialization of the parsed payload, 1–16,384; do not use JavaScript string length for multibyte data. |
| `payload` | typed object / SI units | Exactly `body_velocity` with finite `x_m_s`, `y_m_s`, `yaw_rad_s` within active hardware limits and positive integer `valid_for_ms` within the lease limit. |

The public send contains exactly `op`, `header` and `payload`; the bridge adds the private grant after public checks. On a matching EOS `{"ok":true,"result":...}` response, use the same six-status decoder as the result reader. A missing/mismatched ID, unknown status or malformed response is a protocol error; a transport failure is submission outcome unavailable. The browser never retries a possibly accepted movement. Movement remains bound to a mock or a target qualified by the EOS stop boundary; the per-binding controls cover supported base and arm bindings.

The browser reports `payload_length_bytes` from `TextEncoder(JSON.stringify(payload))` in the public request. The EOS bridge validates the declared length and parsed payload bounds, then canonicalizes the length for its private `serde_json` runtime envelope. This preserves valid small SI values whose JavaScript and Rust compact number spellings differ.

| Client field | JSON type / unit | Purpose, default and validation |
| --- | --- | --- |
| `op` | literal `result` / — | Required exact operation; no default. |
| `command_id` | string / command identity | Required exact ID from the submitted `primitive_typed` header and correlated initial EOS reply, 1–128 UTF-8 bytes; no default. |
| `active_revision` | string / saved graph SHA-256 | Required 64 lowercase hexadecimal digits copied from the current IsOS active revision; it must equal the saved graph revision at each read; no default. |
| `result.command_id` | string / command identity | Must equal the locally tracked command ID; a mismatch is a protocol error, not an outcome. |
| `result.status` | enum / — | Required exact `Completed`, `Failed`, `Cancelled`, `Rejected`, `InProgress` or `OutcomeUnknown`; unknown status is a protocol error. |
| `result.reason` | string or null / UTF-8 text | EOS-reported explanation, at most 512 UTF-8 bytes; render as text, never HTML or a control decision. |

```json
{
  "op": "result",
  "command_id": "move-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

The result reader validates the request before fetch and accepts only `{"ok":true,"result":{"command_id":...,"status":...,"reason":...}}` with the matching ID and bounded fields. EOS bounds the whole success response to 4 KiB. A malformed success, unknown status, ID mismatch or oversized response is shown as a protocol error and never as `Completed`. Fixed owner errors remain errors: malformed request (`400 invalid result request`), invalid origin or Host (`403`), unavailable operator grant (`503 operator control unavailable`), private grant rejection (`operator control grant required`), stale revision (`stale active revision`), opaque other-run/role/ingress/kind/absent ID (`unknown command ID`), bounded result unavailable (`result unavailable`) and IPC outage (`502 runtime unavailable`). The UI does not infer which hidden cause produced `unknown command ID`.

## Run and revision fence

The command record key is `(run_id, active_revision, binding_path, command_id)`, where `run_id` is IsOS's exact current deployment-run ID, `active_revision` is the 64-hex digest, `binding_path` is the active graph node path and `command_id` is generated once per invocation. Only the current selected record can update its visible status. On tab/page unmount, binding selection change, run/revision change or Stop, cancel its pending browser read and ignore late replies by generation token. A late reply from another generation cannot overwrite a newer command. Do not persist command IDs or results as authority in browser storage. A current-run log link may prefill `correlation_id` only when an EOS owner event for the captured run and command provides a valid IsOS correlation ID of 1–64 ASCII letters, digits, `_` or `-`; otherwise show the command ID separately and open unfiltered current-run logs. Never assume `command_id` equals `correlation_id`.

## Visible states and Stop

`sending` is browser-local and must not pulse as success. An EOS reply with matching ID changes it to the exact owner status. `InProgress` is nonterminal; `Completed`, `Failed`, `Cancelled`, `Rejected` and `OutcomeUnknown` are terminal display states. `Failed` receives an explicit failure label and reason. `OutcomeUnknown` receives an explicit uncertainty label; it never triggers a retry of the movement command. If send times out or the bridge disconnects before a correlated reply, display **submission outcome unavailable** and retain the generated ID for investigation; retry only a guarded read after status/revision/run recovery, never the command. An opaque ID or outage during follow-up displays **result unavailable**, retaining the last known `InProgress` state without converting it to a terminal outcome.

The result reader permits at most one in-flight request per command, begins after a one-second interval, backs off repeated follow-up errors to at most 10 seconds, and stops automatic polling after 60 seconds or 12 attempts, whichever comes first. It pauses when the document is hidden and resumes only after rechecking IsOS run/revision; manual Refresh uses the same checks. The UI shows a paused `InProgress` record and a Refresh action rather than implying completion. Page navigation and Stop abort the read. These bounds apply only to read-only result calls, never to another movement submission.

The deployment **Stop** action stays visible and enabled through IsOS `/api/isos/v1/process-control` even when EOS is offline, the graph is read-only, the active revision differs, a result is unknown or the result client has paused. A local stop click is a pending intent until IsOS reports its governed terminal or unknown outcome; the UI does not equate a `Cancelled` command result with physical stop. Stop remains reachable from the workbench and its floating panels.

The Stop reply has `stopped` (reaped direct-child IDs) and a separate `terminal` object or null; it is not a `process-status` snapshot. The UI fences any outstanding EOS result read as soon as Stop is clicked and blocks another Invoke for that captured run. It shows pending until the IsOS reply arrives. It displays **Stop confirmed** only for a well-formed `terminal.outcome:"confirmed"` whose `terminal.run_id` matches the known run captured at the click, with null reason and confirmed child outcomes. A well-formed `unknown` shows its fixed `reason_code` as an alert. Null, absent, malformed, or mismatched terminal evidence is **Stop unconfirmed**; null with an authoritative `run_id_state:none` means there was no managed run to stop. A transport failure is **Stop response unavailable** and never triggers an automatic retry. After a reply, a fresh `GET /process-status` updates live run and revision separately; its failure cannot erase the Stop outcome. Home, Graph, and EOS Stop controls use the same interpretation.

Duplicate process IDs cannot establish a well-formed terminal. A null terminal with nonempty `stopped` is unconfirmed even on a never-run home. An explicit IsOS API error is shown as **Stop rejected by IsOS** with its owner message, separately from transport unavailability.

Every ID in `stopped` must also appear in `terminal.processes`; a confirmed terminal with a missing child record is unconfirmed.

## Per-binding node control

IsOS owns graph bytes, the saved and active SHA-256 revision, run ID and deployment Stop. EOS owns the active binding, target admission, live adapter status, journal and terminal result. IsosUI reads these owners and sends an operator-clicked command only through IsOS `POST /api/isos/v1/owners/eos/control`, which forwards to EOS admission. A graph wire declares the binding; it does not execute a command.

### Active binding and request

Before a send, IsosUI requires IsOS `run_id_state:known`, nonempty run ID, `state:running`, a 64-hex saved revision equal to the active revision and displayed graph revision, no draft edits or compatibility read-only state. It re-reads the saved graph and process status, verifies the same exact binding and target, and finishes the final IsOS run/revision fence before reading EOS process-local monotonic time. Immediately after that time read, it rechecks the latest local run, revision, binding, Stop, edit and abort state, then sends at most one `primitive_typed` POST with a fresh command ID. The four-second deadline starts from the last blocking owner read; an unavailable or expired send is never replayed with that ID. The header carries `data_type_id` (`base_movement` or `manipulator_movement`), `data_type_version:1.0`, exact primitive name/version, canonical binding/target paths, active revision, fresh 1–128-byte command ID, positive sequence, four-second monotonic deadline, UTF-8 JSON payload byte length (1–16384), and `codec:json_v1`. The payload is the validated typed movement object. The browser never supplies a grant or a private IPC operation.
