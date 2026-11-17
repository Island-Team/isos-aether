# EI roadmap

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EI core](README.md)
  - [EI architecture](architecture.md)
- Subfolders:
  - [cognition/](cognition/)
  - [evidence/](evidence/)
  - [governance/](governance/)
  - [io/](io/)
  - [optional/](optional/)
  - [runtime/](runtime/)

</details>
<!-- doc-navigation:end -->

[Current architecture](architecture.md) · [中文](../../../docs_cn/ei/core/roadmap.md)

## Target architecture (not implemented)

**Target design — not implemented.** EI should turn heterogeneous evidence into a time-aware representation, choose reusable skills with explicit resource and risk constraints, and use a world model (WM) to predict, check, and revise short-horizon actions. This is a research and architecture method, not a claim that a particular Physical AI, WAM, or VLA model is already integrated. No model output gains authority to bypass EI's gate or EOS Dispatcher.

### 1. Unified Tensor Space for multimodal evidence

Use modality adapters for images, depth, language, proprioception, force/touch, and EOS semantic events. Each adapter emits typed tokens or tensors plus a modality ID, source and calibration version, coordinate frame, units, capture time, arrival time, sequence, validity mask, and uncertainty. Align a bounded time window around a decision timestamp; preserve asynchronous rates and missing modalities through masks rather than inventing synchronized values. Keep raw sensor evidence and its provenance available for audit, while the fused representation remains a derived, versioned model input. A shared space means a common temporal and semantic indexing scheme, not forcing every sensor into the same numeric shape.

Build the first representation with explicit feature adapters and late fusion. Measure alignment error, missing-sensor behavior, and downstream task value before learning a larger joint encoder. EOS supplies validated physical measurements and semantic catalogs; EI owns task meaning and its cognitive memory. Simulator-only truth may label an offline dataset but must be excluded from inference tensors. A changed calibration or encoder version invalidates cached embeddings for affected evidence.

### 2. VLA skill learning and routing

Separate a slow VLA planner from a skill router and short-horizon skill policies. The VLA proposes a grounded subgoal and expected outcome. The router compares candidate skills against observed state, entry conditions, embodiment capability, resource occupancy, model confidence, duration, and risk budget; it may abstain or ask for a new observation. A selected skill receives a typed, expiring goal with frame, target uncertainty, allowed joints or resources, speed and force bounds, and completion predicate. The skill emits bounded motion proposals or an EOS-supported primitive request, never raw unchecked actuator writes. EOS rechecks device state, freshness, limits, and Stop at its write boundary.

Start with a small fixed library: joint excursion, coordinated reach, hold, and compliant contact. Train generic motion and contact response on diverse configurations and disturbances before using held-out objects for evaluation. Compare a learned joint-space policy against tuned IK and impedance control under the same task, sensing, time, and risk budgets. Add concurrent skill sets only after single-skill transfer works; simultaneous skills need resource arbitration, shared-actuator composition, dwell rules, and preemption, with emergency Stop always taking precedence. A learned router may improve scheduling but cannot define physical safety limits.

### 3. WM in the decision loop

Treat the WM as a replaceable prediction service, whether implemented by a physics-aware model, WAM, or a simpler dynamics baseline. At a bounded decision cadence, EI gives it the current fused evidence, candidate skill, embodiment profile, and horizon. It returns predicted observations, contact or constraint events, uncertainty, and validity horizon. EI compares predictions with fresh EOS observations after execution; a discrepancy event triggers re-observation, skill adjustment, a different contact hypothesis, or Stop. Fast contact reflexes and e-stop stay in EOS because model inference latency and uncertainty cannot protect the immediate physical loop.

Run candidate WM versions in shadow mode first: record predictions without changing commands. Promote one only after calibrated uncertainty, latency, held-out outcome prediction, and recovery tests beat a no-WM and simple dynamics baseline. On timeout, stale input, out-of-distribution state, or disagreement, the router abstains or uses an explicitly qualified conservative behavior. The model must never treat its own predicted state as observed fact. EI stores decision and prediction provenance in cognitive memory; EOS retains authoritative physical evidence and IsOS retains deployment trace identity.

