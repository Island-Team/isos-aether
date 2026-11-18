# EOS hardware runtime design

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/base group](base.md)
  - [hardware/camera group](camera.md)
  - [hardware/gripper group](gripper.md)
  - [Develop EOS hardware adapters](hardware-guide.md)
  - [hardware/manipulator group](manipulator.md)
  - [hardware/mock group](mock.md)
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/hardware/hardware_cn.md) · [Hardware contract](../../interfaces/node/hardware/mock.md#hardware_mock--arm) · [Dispatcher](../dispatcher/dispatcher.md)

## Ownership and data flow

IsOS owns saved graph revisions, verified package resources, and Apply. The current `0.2.1` graph has separate `isos`, `ei`, and `eos` cores. EOS reads its projection and constructs the declared `eos.hardware.mock.arm@v0.2` adapter once per worker run. The historical `isos.eos_hardware.*@v0.2` path is retained as a compatibility boundary; new package camera observation uses the current registered `eos.hardware.camera@v0.2` path. The registered type sets placement, settings, ports, capabilities, and lifecycle; the instance provides identity and values. EOS owns live status and health, and Dispatcher owns command admission and safety.

The Rust hardware layer carries typed camera, base, and manipulator samples and commands. Built-in and package adapters enter through their declared implementation and the same admission boundary. IsOS verifies selected package bytes and resources; EOS validates the adapter, owns its live health, and closes Dispatcher admission before confirming Stop. A package service endpoint is private to the selected run and cannot become a second command route.


## Admission and failure

EOS validates node placement, adapter family, settings, graph references, typed edges, health timing, frame IDs, movement modes, and limits before readiness. A malformed or stale sample degrades only its node. A backend cannot create a second actuator route. Every movement passes through Dispatcher with the active revision, binding, typed payload, deadline, and durable command ID.

Stop closes movement admission, requests stop for admitted actuators, records confirmed or unknown outcomes, then terminates and reaps any selected service child. A lost adapter or child degrades dependent nodes and withholds movement. Physical adapters must enforce their own write-boundary stop and watchdog behavior; mock tests do not replace hardware qualification.

## Backend selection

The current Rust runtime admits the CXX mock hardware adapter, a verified v1 package camera observer, and selected v2 package service devices with Dispatcher mediated commands and run-bound Stop proof. IsOS installs or links an exact `v0.2.x` package, selects its process and camera nodes in graph format `0.2.1`, and verifies the executable, resources, and shared library. EOS reads the applied graph, supervises the package process as its direct child, and runs the declared camera adapter through its private observation route. Package base, manipulator, gripper, and torso commands require a verified v2 service handoff, Dispatcher admission, and physical Stop proof before readiness.

The v2 native worker handles Dispatcher command acknowledgement outside the graph route plan. Once the owner admits a command, the worker invokes only the targeted actuator callback and returns exactly one validated `command_result` to the owner. Graph outputs and observation callbacks cannot use this reserved port; this keeps acknowledgement from becoming a second actuator or graph route.

The registered node type controls placement, capabilities, settings, ports, and lifecycle. The instance supplies identity and selected values. IsOS checks exact package pins and typed edges before Apply; EOS validates adapter settings and health before readiness. Dispatcher admits every actuator command and owns safety and Stop. See the [package observer contract](../../../isos/interfaces/server/eos.md#package_observer) for Run, observation, and Stop. Package-specific setup belongs in that package repository.

The current demo graph uses the registered EOS mock arm adapter and [activation contract](../../interfaces.md#arm_control). Its six-joint state is per worker run; Dispatcher admits the skill-generated delta before any update.

The owner binds a read-only `ArmObserver` instance to the graph's zero-input hardware-layer source path and passes that path to the worker. Monitor requests dispatch through this instance, which reads the sole mock-arm adapter's sampler and publishes a [typed arm observation](../../interfaces.md#arm_monitor) to the monitor dispatcher, status skill, and interface egress nodes. The worker samples at 100 Hz by default and immediately after each admitted movement. The graph may override the 10 ms sample interval and 50 ms freshness timeout through the mock arm’s Advanced settings. Each observation carries the observer path, run fence, monotonic sample time, sequence, graph-ordered joint names, and state. The owner exposes the latest sample through a read-only private route; a stale or invalid sample blocks new movement until a valid sample restores readiness.

## Asynchronous demo execution and monitoring

The EOS worker synchronously records each command reservation in a run-named journal before returning an acceptance receipt. One executor processes accepted commands in order; result lookup uses the event ID to retrieve progress or a terminal result. Repeated event IDs never cause another movement. Stop closes admission and joins the executor; unconfirmed results after worker loss remain uncertain and are not replayed automatically. The arm sampler independently maintains the latest observation and sequence, so multiple readers consume that snapshot without driving sampling. See the [arm control contract](../../interfaces.md#arm_control) for fields.

<a id="reader-preflight-design"></a>

## Reader preflight design

EOS advertises the graph version, registered node types, and features accepted by each selected binary. IsOS uses those exact observations to decide whether a saved deployment can run. The profile tool, runtime, and config server derive built-in type IDs from the same registry. Package-contributed types enter only through verified sidecars passed to setup; a binary probe does not claim that a package is installed.

For Validate and Apply, IsOS fixes the saved graph revision, verifies package pins and selected binaries, derives required features from graph nodes and edges, and invokes each binary directly. It checks exact identity, type IDs, and required features before owner rendering or process changes. Run and Restart check the immutable generation and selected binary hashes again. Apply selects a new generation.

Build the EOS binaries with `cargo build --workspace`, then run `python3 tests/reader_features_cli.py`. The replay checks current graph `0.2.1` probe identity, setup identity agreement, and validation of the checked-in mock profile. Save complete output in ignored `data/test/<run-id>/`.

<a id="graph-vision-first-use"></a>

## Graph Vision first use

Connect a declared camera `frame` output to `eos.interface.egress.vision_flow@v0.2`, then connect the transfer to `isos.ui.vision.monitor@v0.2`. Apply the graph, Run, wait for readiness, and open the Vision page. Check that the displayed source and dimensions match the accepted frame. Stop and confirm the image clears. The private EOS `vision_frame` exchange is run and revision fenced; malformed frames and worker failure return no image.

### Camera Vision class verification (2026-11-15)

The graph Vision input uses data class `image`, matching package camera `image_frame@v0.2` outputs. From the repository root, run `cargo fmt --all -- --check`, `cargo clippy -p aether-node-profile --all-targets -- -D warnings`, `cargo test -p aether-node-profile`, and `python3 scripts/check_doc_links.py --all`. These checks passed; complete output is under ignored `data/test/20261115-vision-camera-image-class/`.

### Removal verification (2026-11-14)

The retired camera-gate/JPEG bridge route was removed while preserving the graph `vision_frame` exchange. Reproduce from the repository root with `cargo test -p aether-http-bridge --test bridge -- --test-threads=1`, `cargo check -p isos`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all`. All passed; complete output is under ignored `data/test/20261114-legacy-vision-removal/`. The EI owner check (`uv run pytest -q tests/ei tests/system/test_reader_features.py tests/benchmarks/test_ei_q03.py`) passed 676 tests with 6 skipped, and the UI typecheck and test suite passed 224 tests.
