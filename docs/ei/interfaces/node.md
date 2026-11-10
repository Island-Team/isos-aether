# EI activation nodes

[中文](../../../docs_cn/ei/interfaces/node.md) · [Setup](setup.md) · [Architecture](../design/architecture.md)

Graph `0.2.0` registers three activation data-pass nodes and three arm-monitor data-pass nodes: `ei.io.input.trigger@v0.2` at `io/input`, `ei.cognition.intent.forwarder@v0.2` at `cognition/intent`, and `ei.governance.safety.gate@v0.2` at `governance/safety`. Each has an `activation` input and output carrying `button_activation@v0.2`, delivery mode `event`, and data class `event`. Node settings are empty. Legacy health and declaration-only node types are no longer admitted.

The input node receives the IsOS owner bridge through `unix_socket`, contract `{owner:isos,id:button-activation,version:1}`. Its output and the cognition node's input and output use `in_process`, contract `{owner:ei,id:activation-local,version:1}`. The safety node receives that local contract and sends through `unix_socket`, contract `{owner:ei,id:activation-to-eos,version:1}`. The graph's input references must form the ordered IsOS button → EI input → cognition → safety path. IsOS verifies the complete path through EOS.

EI carries `{event_id,kind:"activate"}` without choosing a movement. The owner executes these three data-pass stages for the MVP while the managed EI worker supplies readiness and lifecycle. The safety stage checks the selected run, revision, worker liveness, graph path, and EOS owner endpoint before forwarding; it does not open an actuator route. EOS alone admits the arm command.

## Read-only arm monitor return

The return chain registers `ei.io.input.arm_status@v0.2` at `io/input`, `ei.cognition.state.arm_status@v0.2` at `cognition/state`, and `ei.governance.monitor.arm_status@v0.2` at `governance/monitor`. Each carries `arm_monitor_state@v0.2` on an `observation` input and output with delivery mode `event` and data class `observation`. The monitor node has optional Advanced `forward_rate_hz` (default 20, range 1–20); the input and state nodes have empty settings. The EOS egress → EI input Unix socket contract is `{owner:eos,id:arm-monitor-to-ei,version:1}`. The EI input → state → monitor in-process contract is `{owner:ei,id:arm-monitor-local,version:1}`. The EI monitor → IsOS UI Unix socket contract is `{owner:ei,id:arm-monitor-to-isos,version:1}`.

The monitor setting descriptor declares `data_type: "number"`, matching the IsOS node catalog setting schema.

The EOS sample source is `eos/hardware/mock/arm_observer` for the independent observer adapter or `eos/hardware/mock/arm` for an existing applied graph. EI receives only its own graph projection, so it admits these two exact EOS source paths; the run, revision, and ordered joint-name fences still apply.

The EI owner reads EOS's private `POST /v1/monitor` endpoint in a run-owned background task at most `forward_rate_hz` times per second, independently of IsOS readers. It requires the selected run and revision, a live worker, the exact declared EOS egress → EI input → state → monitor path, and a non-regressing EOS sequence. Run receives `endpoints.arm_monitor.joint_names` from the applied EOS arm node; EI rejects observations with different ordered names. It validates the EOS response, then passes its `observation` object unchanged. `ready` carries six finite joint positions in radians; `degraded`, `stale`, and `stopped` carry null positions and an explicit reason. A stopped or stale run cannot read new data. The IsOS UI consumes the final EI observation at up to 5 Hz; the return chain never invokes movement admission.
