# Browser primitive result routing

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/gateway</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [R02 Rust gRPC gateway design](grpc-gateway.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/gateway/browser-results_cn.md) · [Owner control contract](../../../isos/interfaces/server/eos.md#control--r07-authorized-browser-result) · [Primitive binding](../../interfaces.md#primitive_bindings)

The browser uses IsOS for the saved and active graph revision and the loopback HTTP bridge for a bounded `primitive_typed` request. EOS validates the run-scoped grant, one active target, typed payload, health, and revision before Dispatcher prepares a journal record. The browser cannot reach a raw actuator or another caller's result.

## Submission and result

Dispatcher writes a durable `Prepared` record before an admitted managed browser request can return `InProgress`. One bounded worker passes through the existing release gate, writes `Releasing`, calls the adapter once, and stores the terminal result. At most eight managed browser movements may occupy worker slots. Exact duplicate IDs read the retained result; changed signatures reject. Failed durable terminal writes make the ID unavailable, inhibit movement, and retain capacity until sealing or recovery. Recovery cancels a `Prepared` record without physical replay and classifies a `Releasing` record as uncertain.

The bridge validates the public length hint and parsed body against the 16 KiB bound. It writes the private byte count from its own compact JSON serialization; EOS checks that exact private count. This allows equivalent numeric JSON representations across JavaScript and Rust without relaxing the payload bound.


## Verification

Use a controlled-delay managed mock to observe `InProgress`, authorized pending read, and a later terminal read for one ID. Test invalid grant, Origin, target, revision, body, and duplicate signature; journal fault and recovery; Stop during pending release; and bridge outage. Run formatting, strict Clippy, serial workspace tests, the browser result fixture, and the bilingual link audit. Save full commands and output under `data/test/<run-id>/`.

<a id="inspect-a-browser-primitive-result"></a>

## Inspect a browser primitive result

After IsOS reports the deployment running, inspect its saved and active revision and EOS live readiness. In the EOS panel, select one **active primitive binding instance**. The panel shows its registered type and instance descriptions, one graph target, movement type and units, plus owner-reported target health. A draft binding or an unchanged display name alone does not authorize a command. Obtain EOS monotonic time through the loopback bridge, submit the typed `primitive_typed` payload with the active saved-byte revision and a fresh command ID, and keep the local `sending` state until EOS returns that ID and a status. The browser never supplies the private grant or an independent hardware target.

An `InProgress` acknowledgement follows a durable `Prepared` journal write and may arrive before adapter release. It proves only that EOS retained the command ID; EOS checks typed movement limits before this acknowledgement. If the submission response is lost, keep the ID and use the guarded read; never send the movement again. EOS admits at most eight such browser submissions at once, and a full lane gives a durable `Rejected` result for a valid new ID. Stop remains available through IsOS while a result is pending.


**Q01 EOS producer implemented; IsOS admission and selected-pin replay pending:** The bridge writes a sanitized result event under the current run after a valid private response. The published browser command ID remains 1–128 UTF-8 bytes. IsosUI-generated `ui-<UUID>` IDs are logged exactly as `correlation_id`; a different valid ID maps to the deterministic token in the [event contract](../../../isos/interfaces/server/eos.md#events). Query `GET /api/isos/v1/logs?view=current&source=bridge&correlation_id=<token>` while the ordinary rotated segment remains retained. The page's unfiltered anchor is the first ready or failure event; it is not the command result. A missing line or collector gap does not change the EOS journal result.

<a id="q01-browser-command-event-design"></a>

## Q01 browser command event design

The existing P06 sink permanently disables writes after a nonblocking error. The bridge does not retry a lost diagnostic in the same process or change the command response; Q01 full verification fails if its selected correlated line is absent.


**EOS producer implemented; selected IsOS admission and Q01 replay pending.** The bridge forwards typed browser control and authorized result follow-up to private runtime IPC, validates successful private result identity and status, and emits a sanitized event. IsOS's closed P06 bridge role still admits only lifecycle events on the current remote pin. The Q01 replay therefore cannot yet find the UI command ID in P07 current-run logs.

The bridge preserves the published 128-byte UTF-8 browser ID acceptance. It uses the exact command ID as P06 correlation when safe and a deterministic opaque token for other valid IDs, so existing same-run follow-up remains possible without logging arbitrary text. After a private response, it emits a result event only when `ok` is true, the result ID equals the request ID, and the status is in the published Dispatcher enum. It does not derive status from HTTP 200 or a request body. A bounded 1024-entry status cache suppresses repeated `InProgress` polls and terminal reads while retaining submit and first observed terminal transitions. The shared nonblocking event sink serializes bounded writes from client threads; a failed write is diagnostic loss, never command failure or a reason to replay movement. The [owner event contract](../../../isos/interfaces/server/eos.md#events) defines the token mapping, every field, and the safe response boundary.

Boundary tests use a private runtime stub for immediate terminal, `InProgress` then `Completed`, wrong ID/status, missing grant, same-run 128-byte/non-ASCII IDs, duplicate follow-up, sustained polling, cache eviction, concurrent calls, and sink failure. Selected-pin integration compares the raw bridge event with IsOS's normalized line, P07 page, UI-visible ID and one EOS journal outcome. The log proves correlation and admission of a sanitized owner event; the journal remains outcome authority.

### EOS producer verification, 2026-11-03

From this topic worktree, run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p aether-common -p aether-http-bridge -- --test-threads=1`, `cargo test --workspace -- --test-threads=1`, `cargo build --workspace`, `python3 scripts/check_doc_links.py --all`, and `python3 tests/fixtures/jsonl_events/check_events.py`. All passed; complete output is retained locally under `data/test/20261103-q01-browser-events/`. The focused bridge tests cover the UTF-8 command through private submit/follow-up, exact safe ID handling, cache concurrency/eviction, contradictory response rejection and sink loss. This verifies only the EOS producer; selected IsOS log admission and Q01 replay remain open.

Independent review found a concurrent ordering case: a terminal follow-up can complete before the original submit thread records its response. The cache now tracks the first submit and terminal observations separately and emits each once even in that order. A deterministic reversed-order test freezes this boundary. The sink-failure test counts attempted writes and checks that two successful private results still return HTTP 200 after the first diagnostic write fails. Rerun output is in `data/test/20261103-q01-browser-events-rereview/`.
