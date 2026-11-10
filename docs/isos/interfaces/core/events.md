# Local core log contract

[中文](../../../../docs_cn/isos/interfaces/core/events_cn.md) · [Runtime logging](../../design/runtime/logging.md)

Each core writes one private UTF-8 JSONL file. IsOS reads these files locally for the log API and workbench; EI and EOS do not send log events through setup, Run, Status, or Stop interfaces. IsOS never treats a log record as proof of process health or actuator state. The deployment status owner reply remains authoritative for health.

| Field | Type | Rule |
| --- | --- | --- |
| `timestamp` | RFC3339 UTC string | Required; producer time with `Z` suffix. |
| `level` | string | Required: `trace`, `debug`, `info`, `warn`, or `error`. |
| `source` | string | Required logical identity: `<core>.<layer>.<group>.<node>@aether-v0.2.1` for a graph node, or a core-qualified system component such as `isos.runtime.manager.diagnostic@aether-v0.2.1`. |
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
  "source": "eos.hardware.mock.adapter@aether-v0.2.1",
  "message": "Mock hardware is ready",
  "event_code": "mock_hardware_ready",
  "run_id": "r-example"
}
```

The current paths are `data/isos/log/deployments/<deployment>/isos.jsonl`, `data/eos/log/<deployment>/eos.jsonl`, and `data/ei/log/<deployment>/ei.jsonl`. IsOS reads bounded complete lines, checks the source core against the file, and merges accepted records by timestamp. Malformed, oversized, or replaced-file records produce a gap diagnostic rather than a trusted event. Local files are owner-only; the loopback log API retains its Host and Origin checks and filters.
