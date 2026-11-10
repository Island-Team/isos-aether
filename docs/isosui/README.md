# IsosUI documentation

[中文](../../docs_cn/isosui/README_cn.md) · [Documentation](../README.md)

IsosUI is the browser workbench for an IsOS deployment. It edits graph format `0.2.0` across the `isos`, `ei`, and `eos` cores and renders registered nodes in their declared layers. IsOS owns graph persistence, package admission, and process state. EOS owns live control and observation. The workbench presents those owner interfaces without defining another graph format.

## Reading map

- [First use](usage.md) covers opening and operating the workbench.
- [Architecture](architecture.md) separates current client boundaries from future UI targets.
- [Catalog projection](design.md) explains how registered types appear in the picker.
- [Workspace design](design.md) describes client state and owner boundaries; [browser results](design.md#r07-browser-result-state), [log cursors](design.md#p07-log-cursor-design), and [primitive control](design.md#u04-primitive-control-qualification) cover focused flows.
- [Interface overview](interfaces/README.md) maps graph loading, control, logs, and panel contracts.
- [Delivery record](todo.md) records completed UI slices and their evidence.

The source is in [`ui/`](../../ui/). Local and hosted builds are produced by the same TypeScript project.

## Client configuration

IsOS reads [`config/isosui.toml`](../../config/isosui.toml) for Web and terminal client defaults. `ISOSUI_CONFIG` selects another absolute TOML file. Process environment overrides TOML: `ISOSUI_DEFAULT_INTERFACE` (`web` or `tui`), `ISOSUI_LOCALE` (`en` or `cn`), `ISOSUI_STARTUP_TIMEOUT_MS` (1000–60000), and `ISOSUI_POLL_INTERVAL_MS` (500–10000). IsOS validates the resolved settings and serves non-secret values to both clients. Set persistent preferences in TOML; use environment variables for temporary overrides.
