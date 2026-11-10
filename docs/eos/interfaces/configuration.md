# EOS configuration reference

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](arm-control.md) for the active data and command path and [owner lifecycle](owner-health.md) for setup, Run, Status, and Stop.


[中文](../../../docs_cn/eos/interfaces/configuration_cn.md) · [Graph profile](hardware.md) · [IsOS graph](../../isos/interfaces/node/instance.md)

Author deployment graphs under root `config/deployments/`; IsOS saves and applies revisions from that tree. EOS has no independent TOML in `config/`, because its settings are owned by the applied graph. IsOS runtime settings live under `config/isos/`, EI component TOMLs under `config/ei/`, and IsosUI settings in `config/isosui.toml`.

EOS configuration comes from the applied IsOS graph and the selected verified packages. The graph format is `0.2.0` with separate `isos`, `ei`, and `eos` cores; EOS reads only its projection. Each registered `isos.eos_<layer>.<name>@v0.2` type declares placement, settings, ports, capabilities, and lifecycle. Instance values and typed edges select devices and routes. IsOS validates exact package pins and resources before starting EOS; EOS reads the projection without editing it.

## Layout

Start from [`crates/isos/templates/generic-graph.toml`](../../../crates/isos/templates/generic-graph.toml). IsOS stores revisions and renders the selected EOS profile. The runtime receives `--profile`, `--journal-dir`, and `--socket`. The HTTP bridge receives `--socket` and `--listen`; its listener must bind loopback. The persistent EOS owner supervises its runtime worker; IsOS orchestrates the deployment. The [process design](../design/runtime/process.md) describes startup and readiness.

### Where EOS files live

In source and portable installations, IsOS supplies owner-first paths below `data/eos/`. Explicit absolute path overrides remain valid. System installations keep durable EOS files below `/var/lib/island/eos/` and live sockets below `/run/island/eos/`.

| Folder | Contents | Cleanup |
| --- | --- | --- |
| `state/` | Official command journal and run records; short-lived package adapter host files during a run | Preserve and back up the journal. Host files are removed when their child stops. |
| `user/` | User-supplied custom store, simulator maps and library | Preserve and back up. |
| `attachments/` | Attached ASR or other run files | Preserve with their records. |
| `cache/` | Rebuildable runtime data, if used | Clear only while EOS is stopped. |
| `log/` | Raw simulator diagnostics for each deployment | Retain according to log policy. |
| `tmp/` | Short-lived work files, if used | Clear after EOS stops. |
| `run/` | Owner-only sockets and live process files | Stale entries are removed during startup. |

Only folders in use are created. IsOS writes its collected deployment events under `data/isos/log/`; repository test evidence stays under ignored `data/test/`.

## Binding files

A camera, base, or manipulator node names its adapter and typed settings in the graph. The mock CXX adapter supports local admission tests. The MuJoCo adapter uses a package-contributed process node and exact package/resource selections. EOS checks the resolved simulator socket and device bindings before creating adapters. The [hardware contract](hardware.md) defines identity, units, limits, health timing, observations, and movement modes.

## Safety and routes

A primitive binding output connects to exactly one compatible actuator input. Dispatcher checks active revision, target, capabilities, health, limits, deadline, and Stop state at final release. Camera routes are read-only. The bridge forwards only bounded admitted operations; see [control](control.md) and [Vision](vision.md).
