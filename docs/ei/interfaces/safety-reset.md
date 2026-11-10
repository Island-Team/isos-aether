
[中文](../../../docs_cn/ei/interfaces/safety-reset.md) · [Events](events.md) · [First use](../start/quick-start.md)


## Owner-only state


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

