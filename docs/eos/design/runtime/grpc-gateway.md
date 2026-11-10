# R02 Rust gRPC gateway design

[中文](../../../../docs_cn/eos/design/runtime/grpc-gateway_cn.md) · [Owner contract](../../interfaces/grpc.md#r02-frozen-rust-subset) · [First-use replay](grpc-gateway-guide.md) · [Machine cases](../../../../tests/fixtures/r02_grpc_gateway/README.md)


## Composition and publication

Add a bounded `tonic`/`prost` server module to `aether-runtime` and generate Rust stubs from the existing checked-in protobuf files at build time. Pin generator versions and vendor or verify Google API imports; compare a descriptor set against the committed wire identities so a build cannot silently renumber fields. The private `aether-runtime` process remains selected by IsOS. CLI configuration adds a loopback gRPC port (default 16200) and optional owner-permissioned Unix path; these are distinct from the current private JSON-control socket. Reject wildcard binds unless an explicit deployment policy authorizes them. A duplicate or unsafe socket path fails before readiness.



The applied graph is read-only for this process; a new graph generation requires process restart, so the four startup discovery calls cannot straddle an in-process graph replacement. Live health and capability availability may change, but binding identity does not. If later hot refresh is added, it must publish a complete generation and provide a client retry boundary rather than returning a mixed set across separate RPCs.


## Reads and bounded streams


`CaptureFrames` resolves the admitted camera device ID once, checks robot name and live health, and takes 1 or 2 distinct fresh frames from an EOS-owned two-frame cache. Encode with the existing `VisionFrame::encode_jpeg` boundary or validated JPEG preview; verify SOI/EOI, dimensions, timestamp and distinct capture identity. Response ≤16 MiB and JPEG ≤8 MiB each. Honor deadline and cancellation at both cache wait and encode; a timeout cannot leave an unbounded task. An unavailable cache yields `UNAVAILABLE`, and an overfull response yields `RESOURCE_EXHAUSTED`.

## Operations and durable identity





Implement the release coordinator as a writer-preferred movement permit barrier owned by Dispatcher, separate from the journal mutex and emergency-stop latch. A movement acquires a permit before `begin_release`, checks inhibit/deadline under the barrier, writes `Releasing`, and holds the permit across the adapter call and its terminal or fenced physical boundary. RobotStop atomically closes new movement permits before persisting its inhibit; after the inhibit sync it waits for prior permits, including a movement paused after `Releasing` but before the adapter, to drain before fanout. No journal mutex is held while waiting or calling an adapter. A call returning `Accepted`, `Ambiguous` or an error after possible physical release leaves an unresolved generation even if its thread exits. The stop route must obtain adapter-proven quiescence or fence that generation at the physical write boundary before it can claim a terminal stop; otherwise it times out as `OUTCOME_UNKNOWN`, leaves inhibit durable, and never clears it using a single stopped observation. After a crash, reconstruct unresolved `Releasing` movements from the journal and establish an adapter epoch fence or equivalent proof before declaring movement available or clearing any RobotStop inhibit. The independent `SafetyEstop` path remains preemptive and does not wait on this barrier. A deterministic test pauses movement immediately after synced `Releasing` but before its adapter call, starts RobotStop, confirms no stop fanout or clear can overtake that movement, then releases it and confirms the ordered stop; a second branch leaves the movement ambiguous and checks inhibited unknown recovery.

Before Prepared, validate the maximum 64-target snapshot and reserve bounded result space using the 256-byte per-part reason cap; sanitize and truncate actual reasons on return. If serialization exceeds its bound before Releasing, return `RESOURCE_EXHAUSTED` with zero adapter calls and no inhibit transition. If a write or result serialization fails after a call may have occurred, preserve `OUTCOME_UNKNOWN` and the inhibit, never retry that ID, and retain only durably confirmed per-part results. Test a concurrent movement request during fanout, a pre-release injected overflow, and a post-possible-release injected overflow separately.

Translate `DispatchResult` to v2 Operation and canonical `primitive.ack` in one pure projection, including matching ID, primitive/version, accepted bit, status, sequence and bounded result. Do not invent `FAILED` or `TIMED_OUT` from undifferentiated movement outcomes; R02 stop commands have their own confirmed `FAILED` result and R03 may add distinguishable movement causes. `WatchOperation` sends the current record then at most one terminal update. A disconnected watch neither cancels nor retries the command; recovery is `GetOperation` with the same ID. `CancelOperation` uses Dispatcher cancellation and is command-targeted; it cannot stop a subsequent command on that device. The explicit stop capability and shutdown path handle motion clamping. The current synchronous Dispatcher may return terminal immediately; the watch contract still works for future pending adapters.

