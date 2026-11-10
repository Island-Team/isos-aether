# IsosUI architecture

[中文](../../docs_cn/isosui/architecture_cn.md) · [Reading map](README.md) · [Current design](design.md)

The current workbench renders IsOS graph format `0.2.0` and owner catalogs. It keeps canvas geometry in browser storage, graph settings in IsOS, and network calls in `ui/src/services/`. The UI does not own actuator admission. Its control and monitor surfaces consume separate owner routes: a user action sends one fenced command, while observation is sampled and forwarded independently. IsOS owns the run and revision fence; EOS decides whether movement is admitted. Closing a monitor panel does not stop local EOS sampling.

## Future targets

**Target design — not implemented.** The selector framework could cover additional verified package assets, including model weights, once an owning package contract declares their identity, validation, and lifecycle. The UI would display stable catalog IDs rather than accept arbitrary paths. This target does not imply model execution support in the current workbench.

**Target design — not implemented.** A future monitor view could expose per-stage timing and dropped-sample diagnostics for the asynchronous observation route. Such values need owner-defined counters and run-scoped semantics before the UI can present them as authoritative. The current UI shows the latest state, freshness, and fault/stopped status.
