# EI documentation

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EI interfaces](interfaces.md)
- Subfolders:
  - [core/](core/README.md)
  - [interfaces/](interfaces/)

</details>
<!-- doc-navigation:end -->

[中文](../../docs_cn/ei/README_cn.md) · [Documentation index](../README.md)

EI owns cognition, missions, evidence, its local safety gate, and its managed worker lifecycle. IsOS owns saved graphs and deployment orchestration; EOS owns hardware and final actuator admission. Start with the [architecture](core/architecture.md), then choose the boundary you need:

| Area | Read for |
| --- | --- |
| [Start](../isos/start/quick-start.md) | Preparing, starting, inspecting, recovering, and stopping EI. |
| [Server API](interfaces/server.md) | Northbound WebSocket routes and payloads. |
| [Node groups](interfaces/node/README.md) | Registered `<layer>/<group>` node contracts. |
| [Interfaces](interfaces.md#readme) | Contracts consumed by IsOS, EOS, operators, and registered graph nodes. |
| [Design](core/README.md) | Current runtime flow, layer/group algorithms, and ownership. |

The Python implementation is in [`ei/`](../../ei). Official memory and replay live under `data/ei/state/`; rebuildable cache, diagnostics, temporary files, and live sockets use `data/ei/cache/`, `log/`, `tmp/`, and `run/`. Back up state and user files; clear cache or temporary files only while EI is stopped. System installations use `/var/lib/island/ei/` and `/run/island/ei/`. `ISLAND_STATE`, `ISLAND_CACHE`, `ISLAND_LOG`, `ISLAND_RUN`, and `ISLAND_TMP` override their respective roots.

The source boundary is `ei/core/` for cognition and lifecycle, `ei/io/` for external transport and wire models, `ei/contracts/` for read-only EI consumers of IsOS-owned schemas, `ei/generated/` for generated EOS bindings, `ei/lib/` for shared implementation helpers, and `ei/package/` for private external programs. Repository-level EI tooling lives in `scripts/ei/`. IsOS maintains shared schemas in `contract/`; EI validates against them and does not edit their types at runtime.
