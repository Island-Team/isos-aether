# EI interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EI documentation](README.md)
- Subfolders:
  - [core/](core/README.md)
  - [interfaces/](interfaces/)

</details>
<!-- doc-navigation:end -->

<a id="readme"></a>
<a id="readme--ei-external-interfaces"></a>
## EI external interfaces

[中文](../../docs_cn/ei/interfaces_cn.md#readme_cn) · [EI architecture](core/architecture.md)

This directory describes what other owners and clients can consume. EI implementation and algorithms live under [design](core/architecture.md).

| Contract | Consumer and boundary |
| --- | --- |
| [Setup](../isos/interfaces/server/ei.md#setup) | IsOS `ei-config` one-shot and owner socket exchange. |
| [Runtime](interfaces/server.md#runtime) | Operator-facing EI runtime WebSocket API. |
| [Node](../isos/interfaces/server/ei.md#node) | IsOS graph node references and EI–EOS route ownership. |
| [Events](../isos/interfaces/server/ei.md#events), [safety reset](../isos/interfaces/server/ei.md#safety_reset), [reader features](../isos/interfaces/server/ei.md#reader_features) | External event and specialized owner exchanges. |

<a id="readme--node-layers-and-groups"></a>
### Node layers and groups

- [io](interfaces.md#readme--io)
- [evidence](interfaces.md#readme--evidence)
- [cognition](interfaces.md#readme--cognition)
- [governance](interfaces.md#readme--governance)

<a id="readme--io"></a>
### io

IO stages receive owner-bound inputs and connect EI to its declared peer boundaries. Input nodes admit trigger and arm-status events; the link group describes the gateway used for owner exchanges. Hardware control still passes through the declared safety and EOS dispatcher path.

Node groups in this layer:

- [input](interfaces/node/io/input.md#io_input)
- [link](interfaces/node/io/link.md#io_link)

<a id="readme--evidence"></a>
### evidence

Evidence admits perception data before cognition uses it. The perception group describes the admission stage and its boundaries; source payloads and external contracts remain defined by the EI interface pages.

Node groups in this layer:

- [perception](interfaces/node/evidence/perception.md#evidence_perception)

<a id="readme--cognition"></a>
### cognition

Cognition turns admitted observations and mission state into bounded context, decisions, and intent. The stage nodes in these groups pass events through the selected EI projection; they do not own actuator admission. Context assembly, planning, mission queueing, and arm-state projection are described by their group pages.

Node groups in this layer:

- [context](interfaces/node/cognition/context.md#cognition_context)
- [decision](interfaces/node/cognition/decision.md#cognition_decision)
- [intent](interfaces/node/cognition/intent.md#cognition_intent)
- [missions](interfaces/node/cognition/missions.md#cognition_missions)
- [state](interfaces/node/cognition/state.md#cognition_state)
- [isolated cognition library](interfaces.md#cognition_isolated_library)

<a id="readme--governance"></a>
### governance

Governance applies EI safety and observation policy after the preceding stages. The safety gate is the final EI check before an activation reaches EOS; monitor stages handle read-only health and arm-state observations.

Node groups in this layer:

- [monitor](interfaces/node/governance/monitor.md#governance_monitor)
- [safety](interfaces/node/governance/safety.md#governance_safety)

<a id="cognition_isolated_library"></a>
<a id="cognition_isolated_library--isolated-ei-cognition-library"></a>
## Isolated EI cognition library

[中文](../../docs_cn/ei/interfaces_cn.md#cognition_isolated_library_cn) · [Cognition layer](interfaces.md#readme--cognition) · [Node contract](../isos/interfaces/server/ei.md#node)

EI registers six additional `@v0.2` cognition type IDs for direct one-node invocation. They have no `port_routes`, bridge adapter, cross-core delivery, or authored graph execution route. `ei-config catalog` describes them; graph validation rejects an attempted instance with `isolated_node_only`. This keeps the 17-node starter unchanged. The owner or test harness validates settings and exact typed input before invoking a handler.

| Type ID | Input port and data type | Output data type | Availability |
| --- | --- | --- | --- |
| `ei.cognition.policy.system1_python@v0.2` | `input`: `grounded_goal@v0.2` | `motion_proposal@v0.2` | Requires a verified model provider. |
| `ei.cognition.policy.system1_native@v0.2` | `input`: `grounded_goal@v0.2` | `motion_proposal@v0.2` | Requires a verified model provider. A compiled `isos_node_v1` fixture exercises the EI native host; installed package qualification is still required. |
| `ei.cognition.policy.vla@v0.2` | `input`: `vla_input@v0.2` | `grounded_goal@v0.2` | Requires a verified model provider. |
| `ei.cognition.world.predictor@v0.2` | `candidate`: `motion_proposal@v0.2` | `prediction@v0.2` | Deterministic shadow baseline. |
| `ei.cognition.memory.remember@v0.2` | `record`: `memory_reference@v0.2` | `memory_reference@v0.2` | Bounded injected cognitive store. |
| `ei.cognition.memory.recall@v0.2` | `query`: `memory_reference@v0.2` | `memory_reference@v0.2` | Bounded injected cognitive store. |

The model types require `model_resource_id`, a nonempty resource selector of at most 128 characters. A provider can be injected only with a matching resource ID, exact resource file, and SHA-256 digest; bytes are checked at construction and each input. Without a provider, `health` returns `unavailable` with reason `missing_model` and `input` rejects. Python and native System 1 types share the same input, output, and rejection fixtures. The native fixture exercises `isos_node_v1` input, output, and Stop, but does not qualify an installed package or establish a latency bound. No type here invokes an actuator.

Optional `max_input_age_ms` defaults to 5000 and permits 1–60000 ms. `frame_id` constrains the input coordinate frame. `deadline_ms` defaults to 1000 and permits 1–30000 ms; a provider reply after the deadline rejects. `max_output_bytes` defaults to 65,536 and permits 1–65,536 bytes. `max_rate_hz` defaults to 20 and permits positive finite rates up to 100 Hz. `shadow_mode` defaults to true and must remain true for the predictor. Memory nodes accept `max_records` from 1 to 256, default 32. Unknown settings, wrong types, nonfinite values, stale or future source times, expired goals or proposals, and payloads over 65,536 bytes reject. All input ports are optional catalog declarations; direct callers choose one port and value. `read` returns the last typed output without invoking work. `health` includes a generation counter and source time. `stop` closes input and confirms local quiescence. A stopped node rejects further input.

<a id="cognition_isolated_library--data-types"></a>
### Data types

All six direct EI cognition types use `json_v1`, `application/json`, version `0.2`, and a 65,536-byte maximum. Every object requires `source_id` (1–128 characters), offset-aware `source_time`, nonnegative `sequence`, and `frame_id` (1–128 characters). Unknown fields reject. A source time more than the configured age old or more than one second in the future rejects. There is no implicit frame or unit conversion.

| Data type | Additional required fields and bounds |
| --- | --- |
| `language_instruction@v0.2` | `text` (1–4096 characters), `language` (2–16 characters). |
| `vla_input@v0.2` | Requires `image`, `language`, and `robot_state` objects. Image carries a resource ID, SHA-256 digest, source time, frame, 1–8192 pixel width/height, `jpeg` or `rgb8` encoding, and calibration ID. Language carries text (1–4096 characters) and language (2–16 characters). Robot state carries source time, frame, 1–64 unique joint names, the same count of finite `position_rad` values within ±12.57, and model version. Image and robot times must each be fresh and their frames must match the outer frame. The image is a bounded reference; bytes are not embedded. This direct validator checks reference metadata and digest syntax, but does not fetch image bytes or verify their digest or calibration. |
| `grounded_goal@v0.2` | `goal_id`, `target_id`, `model_version` (each 1–128 characters), `confidence` (0–1), future `expires_at`. |
| `motion_proposal@v0.2` | `proposal_id`, `target_id`, `resource_id`, `model_version` (each 1–128 characters), `horizon_ms` (1–1000), `delta_rad` (1–32 finite radian values, each within ±0.25), future `expires_at`. This is a proposal, never a command. |
| `prediction@v0.2` | `target_id`, finite `predicted_value`, `unit:"rad"`, nonnegative `uncertainty_rad`, `shadow_mode:true`. It never replaces measured EOS state. |
| `memory_reference@v0.2` | `record_id`, `policy_id` (1–128 characters), lowercase SHA-256 `digest`. A query must match the stored policy ID. The store remains EI cognition state, separate from EOS physical datasets. |

The six earlier pipeline data IDs (`resource_bundle`, `observation_set`, `context_snapshot`, `selected_mission`, `primitive_proposal`, and `health_update`, each `@v0.2`) still have broad object schemas. Their current stage outputs use different legacy payload shapes, so strict descriptors cannot be registered until those handlers and direct fixtures emit the same attributed, bounded wire values. They are not validated by the direct cognition validator on this page.

For example, the direct predictor accepts this candidate and emits a `prediction@v0.2` with the same source identity, `unit:"rad"`, `uncertainty_rad:0.25`, and `shadow_mode:true`:

```json
{
  "source_id": "fixture-camera",
  "source_time": "2026-01-01T00:00:00+00:00",
  "sequence": 4,
  "frame_id": "base_link",
  "proposal_id": "proposal-1",
  "target_id": "arm-1",
  "resource_id": "joint-1",
  "horizon_ms": 100,
  "delta_rad": [0.1],
  "expires_at": "2026-01-01T00:00:02+00:00",
  "model_version": "fixture-v1"
}
```

The emitted value is a shadow prediction, not an actuator command:

```json
{
  "source_id": "fixture-camera",
  "source_time": "2026-01-01T00:00:00+00:00",
  "sequence": 4,
  "frame_id": "base_link",
  "target_id": "arm-1",
  "predicted_value": 0.1,
  "unit": "rad",
  "uncertainty_rad": 0.25,
  "shadow_mode": true
}
```

`ei/tests/ei/test_isolated.py` is a direct fixture unit test for schema admission, model availability, equivalent System 1 behavior, VLA output rejection, shadow prediction, memory policy, health, and Stop. `ei/tests/ei/test_isolated_native.py` compiles a fixture `isos_node_v1` library and checks native input/output and Stop through the EI host. Run `PYTHONPATH=ei ei/.venv/bin/python -m pytest ei/tests/ei/test_isolated.py ei/tests/ei/test_isolated_native.py -q` from the repository root. These tests use a fixed clock and no physical device or model weight.

For the VLA input change, `PYTHONPATH=ei ei/.venv/bin/python -m pytest ei/tests/ei/test_isolated.py ei/tests/ei/test_isolated_native.py ei/tests/ei/test_stages.py -q` passed 55 tests. `ei/.venv/bin/ruff check ei/core/system/isolated.py ei/tests/ei/test_isolated.py ei/tests/ei/test_stages.py`, `ei/.venv/bin/pyright --project ei/pyproject.toml ei/core/system/isolated.py ei/tests/ei/test_isolated.py`, and the import rules passed. Full historical logs are under ignored `data/test/ei-vla-multimodal-20261115/`; use the current import-rule command in the [EI quick start](../isos/start/quick-start.md) for new checks.
