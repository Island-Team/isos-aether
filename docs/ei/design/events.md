
[中文](../../../docs_cn/ei/design/events.md) · [Owner interface](../interfaces/events.md) · [Fixture](../../../ei/tests/fixtures/jsonl_events/README.md)


## Mapping and ordering



An owner-local 256-key monotonic table suppresses only repeated noncritical diagnostics for up to 60 seconds and reports a bounded count. Startup, ready, state changes, first warning/failure, latch, unknown outcome, terminal and safety-stop failures bypass suppression. The current `_RepeatSuppressor` has a 4,096-entry local-file policy; the closed lifecycle emitter keeps its separate typed-field and suppression rules. On table eviction and shutdown, flush safe counts without blocking stop. A shared write failure increments a bounded owner counter; that failure does not change EOS actuation state.

## A03 durable recovery and shared policy revision




The independent A03 review tightened crash boundaries. `safety-history.json` precedes the latch payload; an engaged marker with a missing payload fails closed. `operations-history.json` and each per-ID index precede the operation payload, so a missing checkpoint is detected. Both latch and one-shot authorization bind an episode ID and selected graph revision; an operation reservation also stores EOS robot identity and graph revision, and terminal state cannot be rewritten. The authorization is consumed immediately after taking the runtime lock, so configuration, discovery or mission checkpoint failures cannot leave it armed. Managed default startup does not clear incomplete missions before restoration.

