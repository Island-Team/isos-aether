# IsOS guide

IsOS (Island OS) is the product manager for graph `0.2.0`, which has `isos`, `ei`, and `eos` cores. Its source lives in `crates/isos/` at source version `0.2.1`. It stores the whole deployment graph, verifies exact package pins and declared resources, serves the local API, and coordinates persistent EI and EOS owner services. Each owner supervises its own worker. IsosUI source lives in `ui/`; MuJoCo remains an external verified package. A **registered type** defines a node's owning core, placement, settings, capabilities, ports, and lifecycle. An **instance** chooses that type and connects declared ports. The **owner** is the core that runs the node and validates its own payload semantics. A **run fence** ties events and handoffs to one active run and saved graph revision.

For an operator, follow this path:

- [First use](start/quick-start.md) starts a local home and shows the order for inspecting state, selecting verified nodes, validating, applying, and running.
- [Configuration](design/app/configuration.md) explains which file supplies defaults, how production overrides are selected, and where to fix startup errors.
- [Package selection](start/quick-start.md) explains how to install or link an exact release, inspect contributions, and choose stable resource IDs without depending on package-specific paths.
- [Publisher sign-in](interfaces/packages/authentication.md) explains the separate Island-port CLI login used to publish packages; local graph control does not use this credential.
- [Run a deployment](start/quick-start.md) covers Validate, Apply, Run, Restart, Stop, and state inspection using the saved revision and owner outcome.
- [Troubleshooting](start/quick-start.md) locates package rejection, launch, readiness, log, and uncertain-stop failures using structured diagnostics.

For an author or contributor:

- [External interfaces](interfaces/README.md) contains the generic node, package, core, and IsosUI contracts that publishers and graph authors need.
- [Registered node families](design/node/README.md) describes the currently registered IsOS-owned process family and explains UI-owned consumers.
- [Architecture and design](design/architecture.md) explains module ownership, package admission, data selection, and the current button-to-mock-arm control and monitor paths without redefining wire fields.
- [Server design](design/app/server.md) explains the launcher and source boundaries; [HTTP](interfaces/http/README.md) and [manager](interfaces/manager/README.md) contracts define their wires.
- [Verification](design/verification.md) records current evidence; [architecture targets](design/architecture.md#target-architecture-not-implemented) separate future work from current contracts.

The current IsOS API uses `/api/isos/v1/*`; JSON payloads that require a schema use `"schema_version":"0.1.0"`. Package and core authors start at [external interfaces](interfaces/README.md).

[中文](../../docs_cn/isos/README_cn.md)
