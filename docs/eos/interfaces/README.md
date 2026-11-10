# EOS interfaces

[中文](../../../docs_cn/eos/interfaces/README_cn.md) · [EOS](../README.md) · [Architecture](../design/architecture.md)

EOS owns the control path to the mock arm and its observation return path in graph format `0.2.0`. The [arm control contract](arm-control.md) defines activation, deltas, and limits. The [arm monitor contract](arm-monitor.md) defines the typed return path and private read route. IsOS owns the saved graph, package checks, Apply, and deployment status. The [owner lifecycle](owner-health.md) defines Run, Status, Stop, and worker failure. The [setup exchange](setup-v1.md) defines identity, catalog, validation, and rendering. The [quick start](../start/quick-start.md) follows the operator sequence.
