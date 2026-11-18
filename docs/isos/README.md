# IsOS guide

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Subfolders:
  - [core/](core/README.md)
  - [interfaces/](interfaces/README.md)
  - [start/](start/)

</details>
<!-- doc-navigation:end -->

IsOS owns graph `0.2.1`, exact package verification, Apply, and run identity. Its source version is `0.2.3`. It obtains contributed node declarations and resources from installed or linked package releases, then asks the selected EI and EOS cores to validate their own graph projections. Package names and hardware implementations do not select a special IsOS path. See the [node registry](../node/interfaces/registry.md#registry) and [package admission](interfaces/package/README.md#verification) contracts.


For an operator, follow this path:

- [First use](start/quick-start.md) starts a local home and shows the order for inspecting state, selecting verified nodes, validating, applying, and running.
- [Configuration](core/config.md) explains which file supplies defaults, how production overrides are selected, and where to fix startup errors.
- [Package selection](start/quick-start.md) explains how to install or link an exact release, inspect contributions, and choose stable resource IDs without depending on package-specific paths.
- [Publisher sign-in](interfaces/package/README.md#authentication) explains the separate Island-port CLI login used to publish packages; local graph control does not use this credential.
- [Run a deployment](start/quick-start.md) covers Validate, Apply, Run, Restart, Stop, and state inspection using the saved revision and owner outcome.
- [Troubleshooting](start/quick-start.md) locates package rejection, launch, readiness, log, and uncertain-stop failures using structured diagnostics.

For an author or contributor:

- [External interfaces](interfaces/README.md) contains the generic node, package, core, and IsosUI contracts that publishers and graph authors need.
- [Registered UI node families](core/server.md) describes the Vision monitor and UI-owned consumer boundary.
- [Architecture and design](core/architecture.md) explains module ownership, package admission, data selection, and the current button-to-mock-arm control and monitor paths without redefining wire fields.
- [Server design](core/server.md) explains the launcher and source boundaries; [HTTP](interfaces/api/http.md) and [manager](interfaces/manager.md) contracts define their wires.
- [Verification](core/server.md) records current evidence; [architecture targets](core/roadmap.md#target-architecture-not-implemented) separate future work from current contracts.

The current IsOS API uses `/api/isos/v1/*`; JSON payloads that require a schema use `"schema_version":"0.1.0"`. Package and core authors start at [external interfaces](interfaces/README.md).

[中文](../../docs_cn/isos/README_cn.md)
