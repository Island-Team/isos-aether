# Current-run log cursors

[中文](../../../../docs_cn/isos/interfaces/manager/log-cursors_cn.md) · [Local core logs](../../design/runtime/logging.md)

The log API reads the three local core JSONL files and merges accepted records by timestamp. A continuation cursor is opaque to clients and signed by IsOS. It binds the source filter, run identity, file identities, and offsets; a forged token is rejected. A changed run or replaced file produces a gap diagnostic and starts at a safe boundary. Logs are observations, not proof of worker health; use deployment status for that.

| Field | Type / unit | Rule |
| --- | --- | --- |
| `cursor` | opaque string / continuation | Optional on the first page; bound to the filters and current run. |
| `limit` | integer / records | 1–200 per page. |
| `source` | string / core | `all`, `isos`, `eos`, or `ei`. |
| `level` | string / severity | `trace`, `debug`, `info`, `warn`, or `error`. |
| `node_path` | string / graph path | Optional exact match. |
| `correlation_id` | string / operation | Optional exact match. |

IsOS reads `data/isos/log/deployments/<deployment>/isos.jsonl`, `data/eos/log/<deployment>/eos.jsonl`, and `data/ei/log/<deployment>/ei.jsonl`. It uses bounded complete lines and rejects symlinks, malformed JSON, oversized records, and records whose source core does not match the file. EOS and EI never send log records through owner IPC. Launcher and install records use the same JSONL envelope in their own IsOS files; see the [logging guide](../../design/runtime/logging.md).