### Proposed cognition pipeline demo architecture


This is a proposed graph architecture, not the current starter or repository mixed example. The current graphs do not declare this full cognition side chain. A future graph could register thirteen demo EI node types with demo-style IDs (`ei.<layer>.<group>.<name>@v0.2`) without changing the production `isos.ei_*` stage contracts.

| Demo type | Placement | Graph inputs | Outputs |
|---|---|---|---|
| `ei.io.link.gateway@v0.2` | io / link | none; the owner injects `resource` | `resource` (`resource_bundle@v0.2`), `health` (`health_update@v0.2`) |
| `ei.evidence.perception.admission@v0.2` | evidence / perception | `resource` | `observations` (`observation_set@v0.2`) |
| `ei.cognition.context.builder@v0.2` | cognition / context | `observations` | `snapshot` (`context_snapshot@v0.2`) |
| `ei.cognition.missions.queue@v0.2` | cognition / missions | none; the owner injects `intent` | `selected` (`selected_mission@v0.2`), `lifecycle` |
| `ei.cognition.decision.planner@v0.2` | cognition / decision | `selected`, `snapshot` | `proposal` (`primitive_proposal@v0.2`), `plan_state` |
| `ei.governance.monitor.health@v0.2` | governance / monitor | `health` | `status`, `latch` |

The other seven proposed demo types would keep the activation and arm-monitor paths: `ei.io.input.trigger`, `ei.cognition.intent.context`, `ei.cognition.intent.forwarder`, `ei.governance.safety.gate`, `ei.io.input.arm_status`, `ei.cognition.state.arm_status`, and `ei.governance.monitor.arm_status`.

In this proposed design, the activation path would enter through the private `/v1/activate` route. The owner would evaluate the declared input, intent context, forwarder, and safety nodes in order and seed the cognition side chain from the same event: the gateway would receive a resource event and the missions queue would receive one demo mission keyed by the button event ID. The intent context node would admit the latest `arm_monitor_state@v0.2` observation into the shared perception queue and render a context snapshot with the product `ContextBuilder`, while emitting the activation unchanged. The forwarder would cache that snapshot on its `snapshot` input and forward the activation verbatim.

The safety gate would receive `proposal` on an optional input but never emit it. A planner proposal would therefore not alter the activation value that reaches EOS, and the owner would record the observed proposal only in the EI log. This would keep the EOS ingress contract identical to the activation MVP while still exercising a full perception → context → mission → decision → governance sequence. Demo EI routes would be declared as in-process links, and IsOS would admit EI→EI routed edges for these data types.

### Method and acceptance gates

1. Define versioned tensor and skill interfaces in EI's owning contracts, with rejection for missing frame, stale sample, unit mismatch, invalid mask, and unsupported embodiment.
2. Build a replayable fixed dataset from EOS evidence with train, validation, and held-out object families frozen before tuning. Account separately for simulation episodes, teacher time, real corrections, VLA calls, and compute.
3. Establish bounded single-joint, coordinated-motion, and hold checks; then compare learned skills with tuned IK/impedance on held-out contact tasks. Report success, force/collision violations, time, latency, and uncertainty intervals together.
4. Compare blind retry with evidence-driven revision under the same attempt budget, and run WM in shadow before intervention. Promote only if changed hypotheses improve outcomes without increasing physical risk.
5. Replay stale evidence, model failure, disagreement, Stop, and run replacement through the real EI–EOS boundary. A failed model or unavailable modality must not produce an unbounded command.

The separate Q03 vision-ingress qualification target needs a repeated stub stream; it does not replace the one-shot Q01 duplicate-generation byte and identity replay. Both target checks still need implementation and their own evidence.


## Design verification

On 2026-11-11, the documentation-only slice ran the bilingual link audit, IsOS documentation checker, `git diff --check HEAD`, and architecture heading-level parity check. Commands and complete outputs are recorded in [IsOS verification](../../isos/core/server.md); no EI runtime behavior is claimed by this design.
