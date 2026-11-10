# EOS local logs

[中文](../../../../docs_cn/eos/design/runtime/events_cn.md) · [Log contract](../../interfaces/events.md) · [troubleshooting](troubleshooting.md)

The EOS owner and arm worker append to one local JSONL file: `data/eos/log/<deployment>/eos.jsonl` for a managed deployment or `data/eos/log/local/eos.jsonl` standalone. IsOS reads the file for its log panel. The owner API returns lifecycle and activation results; log records remain local.

Each record has a UTC RFC 3339 timestamp, lowercase level, source, and message. Run-scoped records include the run ID. The mock arm source is `eos.hardware.mock.arm@aether-v0.2.1`; worker system events use `eos.aether_runtime.worker@aether-v0.2.1`. The owner records a worker loss once when observed and records failed Run or Stop outcomes. An activation reply, rather than a log line, proves the command's terminal result.

Older bridge and config-server binaries retained for callers write bounded records into this same file through `LocalCoreLog`. The current demo graph does not launch them. Verify local logs and owner/worker lifecycle with `cargo test -p aether-runtime --all-targets`; retain complete output under ignored `data/test/<run-id>/`.
