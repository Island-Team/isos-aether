# EOS quick start

[中文](../../../docs_cn/eos/start/quick-start_cn.md) · [EOS](../README.md) · [Architecture](../design/architecture.md)

The starter graph has separate `isos`, `ei`, and `eos` cores. A button activation passes through three EI nodes, then EOS ingress, built-in skill, dispatcher, and mock arm. The [arm control contract](../interfaces/arm-control.md) defines the control nodes and six-joint command result. The [arm monitor contract](../interfaces/arm-monitor.md) defines the four-node independent observation return path.

## Build and setup

From the repository root, run `cargo build -p aether-runtime --bins` and `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`. IsOS uses the `aether-profile` owner socket for setup and its sibling `aether-runtime` binary for the worker. Start from the [IsOS starter graph](../../../crates/isos/templates/generic-graph.toml); verify the owner is configured and the graph passes validation.

## Apply, run, and control

Apply saves a revisioned graph, validates the EOS-only projection, and publishes its rendered graph. Run starts or reconnects to the EOS owner and launches the arm worker. Use `isos status --json` to inspect the EOS process and mock readiness. In IsosUI, click **Activate** on the button node. EOS first returns an accepted event ID and command ID; IsosUI then reads the correlated result until it reports six joint positions in radians or a terminal failure. EI refreshes the latest EOS arm observation independently of UI readers; its return path carries freshness, health, and positions into IsosUI. A second distinct click accumulates the same small delta; repeating the same event ID does not move again. Read the EOS logs for the current run. Restarting IsOS reconnects without stopping the EOS owner.

## Stop and recovery

Use `isos stop` and then `isos status --json`. Stop requires the matching run ID and revision; EOS confirms child exit before reporting success. An uncertain click must not be automatically resubmitted with a new event ID. If the child failed, Stop still reaps it. Run again only after the prior owner run has stopped.
