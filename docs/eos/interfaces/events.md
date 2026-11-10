# EOS local JSONL log contract

[中文](../../../docs_cn/eos/interfaces/events_cn.md) · [EOS interfaces](README.md) · [Owner lifecycle](owner-health.md)

EOS writes one local JSONL file per deployment: `data/eos/log/<deployment>/eos.jsonl`. Standalone EOS uses `data/eos/log/local/eos.jsonl`. IsOS supplies absolute `EOS_LOG_DIR` to the persistent owner and reads this file for the log panel. Logs do not cross owner Run, Status, activation, or Stop replies. The current graph starts the owner and one arm worker; the worker applies admitted deltas to the mock arm.

Each UTF-8 line is one JSON object with these required fields:

| Field | Value |
| --- | --- |
| `timestamp` | UTC RFC 3339 timestamp |
| `level` | Lowercase `debug`, `info`, `warn`, or `error` |
| `source` | `eos.<layer>.<group>.<node>@aether-v0.2.1` for a node, or `eos.<source-file>@aether-v0.2.1` for a system process |
| `message` | Human-readable diagnostic text; no embedded unescaped newline |

Run-scoped records include `run_id`. The mock source is `eos.hardware.mock.arm@aether-v0.2.1`. Owner and worker system sources are `eos.aether_profile.owner@aether-v0.2.1` and `eos.aether_runtime.worker@aether-v0.2.1`. A Run or Stop failure includes the owner error code in `message`. The owner emits one worker-loss error when health or status first observes it. Logs provide diagnostics; owner lifecycle and activation replies determine readiness, command outcome, and Stop confirmation.

Retained bridge/config binaries write bounded diagnostics into this same file through `LocalCoreLog`. They do not write a second `events.jsonl` file. The current graph does not launch those binaries.
