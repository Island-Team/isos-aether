# IsOS product architecture

[中文](../../docs_cn/island/architecture_cn.md) · [Conventions](conventions.v1.4.0.md)

IsOS, EI, and EOS are three independent runtime cores. IsOS owns saved graph revisions, exact package verification, Apply, deployment run identity, and the local UI API. EI owns cognition and its safety gate. EOS owns dispatcher admission, hardware adapters, and the mock arm's local safety decision. The browser and terminal clients use IsOS.

## Current graph and runtime

Graph `0.2.0` uses `core → layer → group → node` and registered `<core>.<layer>.<group>.<node>@v0.2` types. Each core publishes and validates its types. IsOS admits the complete graph, then gives EI and EOS their own projections. The [starter graph](../../crates/isos/templates/generic-graph.toml) has 16 nodes and 15 typed edges: eight nodes carry a button activation to the mock six-joint arm; eight nodes carry an observation from that same arm back to the IsOS UI monitor. The observation output is read-only and creates no second actuator route.

| Core | Current responsibility | Runtime boundary |
| --- | --- | --- |
| IsOS | Graph, packages, Apply, run fence, local API, UI monitor | Manager and workbench |
| EI | Input, cognition, safety gate, monitor forwarding | Persistent owner service and worker |
| EOS | Ingress, skill, dispatcher, mock adapter, local observation checks | Persistent owner service and worker |

The button emits a run-fenced activation. EI passes it through input, cognition, and governance; EOS durably accepts it, maps it to a bounded six-joint delta, and admits it through the dispatcher. IsosUI reads the eventual command result by event ID. EOS samples the same mock arm locally at 100 Hz and checks freshness and validity before another movement. EI refreshes one latest checked snapshot at up to 20 Hz independently of readers; IsOS shares a validated snapshot for up to 200 ms among UI readers, which refresh at up to 5 Hz. Stop and run replacement clear prior-run monitor values. See the [node interface](../isos/interfaces/node/README.md), [EI setup](../ei/interfaces/setup.md), and [EOS owner health](../eos/interfaces/owner-health.md) for current contracts.

EI and EOS expose private Run, Status, and Stop interfaces. IsOS starts missing owner services, reconnects after a manager restart, and asks each owner to supervise its worker. Status is read independently from both owners. Explicit Stop requests both owners and records their confirmations. EOS stops admitted actuators before terminating a simulator child when one is present.

## Cross-owner contracts

Typed edges admit only the declared command and observation routes. Private owner exchanges carry run and revision identity; each owner validates its own payload and rejects stale or mismatched data. The graph describes the route, while owner services carry live values.

## Target architecture (not implemented)

Future algorithms may replace the demo's pass-through cognition and fixed joint-delta skill with owner-defined processing while preserving typed graph admission, run/revision fencing, and EOS-only actuator admission. A future push-based monitor transport should retain bounded latest-state delivery so UI or EI delay cannot block EOS's local 100 Hz checks. These are architecture targets, not current graph interfaces.
