# Shared node interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: node/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Shared node graph interfaces](graph.md)
  - [Shared node registry interfaces](registry.md)
  - [Shared node type interfaces](type.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/node/interfaces/README_cn.md) · [IsOS node groups](../../isos/interfaces/node/README.md)

A registered node type fixes placement, settings, typed ports, implementation, and lifecycle. A saved instance supplies its identity, exact package pin, settings, and input references. IsOS combines the selected owner catalogs and verifies the graph before Apply. Graph format `0.2.1`, node type identities, package versions, and owner protocols are versioned independently.

- [Graph declarations](graph.md) cover profiles, instances, and edges.
- [Types and payloads](type.md) cover registered types, ports, data descriptors, and selectors.
- [Registry and admission](registry.md) cover package claims, owner capabilities, implementation forms, and route proof.

The authored schemas in `contract/node/` define registered type and saved instance shape. IsOS maintains shared payload and service contracts under `contract/`; EI and EOS consume the selected definitions read-only. A declared service binding carries its exact identity; IsOS resolves an endpoint only for the selected run. An optional field may be added without changing existing graph meaning; changing an established required field or wire identity needs an explicit version change.
