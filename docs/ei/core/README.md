# EI core

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EI architecture](architecture.md)
  - [EI roadmap](roadmap.md)
- Subfolders:
  - [cognition/](cognition/)
  - [evidence/](evidence/)
  - [governance/](governance/)
  - [io/](io/)
  - [optional/](optional/)
  - [runtime/](runtime/)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/ei/core/README.md) · [EI documentation](../README.md)

Start with the current [architecture](architecture.md); future cognition work is in the [roadmap](roadmap.md). EI owns cognition and the safety gate for graph-selected behavior. Its node layers describe the declared stages visible in the IsOS library; the core runtime pages describe supervision, exchanges, and optional facilities that support more than one stage. A node page explains its own behavior while the public input and output rules stay in [EI interfaces](../interfaces.md#readme).

Registered node layers and groups:

- [io](README.md#io)
- [evidence](README.md#evidence)
- [cognition](README.md#cognition)
- [governance](README.md#governance)

Core runtime design:

- [Runtime modules](README.md#core)


## Documentation layout verification

The owner reading map was checked with `python3 scripts/check_doc_links.py --all` (639 Markdown files; zero broken relative links) and `git diff --check` (passed). Complete command output is saved under ignored `data/test/2026-11-14-doc-layout/checks.log`.

## EI node catalog verification (2026-11-15)

From the repository root, `python3 scripts/check_doc_links.py --all` checked 545 Markdown files with zero broken relative links, and `git diff --check` passed. Complete command output is saved under ignored `data/test/2026-11-15-ei-doc-layout/checks.log`.

Runtime page consolidation check (2026-11-15): `python3 scripts/check_doc_links.py --all` checked 539 Markdown files with zero broken relative links; `git diff --check` passed. Output is in ignored `data/test/2026-11-15-ei-doc-layout/runtime-merge.log`.

## io

IO stages receive owner-bound inputs and connect EI to its declared peer boundaries. Input nodes admit trigger and arm-status events; the link group describes the gateway used for owner exchanges. Hardware control still passes through the declared safety and EOS dispatcher path.

Node groups in this layer:

- [input](io/input.md)
- [link](io/link.md)

## evidence

Evidence admits perception data before cognition uses it. The perception group describes the admission stage and its boundaries; source payloads and external contracts remain defined by the EI interface pages.

Node groups in this layer:

- [perception](evidence/perception.md)

## cognition

Cognition turns admitted observations and mission state into bounded context, decisions, and intent. The stage nodes in these groups pass events through the selected EI projection; they do not own actuator admission. Context assembly, planning, mission queueing, and arm-state projection are described by their group pages.

Node groups in this layer:

- [context](cognition/context.md)
- [decision](cognition/decision.md)
- [intent](cognition/intent.md)
- [missions](cognition/missions.md)
- [state](cognition/state.md)

## governance

Governance applies EI safety and observation policy after the preceding stages. The safety gate is the final EI check before an activation reaches EOS; monitor stages handle read-only health and arm-state observations.

Node groups in this layer:

- [monitor](governance/monitor.md)
- [safety](governance/safety.md)

## core

These pages describe EI runtime mechanisms that support multiple registered node groups. They are organized by runtime execution, cognition support, and optional facilities so a node page can link to shared behavior without redefining it. Node placement and individual stage behavior remain under the layer and group pages.

- [cognition](README.md#core-cognition)
- [optional](README.md#core-optional)
- [runtime](runtime/runtime.md)

## core cognition

Coordination covers how profile execution shares state and orders work across cognition stages. Memory covers retained context and recall used by those stages. These mechanisms support cognition nodes but are not themselves registered node groups.

- [coordination](cognition/coordination.md)
- [memory](cognition/memory.md)

## core optional

The optional sqlite-vec index augments the core memory layer when available. Sandbox is a core layer with optional activation; see its runtime design for setup and failure handling.

- [memory](optional/memory.md)
- [sandbox](runtime/sandbox.md)
