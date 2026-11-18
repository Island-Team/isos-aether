# EI architecture

<!-- doc-navigation:start -->
<details>
<summary>Folder: ei/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EI core](README.md)
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

[中文](../../../docs_cn/ei/core/architecture.md) · [EI](../README.md) · [Interfaces](../interfaces.md#readme)

EI owns cognition, mission planning, its safety gate, and cognitive memory. It consumes admitted observations and proposes actions; EOS alone admits actuator commands and owns hardware safety. IsOS owns the saved graph, package verification, and deployment run identity. The [EI setup](../../isos/interfaces/server/ei.md#setup), [node](../../isos/interfaces/server/ei.md#node), and [EOS exchange](runtime/io.md#southbound-operation-handling) pages define the current boundaries.

## Current runtime boundary

EI's persistent owner validates the isolated `cores.ei` projection, selects one run, and supervises one managed worker at a time. IsOS calls Run, Status, Stop, and activation; manager exit does not terminate EI. In graph `0.2.1`, three activation nodes pass the same `button_activation@v0.2` from input through intent to the safety gate. EI does not choose arm joint values or create an actuator route. At the gate it checks run, revision, declared path, worker liveness, and EOS endpoint; an uncertain downstream result is surfaced without retry. Stop serializes with activation and clears admission.

The independent read-only monitor chain passes the latest checked EOS arm sample through EI input, state, and monitor to IsOS. EI preserves source sequence, sample time, state, reason, joint names, and valid positions; stale or faulted samples remain observations. The current managed health worker is separate from the owner-side data pass. The current path does not implement a unified tensor space, learned VLA skill routing, or a world-model control loop.

The owner verifies worker digest and run identity before launch. The health-only worker has no safety state or actuator authority and does not take the full EI runtime's global safety-state lock, allowing separate deployments to keep health workers alive under one state root. The 2026-11-09 concurrent-owner check ran two such children, then reaped both on Stop; the full EI test suite, Ruff, Pyright, import rules, and bilingual link audit passed. Complete output is in ignored `data/test/ei-arm-monitor-20261109/`.

Future direction: [roadmap](roadmap.md).
