<a id="cognition_decision"></a>
<a id="cognition_decision--cognition--decision"></a>
# Cognition / decision

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/interfaces/node/cognition</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Cognition / context](context.md)
  - [Cognition / intent](intent.md)
  - [Cognition / missions](missions.md)
  - [Cognition / state](state.md)

</details>
<!-- doc-navigation:end -->

[Node groups](../README.md) · [中文](../../../../../docs_cn/ei/interfaces/node/cognition/decision_cn.md#cognition_decision_cn) · [Layer](../../../interfaces.md#readme--cognition) · [EI node boundary](../../../../isos/interfaces/server/ei.md#node)

The [Shared node contracts](../../../../node/interfaces/README.md) define type identity, settings, ports, routes, and registry rules; EI owns this group’s node behavior. This catalog describes the EI behavior for the nodes in this group; the [group design](../../../core/cognition/decision.md) explains their runtime roles.

<a id="cognition_decision--planner"></a>
<a id="planner"></a>

<a id="cognition_decision--eicognitiondecisionplannerv02"></a>
## ei.cognition.decision.planner@v0.2

The direct `PlannerStage` handler accepts a selected mission with nonempty `mission_id` and a context snapshot with bounded `text` (at most 65,536 bytes). Without a verified model provider, it emits `plan_state` with `state:"abstained"` and `reason:"missing_model"`; it never fabricates movement. An injected provider requires an exact model file and SHA-256 digest, checked at construction and before every call. It must return a nonempty `plan_id`, 1–`max_steps` short step strings, and a proposal with nonempty `backend`, `device`, `primitive`, `version`, `target_id`, object `arguments`, and future offset-aware `expires_at`. `max_steps` defaults to 8 (1–64), and `deadline_ms` defaults to 1000 (1–30000). Invalid or late output rejects before emission. `ei-config` marks this catalog type as requiring a verified model and rejects graph selection with `missing_model_package`; no package-to-graph planner wiring is claimed in this isolated phase. Stop closes input.
