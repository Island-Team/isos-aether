# IsOS–EI server interface

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/server</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS owner server interfaces](README.md)
  - [IsOS–EOS server interface](eos.md)

</details>
<!-- doc-navigation:end -->

<a id="events"></a>
<a id="events--ei-local-diagnostics"></a>
## EI local diagnostics

[中文](../../../../docs_cn/isos/interfaces/server/ei_cn.md#events_cn) · [Logging design](../../../ei/core/runtime/runtime.md) · [First use](../../start/quick-start.md)

EI writes a readable `ei.log` companion and appends one JSON object per line to `data/ei/log/<deployment>/ei.jsonl` (`local` for standalone runs). `EI_LOG_DIR` may select an absolute private directory. IsOS reads the file locally to populate its log panel. The EI owner interface carries health and lifecycle replies only; it has no log transfer route.

Each record has `timestamp` (RFC 3339 UTC), `level` (`debug`, `info`, `warn`, or `error`), `source`, and `message`. A node source is `<core>.<layer>.<group>.<node>@aether-v0.2.3`. System code uses dotted file segments, for example `ei.owner_service.py@aether-v0.2.3`. Optional fields include `event_code`, `run_id`, `correlation_id`, and bounded, sanitized `fields`. The JSONL record itself has no display prefix. EI redacts secrets, URLs, prompts, media, and raw payloads before writing.

The owner records worker start, unexpected exit, and Stop in the same file. A log record is diagnostic evidence, not a command or proof of physical success. A failed log write does not change the owner lifecycle reply.

<a id="node"></a>
<a id="node--ei-activation-nodes"></a>
## EI activation nodes

[中文](../../../../docs_cn/isos/interfaces/server/ei_cn.md#node_cn) · [Setup](ei.md#setup) · [Architecture](../../../ei/core/architecture.md)

A node instance may omit `description` or set it to an empty string; EI retains it as optional display text.

The four Unix socket boundary stages register the `ei.owner.stage.v1` external implementation and execute in the EI owner. The intent forwarder and arm-state stage register `native_plugin` with the shared `native/libei_demo_stage.so` implementing `isos_node_v1`; the EI owner creates, starts, feeds, checks, stops, and destroys each instance for the selected run. Graph `0.2.1` registers three activation data-pass nodes and three arm-monitor data-pass nodes: `ei.io.input.trigger@v0.2` at `io/input`, `ei.cognition.intent.forwarder@v0.2` at `cognition/intent`, and `ei.governance.safety.gate@v0.2` at `governance/safety`. Each has an `activation` input and output carrying `button_activation@v0.2`, delivery mode `event`, and data class `event`. Node settings are empty. Legacy health and declaration-only node types are no longer admitted.

The input node receives the IsOS owner bridge through `unix_socket`, contract `{owner:isos,id:button-activation,version:1}`. Its output and the cognition node's input and output use `in_process`, contract `{owner:ei,id:activation-local,version:1}`. The safety node receives that local contract and sends through `unix_socket`, contract `{owner:ei,id:activation-to-eos,version:1}`. The graph's input references must form the ordered IsOS button → EI input → cognition → safety path. IsOS verifies the complete path through EOS.

EI carries `{event_id,kind:"activate"}` without choosing a movement. The owner delivers through the registered input and forwarder instances and checks the safety stage output before sending to EOS; the managed EI worker supplies process readiness. The safety stage checks the selected run, revision, worker liveness, graph path, and EOS owner endpoint before forwarding; it does not open an actuator route. EOS alone admits the arm command.

<a id="node--read-only-arm-monitor-return"></a>
### Read-only arm monitor return

The return chain registers `ei.io.input.arm_status@v0.2` at `io/input`, `ei.cognition.state.arm_status@v0.2` at `cognition/state`, and `ei.governance.monitor.arm_status@v0.2` at `governance/monitor`. Each carries `arm_monitor_state@v0.2` on an `observation` input and output with delivery mode `event` and data class `observation`. The monitor node has optional Advanced `forward_rate_hz` (default 20, range 1–20); the input and state nodes have empty settings. The EOS egress → EI input Unix socket contract is `{owner:eos,id:arm-monitor-to-ei,version:1}`. The EI input → state → monitor in-process contract is `{owner:ei,id:arm-monitor-local,version:1}`. The EI monitor → IsOS UI Unix socket contract is `{owner:ei,id:arm-monitor-to-isos,version:1}`.

The monitor setting descriptor declares `data_type: "number"`, matching the IsOS node catalog setting schema.

The EOS sample source is `eos/hardware/mock/arm_observer` for the independent observer adapter or `eos/hardware/mock/arm` for an existing applied graph. EI receives only its own graph projection, so it admits these two exact EOS source paths; the run, revision, and ordered joint-name fences still apply.

The EI owner reads EOS's private `POST /v1/monitor` endpoint in a run-owned background task at most `forward_rate_hz` times per second, independently of IsOS readers. It requires the selected run and revision, a live worker, the exact declared EOS egress → EI input → state → monitor path, and a non-regressing EOS sequence. Run receives `endpoints.arm_monitor.joint_names` from the applied EOS arm node; EI rejects observations with different ordered names. It validates the EOS response, then passes its `observation` object unchanged. `ready` carries six finite joint positions in radians; `degraded`, `stale`, and `stopped` carry null positions and an explicit reason. A stopped or stale run cannot read new data. The IsOS UI consumes the final EI observation at up to 5 Hz; the return chain never invokes movement admission.

<a id="node--package-contributed-ei-nodes"></a>
### Package-contributed EI nodes

A verified IsOS package may declare an EI node type with the existing `@v0.2` type identity, exact `ei/<layer>/<group>` placement, typed ports, settings, and an `implementation`. The EI catalog reads only package stores named in the setup request, rejects duplicate type IDs, and publishes explicit package `port_routes` and `bridge_adapters` claims. Matching port types alone do not prove a route. IsOS remains responsible for verifying the package release and complete store before invoking EI; EI rechecks the selected release identity and native library bytes.

On Linux, `native_plugin` uses `abi: "isos_node_v1"` and a package-relative `.so` listed with SHA-256 in `node_contributions.libraries`. EI requires the exported `isos_node_v1` table to report ABI 1, a complete table size, and non-null lifecycle and data callbacks. Run creates and starts the instance, checks health, and Stop requires `stop` to return zero before destroying it. `external` uses interface version `1` and a matching `node_contributions.processes` entry. EI starts that executable as its child with `ISOS_NODE_SOCKET`, `ISOS_RUN_ID`, and `ISOS_REVISION`; the executable serves newline-delimited JSON requests on its private Unix socket. Requests use `{version:1,op:"start"|"input"|"health"|"stop"}`; input supplies `data`, and success replies use `{version:1,ok:true}`. Input replies include an `outputs` array of JSON objects; health replies require `ready:true`; Stop replies require `quiesced:true`. Requests and replies are each limited to 1 MiB and time out after five seconds.

The EI owner's private `POST /v1/node-input` accepts `schema_version:1`, `core:"ei"`, `op:"node-input"`, exact `run_id` and `revision`, a selected EI node path, and a JSON `data` object. It returns the node's emitted `outputs` or rejects a stale run, unavailable node, failed input, or unready node. This owner-only data path does not forward actuator commands; the fixed EI safety stage and EOS dispatcher remain mandatory for actuation. Declared local routes between custom EI nodes deliver outputs synchronously by matching the emitted `port` to a target input. Custom EI↔EOS Unix socket routes are accepted only when both owners claim the exact package route and the applied graph declares the matching instance edge. The EI owner receives EOS data on its private `POST /v1/input` and forwards EI output to EOS `POST /v1/input`. Each request carries `schema_version:1`, `core`, `op:"input"`, `run_id`, `revision`, target `node` and `port`, source `source_node` and `source_port`, and a bounded JSON `payload`. EI checks the exact source link before delivery and returns fenced `outputs` with `{node,output:{port,payload}}`. The built-in activation-to-actuator route continues through EI `/v1/activate` and EOS dispatcher admission.

An absent required input is an IsOS graph warning. EI accepts the selected projection, but a disconnected activation path cannot pass the owner activation-path check and cannot forward to EOS. A disconnected monitor path starts no monitor polling thread.

<a id="reader_features"></a>
<a id="reader_features--ei-reader-identity"></a>
## EI reader identity

[中文](../../../../docs_cn/isos/interfaces/server/ei_cn.md#reader_features_cn) · [Setup exchange](ei.md#setup) · [Pipeline](../../../ei/core/runtime/pipeline.md)

The selected `ei-config` identity reports product `0.2.3`, graph `0.2.1`, node type `0.2`, `supported_graph_versions:["0.2.1"]`, `supported_node_type_versions:["0.2"]`, sorted graph-selectable stage types, and setup schema `3`. Direct-only cognition types appear in the catalog but are omitted from `supported_node_types` because normal graph setup rejects them. Its features include `setup-v3`, `catalog`, `custom-node-v1`, `graph-0.2-ei-projection`, `health-only-owner-v1`, `activation-owner-v1`, and `arm-monitor-owner-v1`.

The selected `ei` runtime probe reports `managed-config-v1`, `eos-gateway-client-v2`, `ei-evidence-scheduler-v1`. IsOS compares required features and selected executable bytes before Apply, Run, and Restart. A probe is read-only and cannot grant process or actuator authority.

EI rejects unsupported graph envelopes at `/graph/version`, unknown stage types, invalid placement or routing, and malformed configuration with bounded diagnostics. EOS service versions are checked as independent wire contracts.

<a id="safety_reset"></a>

[中文](../../../../docs_cn/isos/interfaces/server/ei_cn.md#safety_reset_cn) · [Events](ei.md#events) · [First use](../../start/quick-start.md)


<a id="safety_reset--owner-only-state"></a>
### Owner-only state


| Field | Type / unit | Purpose and validation |
| --- | --- | --- |
| `identities` | object alias → EOS robot ID / none | Nonempty affected alias set, keys 1–64 `[A-Za-z0-9_-]`, robot ID string at most 128 bytes; empty ID means unknown and cannot pass reset. |
| `episode_id` | 32 lowercase hex characters / latch episode | New for each engagement; durable marker, latch and one-shot authorization must match exactly. |
| `graph_revision` | lowercase 64-digit SHA-256 / selected graph | Binds latch and authorization to the selected IsOS graph; standalone latch tests use the all-zero sentinel. Operation checkpoints retain the selected revision or the `standalone` literal. |
| `nonce` | 32 lowercase hex characters / none | One-shot reset request identity; consumed on next startup and never reused. |
| `reserved_count`, `sequence` | positive 64-bit integers / reservation order | Monotonic history count and unique one-based index sequence; the index set must be complete and contiguous. |
| `operation_id` | string / EOS idempotency key | 1–64 `[A-Za-z0-9_-]`; created or validated before gate egress, persisted before Submit. |
| `alias`, `primitive`, `device_id`, `mission_id` | strings / owner identities | Exact recovery target; `mission_id` may be null. Primitive and device strings are at most 128 bytes; alias and mission ID use the opaque ID grammar. |
| `eos_robot_id` | nonempty string / EOS identity | Captured before operation submission; recovery lookup requires the same complete EOS robot identity. |
| `state`, `outcome` | closed strings / none | Operation `pending` or `terminal`; terminal outcome is `succeeded`, `failed`, `cancelled` or `rejected` and requires a matching EOS receipt. |



The private test-only `A03_EVIDENCE_DIR` hook copies raw owner state after each worker has exited, at `positive-before`, `positive-after`, `negative-before`, `negative-after`, and `negative-refused`. Each fresh snapshot directory is mode `0700`; copied files and its deterministic hash/length/mode manifest are mode `0600`. The exact inventory is `operations-history.json`, the selected operation's `.index` and `.json`, and, only for the two later negative snapshots, `safety-history.json` and `safety-latch.json`. The `negative-after` copy happens before reset authorization. The hook rejects missing, symlinked, wrong-type, wrong-owner or wrong-mode source files and never copies `safety-reset.json`, grant bytes or unrelated owner state. With the environment variable unset it creates no evidence directory and has no runtime effect.


Durable `safety-history.json` records prior engagement and the active episode. A missing latch file after an engaged marker fails startup closed. `operations-history.json` retains a monotonic `reserved_count`; it precedes each ID `.index` with its unique one-based `sequence` and payload. Indexes remain after terminal settlement. A missing checkpoint or index fails startup even when other operations remain. A terminal checkpoint cannot be rewritten. One-shot authorization is consumed immediately after the runtime lock, before config, discovery or mission checkpoint reads, even when that startup fails early. Managed launches restore valid mission checkpoints or fail closed even with `mission_preload=false`; prior incomplete work is never cleared.

<a id="setup"></a>
<a id="setup--ei-setup-exchange"></a>
## EI setup exchange

[中文](../../../../docs_cn/isos/interfaces/server/ei_cn.md#setup_cn) · [Reader identity](ei.md#reader_features) · [Pipeline](../../../ei/core/runtime/pipeline.md)

IsOS invokes the selected `ei-config` process with one bounded JSON request for `identity`, `catalog`, `options`, `validate`, or `render`. The managed setup envelope uses numeric `schema_version: 3` and `core: "ei"`. EI accepts an isolated `cores.ei` projection from graph format `0.2.1`. It validates the six built-in stage nodes and package-contributed EI node declarations against their registered placement, settings, ports, and implementation. Legacy health and declaration-only node types are rejected. IsOS owns saved revisions, package verification, and publication of returned files.

`values` contains the IsOS editable EI listener and gateway connection. `mode` is exactly `isos`; `server` and `gateway` use their documented strict fields. A setup error has a stable JSON Pointer `path`, `code`, and bounded `message`, with no files. The setup operation does not start the EI runtime or connect to EOS.

```json
{
  "schema_version": 3,
  "op": "identity",
  "core": "ei"
}
```

The identity reports source product version `0.2.3`, graph version `0.2.1`, node type version `0.2`, exact supported sets, setup schema version `3`, and selected features. EI's required EOS Gateway, Capability, and Skill service versions are independent wire identities.

For a valid `validate` request, EI returns no files. A rejected node returns a JSON Pointer to the exact field and a stable code; the message is diagnostic text.

```json
{
  "schema_version": 3,
  "ok": true,
  "diagnostics": [],
  "files": []
}
```

```json
{
  "schema_version": 3,
  "ok": false,
  "diagnostics": [
    {
      "path": "/graph/cores/ei/layers/cognition/groups/context/nodes/bad/node_type",
      "code": "unsupported_node_type_version",
      "message": "unsupported EI type 'ei.cognition.context.retired@v0.2'"
    }
  ],
  "files": []
}
```

For a rejected EI node, both the one-shot setup command and the owner socket return the same typed diagnostic. Its `path` points to the rejected graph field, such as `/graph/cores/ei/layers/cognition/groups/intent/nodes/bad/node_type`; `code` distinguishes `unsupported_node_type_version` from `invalid_node_placement`. IsOS can record this bounded response in its setup diagnostics without relying on unadmitted owner stderr.

<a id="setup--persistent-owner-control-socket"></a>
### Persistent owner control socket

`options` returns an empty `setting_options` list while node settings are static. `validate` and `render` validate the EI activation declarations and resolved endpoint inputs.

`ei-config owner --socket <absolute-path>` runs a persistent EI owner service on a Unix socket. Its socket file is mode `0600`; the parent directory must already exist. `GET /v1/health` needs no run request body and returns `schema_version:1`, `core:"ei"`, `setup_schema_version:3`, state `idle`, `running`, or `exited`, and nullable `run_id` and `revision`. Both identity fields are null while idle; a selected run retains its exact bounded ASCII run ID and lowercase 64-hex applied revision even if the worker exits. IsOS reads this small status snapshot before requesting the catalog or other inner configuration. It reattaches only when the selected identity matches its retained run receipt; mismatches are conflicts, and health alone is not Stop proof. `POST /v1/identity`, `/v1/catalog`, `/v1/options`, `/v1/validate`, and `/v1/render` accept a JSON envelope with `schema_version:1`, `core:"ei"`, and `op` matching the route. The remaining fields match the setup exchange above. Replies retain setup schema version 3. Requests, headers, and responses are bounded to 1 MiB, 8 KiB, and 4 MiB respectively; each connection has a ten second timeout.

`GET /v1/logs?after=<decimal byte offset>` uses the same private socket. It returns `{schema_version:1,core:"ei",next_cursor,entries}`, where `entries` contains original local `ei.jsonl` records and `next_cursor` is a nonnegative source-byte offset. A call reads at most 64 KiB and advances only across complete newline-terminated records. An absent file returns an empty page at `0`; an offset past file length restarts at `0`. A malformed offset returns HTTP 400 `invalid_log_cursor`; a file-read failure returns HTTP 503 `owner_logs_unavailable`. The page includes all warnings and errors plus selected owner and health-worker overview information, omitting routine lower-level information. IsOS retains a bounded durable copy for later workbench readers and resumes from this owner cursor after reattachment. Logs are diagnostics, not worker health or Stop proof.

Production `POST /v1/run` accepts a transport-v1 envelope with `core:"ei"`, `op:"run"`, a bounded `run_id`, lowercase 64-hex `revision`, the owner-projected `graph`, `endpoints`, verified `packages`, and `launch:{sha256,args,env}`. EI revalidates the graph, verifies the selected `ei` worker bytes against `sha256`, and starts at most one worker per owner in its own process group. The health-only worker does not take the full runtime safety-state lock; the owner still fences the selected run and Stop. `POST /v1/status` and `/v1/stop` require the exact run identity. Stop signals and reaps the managed worker, then reports `outcome:"stopped"` and `worker_exited:true`. The owner service survives an IsOS client exit and accepts a new run after Stop, while rejecting stale prior run identities. `/v1/apply` returns 501. `POST /v1/activate` accepts numeric schema version 1, `core:"ei"`, `op:"activate"`, run ID, revision, source node, event ID, and `kind:"activate"`. It checks the active run, worker, and ordered EI path, then forwards to `endpoints.eos_owner.path` supplied by Run. Success has `ok:true`, the EOS command ID, `status:"accepted"`, and the fenced run, revision, and event ID; physical completion is asynchronous. `POST /v1/result` accepts the same version and fence with `op:"result"` and `event_id`, and relays EOS's correlated `accepted`, `running`, `succeeded`, `failed`, `cancelled`, or `outcome_unknown` result; only success carries six mock joint positions. A stale run returns 409; an invalid route returns 422; an unavailable worker or uncertain EOS outcome returns 503. Stop closes admission before worker exit. EI writes local JSONL diagnostics.

The EI owner injects `ISOS_RUN_ID` from the envelope and rejects a conflicting value in `launch.env`. A completed run identity remains fenced for the service lifetime. If a worker exits unexpectedly, Stop reaps it before another run can start.

`POST /v1/monitor` accepts numeric schema version 1, `core:"ei"`, `op:"monitor"`, run ID, and revision. Its success reply carries the unchanged EOS `observation` under the EI run/revision envelope. For a graph with monitor nodes, Run requires `endpoints.arm_monitor.joint_names`, the six ordered names from the applied EOS arm node. The monitor node’s `forward_rate_hz` defaults to 20 and admits 1–20 Hz; a run-owned background task polls EOS no faster than that rate, independent of IsOS readers. Reads return its latest validated snapshot while fresh (until the greater of three intervals or one second has elapsed since refresh). A stale run returns 409; invalid graph path returns 422; a lost worker, EOS owner, mismatched joint names, invalid sample, regressed sequence, or expired cache returns 503. Stop clears the task and cache before a new run can start.

For package-contributed nodes, each verified `packages` reference supplies `id`, `version`, `path` to the IsOS package store, and optionally `payload_root` for linked package files. EI reads `release.json` from `path`, resolves `.so` libraries and external entrypoints under `payload_root` when present, and rejects path escapes or changed native digests. The catalog returns the six built-in stage types plus supported package types, explicit `port_routes`, and `bridge_adapters`. A package route is published only for local custom EI nodes with an implemented carrier; custom EI↔EOS Unix socket routes require matching owner claims and graph edges. The [node interface](ei.md#node) defines the private input and lifecycle messages.

The `ei_to_eos_edges` endpoint projection in Run contains IsOS-admitted `{source,output,target,input}` entries. EI binds only live custom source nodes to these entries and forwards their matching output through the EOS owner socket. The built-in safety-to-actuator route is excluded from this generic path.
