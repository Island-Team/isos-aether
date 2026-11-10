# Browser primitive result routing

[中文](../../../../docs_cn/eos/design/runtime/browser-results_cn.md) · [Owner control contract](../../interfaces/control.md#r07-authorized-browser-result) · [Primitive binding](../../interfaces/primitive-bindings.md)

The browser uses IsOS for the saved and active graph revision and the loopback HTTP bridge for a bounded `primitive_typed` request. EOS validates the run-scoped grant, one active target, typed payload, health, and revision before Dispatcher prepares a journal record. The browser cannot reach a raw actuator or another caller's result.

## Submission and result

Dispatcher writes a durable `Prepared` record before an admitted managed browser request can return `InProgress`. One bounded worker passes through the existing release gate, writes `Releasing`, calls the adapter once, and stores the terminal result. At most eight managed browser movements may occupy worker slots. Exact duplicate IDs read the retained result; changed signatures reject. Failed durable terminal writes make the ID unavailable, inhibit movement, and retain capacity until sealing or recovery. Recovery cancels a `Prepared` record without physical replay and classifies a `Releasing` record as uncertain.

The bridge validates the public length hint and parsed body against the 16 KiB bound. It writes the private byte count from its own compact JSON serialization; EOS checks that exact private count. This allows equivalent numeric JSON representations across JavaScript and Rust without relaxing the payload bound.

The UI shows `Sending` until it receives a correlated ID and EOS status. It reads a pending `InProgress` result only while the same graph revision remains active. `Completed`, `Failed`, `Cancelled`, `Rejected`, and `OutcomeUnknown` are terminal displays. A bridge outage, changed revision, or stopped runtime displays an unavailable result. IsOS Stop remains available throughout; EOS fences movement and confirms physical Stop before terminating its direct MuJoCo child.

## Verification

Use a controlled-delay managed mock to observe `InProgress`, authorized pending read, and a later terminal read for one ID. Test invalid grant, Origin, target, revision, body, and duplicate signature; journal fault and recovery; Stop during pending release; and bridge outage. Run formatting, strict Clippy, serial workspace tests, the browser result fixture, and the bilingual link audit. Save full commands and output under `data/test/<run-id>/`.
