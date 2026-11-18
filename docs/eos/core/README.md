# EOS core

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EOS architecture](architecture.md)
  - [EOS roadmap](roadmap.md)
- Subfolders:
  - [dispatcher/](dispatcher/)
  - [gateway/](gateway/)
  - [hardware/](hardware/)
  - [interface/](interface/)
  - [process/](process/)
  - [skill/](skill/)
  - [skills/](skills/)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/eos/core/README_cn.md) · [EOS](../README.md)

Start with the current [architecture](architecture.md) for execution ownership and lifecycle; future work is in the [roadmap](roadmap.md). Shared runtime mechanisms cover [process ownership and shutdown](process/process.md), [Dispatcher admission and scheduling](dispatcher/dispatcher.md), [hardware adapters and frame provenance](hardware/hardware.md), [browser and gRPC gateways](gateway/grpc-gateway.md), and the [Rust skill host](skills/host-rust.md). Current node-library groups are indexed below. Exact externally consumed fields remain in [interfaces](../interfaces.md#readme). Operator tasks for [ARM64 build](../../isos/start/quick-start.md#arm64-build), [configuration](../../isos/start/quick-start.md#configure-eos), and [debugging](../../isos/start/quick-start.md#debug-eos) are in the quick start.

The current library has [interface](README.md#eos-interface-layer), [skill](README.md#eos-skill-layer), [dispatcher](README.md#eos-dispatcher-layer), and [hardware](README.md#eos-hardware-layer) layer sections. Historical `isos.eos_*` types are identified in the owning group pages and do not define current `eos.*` nodes.

## Documentation layout verification

The owner reading map was checked with `python3 scripts/check_doc_links.py --all` (639 Markdown files; zero broken relative links) and `git diff --check` (passed). Complete command output is saved under ignored `data/test/2026-11-14-doc-layout/checks.log`.

<a id="eos-hardware-layer"></a>

## EOS hardware layer

The hardware layer adapts graph nodes to selected devices. The built-in mock group serves the demo arm path. Other groups are package-contributed candidates and appear in the node library only after their exact package is installed and verified. Each adapter owns its device-specific behavior; the [core hardware design](hardware/hardware.md) describes the shared runtime boundary.

Group introductions summarize the available [mock](hardware/mock.md), [camera](hardware/camera.md#camera-adapter-node), [base](hardware/base.md#mobile-base-adapter-node), [manipulator](hardware/manipulator.md#manipulator-adapter-node), [gripper](hardware/gripper.md), and [torso](hardware/torso.md) placements. Node pages describe behavior; [hardware interfaces](../interfaces.md#readme--eos-hardware-contracts) own exact public contracts.

<a id="eos-dispatcher-layer"></a>

## EOS dispatcher layer

The dispatcher layer is the EOS control boundary. Its arm group admits requested joint deltas against the active run and movement limits, then passes accepted commands to the selected mock actuator. The monitor node carries the return observation path; it does not open another actuator route. Shared admission, scheduling, and route mechanics are described in the [core dispatcher design](dispatcher/dispatcher.md).

[Arm group](dispatcher/arm.md) contains the two current node designs. Their exact ports and payload rules are in the [dispatcher interfaces](../interfaces.md#readme--eos-dispatcher-contracts).

<a id="eos-interface-layer"></a>

## EOS interface layer

The interface layer connects EOS graph routes to their external ingress and egress boundaries. Ingress receives the declared trigger; egress exposes arm status and validated vision frames. The graph edges and run identity determine the source and destination of each transfer. These nodes do not select hardware or bypass dispatcher admission.

Read the [ingress group](interface/ingress.md) and [egress group](interface/egress.md) for node behavior. The [interface contracts](../interfaces.md#readme--eos-interface-contracts) define exact boundary values; [gateway design](gateway/grpc-gateway.md) covers shared transport mechanics.

<a id="eos-skill-layer"></a>

## EOS skill layer

The skill layer contains software transformations in the EOS graph. The current demo group produces a joint delta and handles arm status in the button-to-mock-arm control and return chains. Skills connect through typed graph ports; actuator admission remains with the dispatcher.

Read the [demo group](skill/demo.md) for its node designs. The [skill interfaces](../interfaces.md#readme--eos-skill-contracts) define the external node contract, while the [Rust skill host design](skills/host-rust.md) describes shared runtime hosting.

<a id="eos-legacy-layer"></a>

## Legacy EOS node designs

The group sections describe saved-graph compatibility types with `isos.eos_*` identities. They do not define nodes in the current `eos.<layer>.<group>` library. See [dispatcher bindings](dispatcher/base.md#base-primitive-binding-node), [control](dispatcher/control.md#shared-control-dispatcher-node), [manipulator binding](dispatcher/manipulator.md#manipulator-primitive-binding-node), [hardware base](hardware/base.md), [camera](hardware/camera.md), [manipulator](hardware/manipulator.md).
