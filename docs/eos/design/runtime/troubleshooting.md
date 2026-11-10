# Debug EOS

[中文](../../../../docs_cn/eos/design/runtime/troubleshooting_cn.md) · [Owner lifecycle](../../interfaces/owner-health.md) · [Architecture](../architecture.md)

For the current graph `0.2.0`, start with `./isos.sh status --json` and the active graph revision. EOS has one persistent `aether-profile` owner and one `aether-runtime` arm worker. IsOS reads `data/eos/log/<deployment>/eos.jsonl` for its log panel; standalone EOS uses `data/eos/log/local/eos.jsonl`. Absolute `EOS_LOG_DIR` selects another directory. The owner interface carries Run, Status, activation, and Stop results; logs are separate.

If **Activate** fails, compare the UI event ID and run revision with owner status. A stale run or revision is rejected before movement. `invalid_eos_graph` means the projection lacks the four-node chain or a declared edge. `joint_limit` means the next delta would exceed the configured mock arm limit or ±3.14 rad. `outcome_unknown` means the worker may have acted; do not issue a fresh event ID as an automatic retry. The activation result reports the six mock joint positions on success.

Each JSONL record has UTC `timestamp`, lowercase `level`, `source`, and `message`. System sources include `eos.aether_profile.owner@aether-v0.2.1` and `eos.aether_runtime.worker@aether-v0.2.1`; the mock node uses `eos.hardware.mock.arm@aether-v0.2.1`. An unavailable worker is reported once when observed. Compare logs and status for the same run; a log line alone never proves readiness.

Stop closes admission and waits for worker exit. An unconfirmed Stop remains a failed owner reply. The mock arm has no physical hardware connection. Save reproducible test output under ignored `data/test/<run-id>/`.
