# IsosUI 0.x delivery record

This record tracks IsosUI workbench delivery. First use shows owner status before the current run’s causal logs.

[中文](../../docs_cn/isosui/todo_cn.md) · [Design](design.md)

1. **Done — graph and catalog read.** Current graph and selected catalog revisions govern editing and Invoke. See U01 #40 and selected catalog #44.
2. **Done — U01 forms and selectors.** Metadata forms and searchable scene/device options retain unavailable IDs and block invalid Apply. See U01 #40.
3. **Done — U02 edges.** Exact IsOS diagnostics, routed priority/rate editing, and accessible labels for design-only and fixed routes are covered by U02 #41.
4. **Done — U04 node card.** Hover, focus and touch expose type and instance descriptions, ports, units and owner-reported health without blocking graph interaction. See U04 #42.
5. **Done — U03/U04 workbench.** Independent control, monitor, Vision and log panels preserve bounded layout and Stop authority; each supported active primitive binding has one graph-resolved target and a typed `primitive_typed` control with correlated result follow-up. Graph wires do not execute commands. See U04 #42, U03 #43 and refresh #45.
6. **Done — current-run logs.** Home uses IsOS JSONL cursors, pinned first-event anchors, gap notices and source filters while keeping command results distinct. See the [P07 UI contract](interfaces/log-cursors.md).
7. **Owner checks recorded.** English/Chinese first use, accessible interactions, Node 22/npm 10 tests and both local/hosted builds are recorded in the Q03 owner evidence. The sealed Q03 final-17 selected replay and Q04 final-5 merged-head replay passed their exact-input verifiers. Final-5 used a simulated hosted Origin on loopback; it does not establish live public HTTPS first use. The current unified product release is described in the [product release guide](../island/release.md).
