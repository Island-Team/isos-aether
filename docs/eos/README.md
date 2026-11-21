# EOS documentation

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EOS interfaces](interfaces.md)
  - [EOS interfaces](models.md)
- Subfolders:
  - [core/](core/README.md)
  - [interfaces/](interfaces/)

</details>
<!-- doc-navigation:end -->

EOS exposes one hardware control boundary for every admitted adapter. The selected package and owner catalog supply device implementation details; Dispatcher admission, typed observations, health, and Stop use the same owner contracts.

[中文](../../docs_cn/eos/README_cn.md) · [Documentation](../README.md) · [Conventions](../conventions.v2.0.0.md)


- [Start](../isos/start/quick-start.md): build, configure, launch, inspect, stop and troubleshoot EOS.
- [Server HTTP](interfaces/server.md): Loopback HTTP bridge and control route.
- [Node groups](interfaces/node/README.md): registered `<layer>/<group>` node contracts.
- [Interfaces](interfaces.md#readme): externally consumed payloads, control, adapter, and diagnostic contracts.
- [Architecture](core/architecture.md): ownership and lifecycle overview, with links to runtime and node-family design.
