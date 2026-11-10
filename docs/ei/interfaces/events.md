# EI local diagnostics

[中文](../../../docs_cn/ei/interfaces/events.md) · [Logging design](../design/runtime/lib.md) · [First use](../start/quick-start.md)

EI appends one JSON object per line to `data/ei/log/<deployment>/ei.jsonl` (`local` for standalone runs). `EI_LOG_DIR` may select an absolute private directory. IsOS reads the file locally to populate its log panel. The EI owner interface carries health and lifecycle replies only; it has no log transfer route.

Each record has `timestamp` (RFC 3339 UTC), `level` (`debug`, `info`, `warn`, or `error`), `source`, and `message`. A node source is `<core>.<layer>.<group>.<node>@aether-v0.2.1`. System code uses dotted file segments, for example `ei.owner_service.py@aether-v0.2.1`. Optional fields include `event_code`, `run_id`, `correlation_id`, and bounded, sanitized `fields`. The JSONL record itself has no display prefix. EI redacts secrets, URLs, prompts, media, and raw payloads before writing.

The owner records worker start, unexpected exit, and Stop in the same file. A log record is diagnostic evidence, not a command or proof of physical success. A failed log write does not change the owner lifecycle reply.
