# EI documentation

[中文](../../docs_cn/ei/README_cn.md) · [Documentation index](../README.md)

EI owns cognition, missions, evidence, its local safety gate, and its managed worker lifecycle. IsOS owns saved graphs and deployment orchestration; EOS owns hardware and final actuator admission. Start with the [architecture](design/architecture.md), then choose the boundary you need:

| Area | Read for |
| --- | --- |
| [Start](start/README.md) | Preparing, starting, inspecting, recovering, and stopping EI. |
| [Interfaces](interfaces/README.md) | Contracts consumed by IsOS, EOS, operators, and registered graph nodes. |
| [Design](design/architecture.md) | Current runtime flow, layer/group algorithms, and ownership. |

The Python implementation is in [`ei/`](../../ei/). Official memory and replay live under `data/ei/state/`; rebuildable cache, diagnostics, temporary files, and live sockets use `data/ei/cache/`, `log/`, `tmp/`, and `run/`. Back up state and user files; clear cache or temporary files only while EI is stopped. System installations use `/var/lib/island/ei/` and `/run/island/ei/`. `ISLAND_STATE`, `ISLAND_CACHE`, `ISLAND_LOG`, `ISLAND_RUN`, and `ISLAND_TMP` override their respective roots.
