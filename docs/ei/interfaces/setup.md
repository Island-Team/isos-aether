# EI setup exchange

[中文](../../../docs_cn/ei/interfaces/setup.md) · [Reader identity](reader-features.md) · [Pipeline](../design/pipeline.md)

IsOS invokes the selected `ei-config` process with one bounded JSON request for `identity`, `catalog`, `options`, `validate`, or `render`. The managed setup envelope uses numeric `schema_version: 2` and `core: "ei"`. EI accepts an isolated `cores.ei` projection from graph format `0.2.0`. It validates the three activation nodes and three arm-monitor nodes, their placements, empty settings, and declared activation inputs. Legacy health and declaration-only node types are rejected. IsOS owns saved revisions, package verification, and publication of returned files.

`values` contains the IsOS editable EI listener and gateway connection. `mode` is exactly `isos`; `server` and `gateway` use their documented strict fields. A setup error has a stable JSON Pointer `path`, `code`, and bounded `message`, with no files. The setup operation does not start the EI runtime or connect to EOS.

```json
{
  "schema_version": 2,
  "op": "identity",
  "core": "ei"
}
```

The identity reports source product version `0.2.1`, graph version `0.2.0`, node type version `0.2`, exact supported sets, setup schema version `2`, and selected features. EI's required EOS Gateway, Capability, and Skill service versions are independent wire identities.

For a rejected EI node, both the one-shot setup command and the owner socket return the same typed diagnostic. Its `path` points to the rejected graph field, such as `/graph/cores/ei/layers/cognition/groups/intent/nodes/bad/node_type`; `code` distinguishes `unsupported_node_type_version` from `invalid_node_placement`. IsOS can record this bounded response in its setup diagnostics without relying on unadmitted owner stderr.

## Persistent owner control socket

`options` returns an empty `setting_options` list while node settings are static. `validate` and `render` validate the EI activation declarations and resolved endpoint inputs.

`ei-config owner --socket <absolute-path>` runs a persistent EI owner service on a Unix socket. Its socket file is mode `0600`; the parent directory must already exist. `GET /v1/health` returns `schema_version:1`, `core:"ei"`, setup schema version `2`, and state `idle`, `running`, or `exited` from the worker process. `POST /v1/identity`, `/v1/catalog`, `/v1/options`, `/v1/validate`, and `/v1/render` accept a JSON envelope with `schema_version:1`, `core:"ei"`, and `op` matching the route. The remaining fields match the setup exchange above. Replies retain setup schema version 2. Requests, headers, and responses are bounded to 1 MiB, 8 KiB, and 4 MiB respectively; each connection has a ten second timeout.

Production `POST /v1/run` accepts a transport-v1 envelope with `core:"ei"`, `op:"run"`, a bounded `run_id`, lowercase 64-hex `revision`, the owner-projected `graph`, `endpoints`, verified `packages`, and `launch:{sha256,args,env}`. EI revalidates the graph, verifies the selected `ei` worker bytes against `sha256`, and starts at most one worker per owner in its own process group. The health-only worker does not take the full runtime safety-state lock; the owner still fences the selected run and Stop. `POST /v1/status` and `/v1/stop` require the exact run identity. Stop signals and reaps the managed worker, then reports `outcome:"stopped"` and `worker_exited:true`. The owner service survives an IsOS client exit and accepts a new run after Stop, while rejecting stale prior run identities. `/v1/apply` returns 501. `POST /v1/activate` accepts numeric schema version 1, `core:"ei"`, `op:"activate"`, run ID, revision, source node, event ID, and `kind:"activate"`. It checks the active run, worker, and ordered EI path, then forwards to `endpoints.eos_owner.path` supplied by Run. Success has `ok:true`, the EOS command ID, `status:"accepted"`, and the fenced run, revision, and event ID; physical completion is asynchronous. `POST /v1/result` accepts the same version and fence with `op:"result"` and `event_id`, and relays EOS's correlated `accepted`, `running`, `succeeded`, `failed`, `cancelled`, or `outcome_unknown` result; only success carries six mock joint positions. A stale run returns 409; an invalid route returns 422; an unavailable worker or uncertain EOS outcome returns 503. Stop closes admission before worker exit. EI writes local JSONL diagnostics.

The EI owner injects `ISOS_RUN_ID` from the envelope and rejects a conflicting value in `launch.env`. A completed run identity remains fenced for the service lifetime. If a worker exits unexpectedly, Stop reaps it before another run can start.

`POST /v1/monitor` accepts numeric schema version 1, `core:"ei"`, `op:"monitor"`, run ID, and revision. Its success reply carries the unchanged EOS `observation` under the EI run/revision envelope. For a graph with monitor nodes, Run requires `endpoints.arm_monitor.joint_names`, the six ordered names from the applied EOS arm node. The monitor node’s `forward_rate_hz` defaults to 20 and admits 1–20 Hz; a run-owned background task polls EOS no faster than that rate, independent of IsOS readers. Reads return its latest validated snapshot while fresh (until the greater of three intervals or one second has elapsed since refresh). A stale run returns 409; invalid graph path returns 422; a lost worker, EOS owner, mismatched joint names, invalid sample, regressed sequence, or expired cache returns 503. Stop clears the task and cache before a new run can start.
