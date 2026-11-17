# Interfaces for nodes, packages, cores, and IsosUI

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Shared contract catalog](contract-catalog.md)
  - [Core owner contracts](core.md)
  - [Owner implementation notes](manager.md)
- Subfolders:
  - [api/](api/README.md)
  - [node/](node/README.md)
  - [package/](package/README.md)
  - [server/](server/README.md)

</details>
<!-- doc-navigation:end -->

This is the external contract map for authors connecting nodes to IsOS. A **registered type** declares placement, settings, ports, and lifecycle; an **instance** selects that type and supplies values. A **core** owns execution. An **exact pin** identifies one verified package release. A **run fence** combines run identity and saved graph revision so a delayed event cannot affect a replacement run. IsOS currently admits graph declarations and packages through these contracts; runtime delivery remains the selected owner's responsibility.

- [IsOS nodes](node/README.md) defines IsOS UI node groups. [Shared node interfaces](../../node/interfaces/README.md) define type, instance, port, data type, selector, and edge rules for every owner.
- [Packages](package/README.md) defines exact releases, declared resources, processes, and verification. Publishers use it to contribute node types without package-specific IsOS logic.
- [Cores](core.md) defines IsOS owner exchanges for setup, process handoff, events, grants, and terminal stop.
- [Shared contract catalog](contract-catalog.md) classifies source contracts by purpose, owner, consumers, and domain independently of graph node paths.
- [EI server interface](server/ei.md) defines IsOS setup, graph, event, reader, and reset exchanges with EI. EI-owned client and node contracts are in [EI interfaces](../../ei/interfaces.md).
- [EOS server interface](server/eos.md) defines IsOS setup, lifecycle, control, package, and event exchanges with EOS. EOS-owned hardware, dispatcher, and model contracts are in [EOS interfaces](../../eos/interfaces.md).
- [API](api/README.md) defines local HTTP routes and Web/TUI client contracts.

- [Local HTTP API](api/http.md) defines client-visible routes, request admission, envelopes, and diagnostics.
- [Manager IPC](manager.md) defines the private CLI-to-manager wire and log cursor.

Implementation flow is in [server design](../core/server.md).

[中文](../../../docs_cn/isos/interfaces/README_cn.md)
