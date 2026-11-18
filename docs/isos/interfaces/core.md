<a id="core"></a>
<a id="core--core-owner-contracts"></a>
# Core owner contracts

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Interfaces for nodes, packages, cores, and IsosUI](README.md)
  - [Shared contract catalog](contract-catalog.md)
  - [Owner implementation notes](manager.md)
- Subfolders:
  - [api/](api/README.md)
  - [node/](node/README.md)
  - [package/](package/README.md)
  - [server/](server/README.md)

</details>
<!-- doc-navigation:end -->

These pages are for cores that accept IsOS-admitted nodes. A core owns its execution, lifecycle adapter, health, and physical safety; IsOS verifies exact package bytes, registered placement, and a run fence before handoff. The exchanges below carry bounded identities and outcomes, not package-specific scene or device semantics.

- [Setup](core.md#core_setup) tells a core which verified graph and configuration it must read before becoming ready.
- [Handoff](core.md#core_handoff) binds a declared process, executable, resources, owner, run ID, and revision to one owner acceptance.
- [Events](core.md#core_events) reports owner state back to IsOS with enough identity to reject stale observations.
- [Control grants](core.md#core_control_grants) carries owner-scoped permission to issue control; a graph edge alone cannot grant actuation.
- [Dependent stop](core.md#core_dependent_stop) requires terminal evidence before IsOS treats stop as complete or permits replacement.

Private manager IPC and service configuration live in [development](manager.md).

[中文](../../../docs_cn/isos/interfaces/core_cn.md)

<a id="core_control_grants"></a>
<a id="core_control_grants--control-grant-boundary"></a>
## Control-grant boundary

[中文](../../../docs_cn/isos/interfaces/core_cn.md#core_control_grants) · [Design](../core/README.md)

The implemented IsOS grant bundle consists of private per-run files. The two control grant files are random, distinct secrets delivered only through reserved owner environment paths. The private `verifier.json` has the exact fields below. IsOS checks owner-only directory/file permissions, no symlink ancestors, the selected run and revision, and grant hashes. Stop and replacement revoke the files. EOS owns actuator command admission; a package contribution cannot create another route.

| `verifier.json` field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `schema_version` | integer / version | Current private verifier value `1`. | Required. |
| `run_id` | string / run identity | Exact bundle generation. | Required. |
| `active_revision` | lowercase 64-hex / SHA-256 | Binds saved graph bytes. | Required. |
| `operator_control_sha256` | lowercase 64-hex / grant bytes | Digest of second private grant. | Required. |

<a id="core_dependent_stop"></a>
<a id="core_dependent_stop--dependent-stop-contract"></a>
## Dependent stop contract

[中文](../../../docs_cn/isos/interfaces/core_cn.md#core_dependent_stop) · [Design](../core/README.md)

Current IsOS reads a strict, newline-terminated terminal-stop receipt from the selected owner pipe. The receipt uses schema `1`. IsOS binds the receipt to the selected `run_id`, owner PID, active revision, admitted package process node and expected parts. The owner proves its descendants and physical stop; IsOS independently proves its own direct process group exit. Missing, malformed, stale, late, or contradictory proof yields `unknown`.

| Receipt field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `schema_version`, `type` | integer `1`; string `eos_terminal_stop` / identity | Exact currently accepted owner receipt shape. | Required. |
| `run_id`, `active_revision` | strings / run and SHA-256 | Must match selected run and graph revision. | Required. |
| `eos_pid`, `backend_node` | integer; graph path / owner and node | Must match admitted owner process and backend node. | Required. |
| `backend_pid`, `backend_pgid` | integers or null / process IDs | Bind admitted descendant process group. | Required fields; values may be null. |
| `generation_id`, `stop_epoch` | strings or null / owner generation | Owner-issued stop correlation. | Required fields; values may be null. |
| `admission_closed`, `backend_group_drained`, `backend_reaped`, `sockets_removed` | booleans / stop facts | All required facts are checked before `stopped`. | Required. |
| `parts` | array / selected physical outcomes | Each entry gives `node_path`, registered hardware group as `kind`, and `stopped` or `unknown`; sorted paths must cover the admitted backend references exactly. | Required. |
| `outcome`, `reason_code` | string and optional string / terminal result | `unknown` remains conservative. | Required outcome. |

Restart cannot replace a run when physical stop remains uncertain.

<a id="core_dependent_stop--interrupted-three-core-owner-run"></a>
### Interrupted three-core owner run

The three-core demo keeps a private `default.owner-run.json` receipt with `run_id`, `revision`, `pending`, `attempted`, and `stopped`. The normal deployment Stop removes it only after both EI and EOS confirm Stop for that run. If the owners have lost the run, IsOS retains the receipt and reports the owners unavailable; monitor reads return 409 because the old run is no longer valid.

<a id="core_dependent_stop--local-emergency-stop"></a>
### Local emergency Stop

The CLI command `isos stop` is an emergency shutdown available even when the IsOS server is absent or the owners reject the saved run ID. It sends SIGTERM to local processes identified as belonging to the selected checkout or bound to its exact owner sockets, and escalates to SIGKILL if they remain. The saved run ID is not an admission condition. Once no matching processes remain, IsOS retires the stale owner-run receipt and records an emergency audit entry. The audit means that processes exited; it **does not prove physical actuator safety** or replace a terminal-stop receipt. The command prominently warns that it may interrupt active hardware. The operator must independently inspect hardware before another Run.

The operating system's same-user process permissions govern this command by default. An optional `config/isos/emergency-stop.toml` file may contain `code_sha256 = "<64 lowercase hex characters>"`; `ISOS_STOP_CODE_SHA256` overrides that value. When configured, the command requires `--code-stdin` and reads the code from standard input. A mismatch rejects the command before signaling. The code must not be supplied as a command-line argument, where shell history and process listings could expose it. This optional code supplements local OS permissions; it does not grant the command authority over other users' processes.

The workspace `kill.sh` script is the process-signaling implementation used by a source-checkout `isos stop` and also supports a dry-run preview. It selects this checkout's processes and owners bound to this workspace's exact `eos-owner.sock` or `ei-owner.sock`, even if those owners were launched from another checkout; their descendants are included. It reports the PIDs it signals. IsOS retires the receipt only after the script verifies that no matching processes remain. A private emergency-stop marker makes offline status report `stopped` with terminal outcome `emergency_signal_unproven`; a new Run clears the marker. The source launcher waits for the Stop stderr relay to finish before returning to the shell, so its final warning appears before the next prompt. Verify physical safety before relaunching.

An operator who has independently verified physical safety may run `isos physical-stop recover --run-id <exact-id> --attest-safe` after stopping the IsOS manager. The CLI requires an exact, regular, private owner-run receipt and no process candidates from its local scan. It writes a private audit record with the run ID, revision, receipt SHA-256, and `operator_attestation` before retiring the receipt. A missing attestation, running manager, changed receipt, wrong run ID, or possible process candidate rejects recovery. This command does not signal an owner or prove physical safety; the operator's attestation supplies that judgment. Run must still pass normal graph, owner, and generation admission afterward.

<a id="core_events"></a>
<a id="core_events--local-core-log-contract"></a>
## Local core log contract

[中文](../../../docs_cn/isos/interfaces/core_cn.md#core_events) · [Runtime logging](../core/logger.md)

Each core writes one private UTF-8 JSONL file and a readable same-name `.log` companion. Text lines use `IsOS-v0.2.3@<source>: [LEVEL][UTC timestamp]<message>` with embedded line breaks escaped. EOS and EI expose selected owner and worker records through their private `GET /v1/logs` owner socket route. IsOS validates those records and keeps bounded, durable per-core pools for the log API and workbench. IsOS never treats a log record as proof of process health or actuator state. The deployment status owner reply remains authoritative for health.

| Field | Type | Rule |
| --- | --- | --- |
| `timestamp` | RFC3339 UTC string | Required; producer time with `Z` suffix. |
| `level` | string | Required: `trace`, `debug`, `info`, `warn`, or `error`. |
| `source` | string | Required logical identity: `<core>.<layer>.<group>.<node>@aether-v0.2.3` for a graph node, or a core-qualified system component such as `isos.runtime.manager.diagnostic@aether-v0.2.3`. |
| `message` | string | Required bounded human diagnostic; no secrets or raw hardware payloads. |
| `event_code` | string | Optional stable machine code. |
| `run_id`, `correlation_id` | strings | Optional operation identity for joining status and HTTP errors. |
| `node_path` | string | Optional canonical graph path when the event belongs to a node. |
| `fields` | object | Optional bounded structured details. |

A representative record is:

```json
{
  "timestamp": "2026-11-09T09:24:22.340Z",
  "level": "info",
  "source": "eos.hardware.mock.adapter@aether-v0.2.3",
  "message": "Mock hardware is ready",
  "event_code": "mock_hardware_ready",
  "run_id": "r-example"
}
```

The producer paths are `data/isos/log/deployments/<deployment>/isos.jsonl`, `data/eos/log/<deployment>/eos.jsonl`, and `data/ei/log/<deployment>/ei.jsonl`. IsOS serves `isos.jsonl` and `owner-pool/{eos,ei}.jsonl` under its deployment log directory. It reads bounded complete lines, checks the source core against the pool, and merges accepted records by timestamp. Owner pages include important overview information and all warnings and errors; `warning` and `critical` are normalized to the shared `warn` and `error` levels before pooling. Malformed, oversized, or replaced-file records produce a gap diagnostic rather than a trusted event. Local files are owner-only; the loopback log API retains its Host and Origin checks and filters.

<a id="core_handoff"></a>
<a id="core_handoff--bound-package-process-handoff"></a>
## Bound package process handoff

[中文](../../../docs_cn/isos/interfaces/core_cn.md#core_handoff) · [Process declarations](package/README.md#process) · [Core index](core.md)

The `isos.package-process@v1` handoff is one bounded, private, run-bound JSON message. It is written only after exact-pin release, payload, registered type, resource, and owner-capability verification. The selected owner must recheck its adapter and package-specific semantics before readiness. An unknown owner or adapter rejects without child spawn.

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `contract` | exact string / identity | `isos.package-process@v1`. | Required. |
| `schema_version` | exact string / version | `0.1.0`. | Required. |
| `owner_core` | string / core ID | Registered owner for the selected node type. | Required. |
| `run_id`, `graph_revision` | bounded string; lowercase 64-hex / run and SHA-256 | Fence this handoff to one admitted run and saved graph. | Required. |
| `node_path`, `node_type` | strings / canonical path and registered type | Identify the selected process. | Required. |
| `package` | `{id,version}` / exact pin | Selects one verified release, never latest. | Required. |
| `entrypoint`, `resources` | verified paths and digests / files | Contained executable and selected resource files; no package-specific filename. | Required; resources may be empty. |
| `entrypoint_sha256` | lowercase 64-hex / file bytes | Pins the executable bytes selected after IsOS verifies the payload tree; the owner compares it before spawn. | Required. |
| `args` | array of at most 16 strings / arguments | Literal tokens or the bounded `${socket}` and `${resource.<setting>.path}` templates; no shell expansion. | Empty array. |
| `lifecycle_adapter` | registered string / owner capability | Owner must accept it before spawn. | Required. |
| `release_sha256`, `payload_sha256` | lowercase 64-hex / bytes | Bind accepted manifest and payload. | Required. |

Dependency order and readiness/stop conditions remain in the runtime plan and owner policy; they are not fields in this handoff. It is not a public endpoint, graph field, or actuator grant.

<a id="core_setup"></a>
<a id="core_setup--selected-core-setup-adapter"></a>
## Selected core setup adapter

[中文](../../../docs_cn/isos/interfaces/core_cn.md#core_setup) · [Cores HTTP](api/http.md#http_cores)

The implemented external owner setup exchange uses **schema 3**. IsOS selects the configured owner executable; a client cannot supply a path. Input is bounded to 1 MiB, output to 4 MiB, and the deadline is 10 seconds. `identity`, `catalog`, and `options` are read-only; `options` returns contextual selector choices from the owner projection; `validate` rechecks the draft before Apply; `render` previews bounded generated files. IsOS rejects wrong core, incompatible identity, malformed or oversized reply, escaping path, and timeout before publication. This external wire is independent of the IsOS-owned HTTP `"0.1.0"` schema.

When `isos serve` is active, it starts idle EOS and EI services on owner-only Unix sockets under the IsOS runtime directory. IsOS calls `POST /v1/identity`, `/v1/catalog`, `/v1/options`, `/v1/validate`, and `/v1/render` over HTTP/1.1 on those sockets. The transport request uses `schema_version:1`; the owner translates it to this schema 3 setup operation and returns a schema 3 reply. The request includes only that owner's graph projection, verified package store receipts, endpoints, and revision. A missing owner socket fails the operation; IsOS never selects another owner. Standalone offline setup checks still invoke the selected adapter directly.

The service health route is `GET /v1/health` and returns `schema_version:1`, the fixed core identity, and `state:"idle"`. The current service returns HTTP 501 for Apply, Run, Stop, status, and events; those operations still use the established IsOS process supervisor and run fences until owner worker handoff is implemented.

When an owner rejects a setup request with typed diagnostics, IsOS records up to 32 diagnostic `path` and `code` pairs under `setup_owner_diagnostic` in its local manager log. The graph response retains its stable public rejection code; the local log gives the operator the owner's exact node mismatch location without copying arbitrary diagnostic messages into the shared cursor.
For direct offline adapter invocation, exit status 2 is accepted only with a valid schema 3 reply where `ok` is false and diagnostics are present. Other nonzero exits and a success reply with exit status 2 fail closed.

```json
{
  "schema_version": 3,
  "ok": true,
  "diagnostics": [],
  "files": [],
  "identity": {
    "core": "eos",
    "setup_schema_version": 3,
    "product_version": "0.2.3",
    "graph_version": "0.2.1",
    "node_type_version": "0.2"
  }
}
```

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `schema_version` | integer / external owner schema | Exactly `3` for request and reply. | Required. |
| `op` | `identity`, `catalog`, `options`, `validate`, `render` / operation | Selects one bounded operation. | Required in request. |
| `core` | string / owner role | Must match selected configured owner. | Required in request. |
| `graph`, `revision` | JSON object and optional 64-hex / graph and SHA-256 | Validate/render draft and optional saved-byte fence. | Operation-specific. |
| `ok`, `diagnostics`, `files` | boolean and arrays / result | Closed reply; diagnostic entries have `path`, `code`, `message`; file entries have safe relative `path`, `content`. | Required in reply. |
| `identity`, `catalog` | JSON or null / owner metadata | Present according to operation; identity has `setup_schema_version:3`. | Optional. |

For `options`, `catalog.setting_options` is a bounded array of `{node_path, setting, selected, options}` groups. Each option has `value`, `label`, `description`, `available`, `reason_code`, and `reason`. The `node_path` belongs to the requested owner projection; `setting` names a declared owner selector or an owner-owned hardware field. The owner reports an unavailable selected value rather than silently replacing it. IsOS checks group identity, limits, and the selected draft value before exposing the result. This read-only preview does not authorize Apply: `validate` and `render` recheck the selected settings against the current owner inputs.

EOS may return adapter choices for an incomplete hardware instance. The missing selection is `null`; this operation does not validate a runnable profile. IsOS adds camera service and device choices from the exact verified package's node-set profiles to its public catalog response. A device choice may carry that verified instance `profile` for the editor to copy; the editor retains its own node identity and graph edges. Apply and Run validate the resulting instance anew.
