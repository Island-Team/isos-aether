# IsosUI interfaces

[中文](../../../docs_cn/isosui/interfaces/README_cn.md) · [IsosUI overview](../README.md)

These contracts describe what the browser displays and sends through IsOS. IsOS owns graph revisions, runs, packages, and Stop; EOS owns command admission, health, and results. The browser keeps local presentation state and must recheck owner evidence before control.

| Contract | Scope |
| --- | --- |
| [Control and results](control.md) | Graph binding, typed primitive send, correlated result read, and Stop display. |
| [Graph loading](graph-loading.md) | Revision-matched graph and catalog selection, reload, and polling. |
| [Current-run logs](log-cursors.md) | Anchor, filters, cursors, gaps, and run boundaries. |
| [Workspace panels and Vision](workspace-panels.md) | Docking, persistence, visibility, Vision provenance, and frame display. |

Read the [first-use guide](../usage.md) for the operator sequence and the [design guide](../design.md) for client state and qualification evidence.
