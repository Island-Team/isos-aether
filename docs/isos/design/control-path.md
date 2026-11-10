# Mock arm control and monitor paths

The current 16-node [demo graph](../../../config/graph-pool/demo.nodes.json) declares two paths around one mock arm. Each edge uses registered ports and exact data types. IsOS saves and admits the graph; EI and EOS own their runtime stages.

The starter graph names the button **Forward: move arm** and the reader **Return: arm state**. Its explicit monitor settings refresh EI at 10 Hz and the UI at 5 Hz. On a fresh canvas, the control chain runs left to right in the upper lane and the observation chain runs right to left below it. The lane arrangement is UI layout only; dragging a node keeps the authored position locally. The two chains have no edge between them. EOS reads the same mock arm sampler locally for the return chain without creating another actuator route.

## Control

`isos/ui/button/trigger` → `ei/io/input/trigger` → `ei/cognition/intent/forwarder` → `ei/governance/safety/gate` → `eos/interface/ingress/trigger` → `eos/skill/demo/joint_delta` → `eos/dispatcher/arm/delta` → `eos/hardware/mock/arm`.

The button emits a run-fenced `button_activation@v0.2` without a movement target. EI validates and passes the activation. EOS creates a bounded `arm_joint_delta@v0.2` and admits it through the dispatcher before updating the mock adapter. The saved graph cannot create a second actuator route.

## Observation

The independent observation source `eos/hardware/mock/arm_observer` → `eos/dispatcher/arm/monitor` → `eos/skill/demo/arm_status` → `eos/interface/egress/arm_status` → `ei/io/input/arm_status` → `ei/cognition/state/arm_status` → `ei/governance/monitor/arm_status` → `isos/ui/monitor/arm`.

This path carries `arm_monitor_state@v0.2`. EOS samples and checks the arm locally at 100 Hz, lets EI poll the latest state at up to 20 Hz, and the UI refreshes at up to 5 Hz. EI and UI polling are independent of local EOS safety checks; a slow UI read does not hold the control path. EOS alone blocks movement on stale or invalid local samples. Stop and run changes clear the displayed positions.

The 2026-11-10 two-chain check traversed the saved pool graph and found 16 nodes, 14 edges, and two components of eight nodes. `ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --no-fail-fast` passed (284 unit and 18 integration tests); `cargo fmt --all -- --check` and `python3 scripts/check_doc_links.py --all` passed. Complete IsOS test output is under ignored `data/test/async-two-chain-20261110/`.

The [button event](../interfaces/ui/node-events.md), [monitor state](../interfaces/ui/node-monitor.md), and [data types](../interfaces/node/data-types.md) define the external boundaries.

[中文](../../../docs_cn/isos/design/control-path_cn.md)
