# Develop EOS hardware adapters

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/hardware</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [hardware/base group](base.md)
  - [hardware/camera group](camera.md)
  - [hardware/gripper group](gripper.md)
  - [EOS hardware runtime design](hardware.md)
  - [hardware/manipulator group](manipulator.md)
  - [hardware/mock group](mock.md)
  - [hardware/torso group](torso.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/hardware/hardware-guide_cn.md) · [Hardware interface](../../interfaces/node/hardware/mock.md#hardware_mock--arm) · [Runtime design](hardware.md) · [Dispatcher design](../dispatcher/dispatcher.md)

Start with the registered node type and its owning interface contract. The current demo EOS hardware node lives under `eos/hardware/mock/arm` in graph format `0.2.1`. Declare its capabilities, typed ports, settings, limits, health timing, and adapter family in the registry; the graph instance supplies identity and selected values. Do not create a second manifest or actuator route.

## Add or change a node

1. Update the [hardware contract](../../interfaces/node/hardware/mock.md#hardware_mock--arm) and its Chinese mirror with each public field's type, unit, purpose, default, and validation.
2. Implement one adapter per node. Keep device communication behind the Rust HAL or its narrow CXX/C ABI boundary; transfer bounded typed samples and commands.
3. Validate settings and references before device construction. Return explicit health and status, reject stale or malformed observations, and correlate every command with its durable ID.
4. Route all movement through Dispatcher. Implement independent physical Stop at the adapter write boundary and verify the watchdog for leased movement.
5. Add mock unit tests and an integration boundary covering success, rejection, failure, restart, and Stop.

## Checks and logs

Run `cargo fmt --all`, `cargo fmt --all -- --check`, workspace Clippy, focused HAL/Dispatcher/runtime tests, applicable mock or hardware integration, and `python3 scripts/check_doc_links.py --all`. Save complete outputs under ignored `data/test/<run-id>/`; document reproducible commands and results in the owning guide. A benchmark uses a fixed workload and reports throughput, median, p99, and environment before setting a performance target.

On 2026-11-12, `cargo fmt --all -- --check`, focused `aether-profile` setup and catalog tests, `cargo clippy -p aether-runtime --bin aether-profile --offline -- -D warnings`, `cargo build -p aether-runtime --bin aether-profile --offline`, UI typecheck, all 218 UI tests, Web and TUI builds, and `python3 scripts/check_doc_links.py --all` passed. The live IsOS options request for the installed MuJoCo `0.2.2` Native graph returned four available `mujoco-v2` adapter groups; the unmodified mock graph returned its mock-arm choice. Outputs are under ignored `data/test/20261112-eos-package-adapter-options/`. The broader EOS binary test run encountered an unrelated in-progress edit to `config/graph-pool/demo.nodes.json`; four fixture-dependent tests failed because the edit added an incomplete package camera. The edit was preserved.

The built-in EOS catalog is constructed directly from the current `eos.*@v0.2` node definitions in `crates/aether-node-profile/src/registry.rs`. Verify registry and edge admission with `cargo test -p aether-node-profile`, command admission and runtime integration with `cargo test -p aether-runtime --lib`.

For the owner-first file layout documentation change on 2026-11-07, `python3 scripts/check_doc_links.py --all` passed (464 Markdown files, no broken relative links). Complete output is in ignored `data/test/20261107-eos-paths/doc-links.log`. EOS runtime path inputs are required or explicitly injected; IsOS owns the source, portable, and system defaults.

For the 2026-11-08 node mismatch event and sidecar placement alignment, `cargo fmt --all -- --check`, `cargo test -p aether-common -p aether-runtime --bin aether-runtime --bin aether-profile`, `cargo clippy -p aether-common -p aether-runtime --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all` passed. Complete output is in ignored `data/test/20261108-eos-node-alignment/`.

Contributed node types use the same `isos.<layer>.<name>@v0.2` identity and a nonblank description with fully described typed ports and settings. The registry rejects other graph type identities before they can enter the catalog.

## Button-to-mock-arm demonstration, 2026-11-09

The EOS projection has one live ingress, skill, dispatcher, and mock arm node. The mock arm is registered as a manipulator adapter with a `mock_arm` safety adapter. The worker stores six joint positions for one run. The built-in skill transforms an admitted activation to a fixed relative delta, and Dispatcher checks per-command and resulting position bounds before the mock adapter updates state. See the [arm control contract](../../interfaces.md#arm_control) for the wire fields and units.

Verify with `cargo test -p aether-node-profile`, `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`, `cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all`. The socket integration test covers Run, two activations, duplicate replay, stale revision rejection, and Stop. Complete command output is saved under ignored `data/test/20261109-eos-arm-demo/`.

## Mock arm observation return, 2026-11-09

The [arm monitor contract](../../interfaces.md#arm_monitor) defines four return-path nodes sourced from the mock arm sampler, one typed observation payload, and `POST /v1/monitor`. The worker samples at 100 Hz by default and immediately after movement. Advanced mock arm settings expose a 10 ms sampling period and 50 ms freshness timeout; bounded graph overrides reach the worker. Stale and invalid samples block movement until a fresh valid sample restores readiness.

## Independent mock observation source, 2026-11-10

The new demo uses `eos.hardware.mock.arm_observer@v0.2` as the zero-input first node of the return chain. It reads the existing adapter's periodic sampler; the command chain ends at the sole mock-arm actuator. The old input-bound `arm_output@v0.2` route remains valid for saved graphs. From the repository root, run `cargo fmt --all -- --check`, `cargo test -p aether-node-profile --tests`, `cargo test -p aether-runtime --test arm_activation`, `cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all`. Profile tests cover both graph routes and reject a command-chain input on the observer. The owner/worker socket test proves the observation sequence advances before activation, then continues after movement and ends on Stop. Full output is saved under ignored `data/test/eos-observer-20261110/`.

From the repository root, run `cargo fmt --all`, `cargo fmt --all -- --check`, `cargo test -p aether-node-profile --tests`, `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`, `cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all`. The unit tests cover initial zero, stale and invalid samples, recovery, immediate post-move observation, and Stop. The real owner/worker socket test covers Run, sampling progress, monitor reads, movement, stale revision rejection, and Stop. Complete output is saved under ignored `data/test/20261109-eos-arm-monitor/`.

## Monitor health and timing controls, 2026-11-09

The mock arm catalog exposes optional Advanced `sample_interval_ms` (default 10) and `freshness_timeout_ms` (default 50). The profile bounds each and requires freshness to be at least twice the sample interval. EOS passes effective values to its worker. Owner health and Status preserve the worker's `degraded` or `stale` state, while Monitor remains readable and movement stays blocked. Verify with `cargo test -p aether-node-profile --tests`, `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`, `cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all`. Complete output is in ignored `data/test/20261109-eos-monitor-health/`.

## Asynchronous demo verification (2026-11-10)

`cargo fmt --all -- --check`, `cargo clippy -p aether-runtime --all-targets -- -D warnings`, `cargo test -p aether-runtime`, and `python3 scripts/check_doc_links.py --all` passed. The real owner/worker socket test covers the acceptance receipt, result lookup, duplicate ID, monitoring, and Stop; a reservation unit test checks journal writes and single movement. Complete output is in ignored `data/test/async-eos-20261110/`.

## File validator CLI (2026-11-11)

`aether-profile validate --file <graph>` now checks the EOS-only projection against the merged registry and executable-mode admission used by setup and Run. From the repository root, `cargo fmt --all -- --check`, `cargo test -p aether-runtime --bin aether-profile --test profile_validate`, `cargo clippy -p aether-runtime --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all` passed. The CLI tests cover an executable demo, an incomplete chain, a non-executable EOS projection, a multi-core graph, and an unavailable sidecar. Complete output is in ignored `data/test/20261111-eos-profile-validate/checks.log`.


## General hardware contract slice (2026-11-11)

At this earlier general-contract checkpoint, the registry declared bounded image, base, and joint observations, base velocity and joint position commands, and generic camera, base, and manipulator nodes. `aether-profile validate --file <eos-projection> --node-types <verified-sidecar>` validated the EOS-only graph without starting a worker. Setup then rejected execution of all three generic device kinds. The later observer slice below admits verified camera observation only; base and manipulator execution still requires Dispatcher admission and physical Stop proof.

From the repository root, `cargo fmt --all`, `cargo fmt --all -- --check`, `cargo test -p aether-node-profile`, `cargo test -p aether-runtime`, `cargo clippy -p aether-node-profile -p aether-runtime -p isos --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all` passed. The profile tests cover missing backend binding, package/socket/device coherence, schema bounds, and Dispatcher-only command sources; EOS tests cover executable demo admission, unavailable generic execution, owner/worker control, and Stop. Complete output and the replay script are under ignored `data/test/20261111-general-hal/`.

## External package handoff admission (2026-11-11)

The EOS owner accepts `--adapter-library family=<absolute .so>@<sha256>` and one `--package-process <absolute file>` only with `owner --socket`. At owner startup it hashes each adapter again, compares it with the selected sidecar, and validates the IsOS process handoff against its exact package receipt, release digest, contained executable, and selected resource digests. Duplicate families and mismatches fail before the owner listens. A direct-child supervisor helper creates a separate process group and reaps it even when the owner-specific Stop proof fails; it never reports confirmed Stop from process exit alone. At this handoff checkpoint, general hardware execution remained rejected. The later observer slice adds camera sampling and bounded Stop; actuator execution remains gated. Verify with `cargo test -p aether-runtime --bin aether-profile`, `cargo clippy -p aether-runtime --bin aether-profile -- -D warnings`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all`.

## Package observer execution (2026-11-11)

The owner now starts one verified observation-only package process before its native camera worker, binds `${socket}` and selected resource paths from bounded process arguments, and exposes `/v1/observe`. The worker invokes the adapter's `isos_node_v1` callbacks and EOS validates declared output ports and payload schemas. Stop requires adapter quiescence and process group exit. Actuator execution now passes through Dispatcher admission and requires a run-bound physical Stop proof for the selected v2 service. The isolated ABI probe is `aether-runtime --probe-adapter <absolute .so>`. See the [package observer contract](../../../isos/interfaces/server/eos.md#package_observer).

Reproduce this boundary from the repository root with `cargo fmt --all -- --check`, `cargo test -p aether-node-profile`, `cargo test -p aether-runtime --bin aether-profile`, `cargo test -p isos --bin isos pkg_`, and `python3 scripts/check_doc_links.py --all`. Build a candidate with its own `v0.2.x` script, then run `isos pkg check --source <absolute-checkout> --version <exact-version> --output <empty-absolute-directory> --graph <candidate-graph>`. The check requires isolated ABI probing and managed Run, observation, logs, and confirmed Stop replay. Save complete output under ignored `data/test/<run-id>/`.

## General package v2 declaration and ABI checkpoint (2026-11-12)

The EOS catalog now registers normalized base and Cartesian arm interfaces, a binary gripper, and a normalized torso alongside the existing SI base and joint-position devices. The node-profile reader accepts a valid `adapter.settings.service_instance` with an exact package pin and rejects an authored `socket_path`; IsOS must inject the private endpoint at Run. The Linux `isos_node_v2` host probes the ABI, invokes observation separately from commands, and checks a run-bound stop proof including the selected actuator outcome. At this earlier ABI checkpoint, the host rejected v2 commands until the owner bound Dispatcher reservation and the durable journal. The current owner now supplies that binding. The existing v1 package path remains available.

For camera source selection, EOS `options` now reads incomplete hardware instances before executable-profile validation and returns exact-pin adapter choices. IsOS reads camera source profiles from the verified package node sets, exposes service and device choices, and checks the selected service's package configuration and host prerequisites before marking it available. The editor has no default camera. Selecting a device copies its declared frame and adapter fields while retaining the instance identity and edges. The current MuJoCo `0.2.2` release declares native `cam-0` and three RoboCasa cameras. The exact `0.2.2` local package configuration was imported from the already configured `0.2.1` pin after confirming its host paths exist. Verification commands and full output are under ignored `data/test/20261112-mujoco-camera-options/`. The stubbed package observer passed frame delivery and Stop. An isolated candidate replay applied both native and RoboCasa graphs, observed a native JPEG and a RoboCasa wrist RGB frame, and confirmed Stop for both; the overall package-check command exited unsuccessfully after these replay records and produced no final `check.json`, so it does not qualify the candidate release.

From the repository root, `cargo fmt --all -- --check`, `cargo test -p aether-node-profile --tests`, `cargo test -p aether-runtime --bin aether-runtime native_node`, `cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, `cargo check --workspace`, and `python3 scripts/check_doc_links.py --all` passed. Full outputs are in ignored `data/test/20261112-eos-full-hal/`. The unrelated existing `reserved_event_is_durable_and_repeated_once` test fails when running the complete `aether-runtime` binary suite in this checkout; the following test then encounters its poisoned test lock. This paragraph records the earlier ABI checkpoint; the later full managed package qualification exercises the Dispatcher route and IsOS service handoff.


## Full managed package qualification (2026-11-12)

Reproduce with `isos pkg check --source <absolute-clean-checkout> --version 0.2.1 --output <empty-absolute-directory> --package-config <absolute-package.toml> --graph <absolute-native-graph.json> --graph <absolute-robocasa-graph.json>`, then inspect `check.json`, `qualification.json`, and each `graph-*/managed-replay/` directory. An independent exact-pin test installed and verified `0.2.0` and `0.2.1` together, imported configuration for only `0.2.1`, and removed either version without changing the other pin's hashes; its local record is `<qualification-checkout>/data/test/two-version-isolation/README.md`.

## Graph Vision transfer

Connect a selected package camera's `frame` output to `eos.interface.egress.vision_flow@v0.2` `frame`, then connect its output to an IsOS UI Vision monitor. EOS owns the transfer as a standard interface and keeps it outside the native plugin worker. The private `vision_frame` owner request checks the applied camera binding and run identity, observes through the verified native camera adapter, and returns one validated frame. `cargo test -p aether-runtime --test package_observer` exercises the real owner and native ABI with successful frame transfer, wrong-source refusal, and Stop.

The 2026-11-12 isolated replay used `island/mujoco-sim@0.2.2` with native `cam-0`: `isos config validate` → `isos config apply --revision <sha256>` → `isos run` → `GET /api/isos/v1/vision/bindings` → one graph-bound WebSocket frame → `isos deployment stop`. It observed a fenced 640×480 JPEG and confirmed Stop; bindings then returned 503. Reproduce the exact commands and inspect complete logs under ignored `data/test/20261112-eos-ui-vision-flow/replay.txt`. Formatting, Clippy, focused Rust tests, UI typecheck/tests/build, and the bilingual link audit passed. The workspace suite passes when three existing tests that read the user's unfinished local camera draft as the fixed demo graph are skipped; complete output is in the same evidence directory.

## Mock worker observation exchange (2026-11-12)

The mock worker exposes its declared observer sample through the read-only `observe_output` private exchange for owner-side cross-worker routing. Reproduce with `cargo fmt --all`, `cargo fmt --all -- --check`, `cargo build -p aether-runtime --lib`, `cargo test -p aether-runtime --bin aether-runtime`, `cargo clippy -p aether-runtime --bin aether-runtime -- -D warnings`, and `python3 scripts/check_doc_links.py --all`. These checks passed; complete outputs are under ignored `data/test/20261112-eos-mock-worker-exchange/`. The explicit library build is required in a fresh worktree because the existing demo skill test loads the sibling `libaether_runtime.so` and the bin-only test command does not build it. The initial failed bin-only run, before the library build, is retained as `bin-tests.log`; the passing full run is `bin-tests-after-lib.log`.

## Mixed mock-arm and package camera replay (2026-11-13)

The checkpointed 19-node graph at revision `5175494c07b46f31979582b1c39082d48d268631e4ea7d6c7dd26386a88ab386` passed isolated Setup and Apply with the verified `island/mujoco-sim@0.2.2` package. Run reached EI and EOS readiness with the mock worker, native worker, and RoboCasa service child. A graph-bound Vision WebSocket delivered a fenced `robocasa/camera/agentview-right` frame; the button activation succeeded with joint positions `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`, and the arm monitor returned the same positions. Logs were read before `isos deployment stop` returned `outcome:"confirmed"`; the final status had no active run or children, and the selected service and worker PIDs had exited. Use a short `ISOS_HOME` path: the first evidence home placed `ei-owner.sock` at 108 bytes and its owner services could not bind. The stable 16-node test fixture keeps demo-only regression coverage while the repository graph pool carries this mixed example; a fresh `isos config init` still creates its package-free starter. `ISOS_AETHER_BIN_DIR=$PWD/target/debug RUST_TEST_THREADS=1 cargo test --workspace`, `cargo clippy -p aether-runtime -p isos --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all` passed. The exact commands, HTTP replies, full frame and log output, Stop proof, and test output are under ignored `data/test/2026-11-12-eos-mixed-replay/evidence/`.

The native v2 Dispatcher acknowledgement boundary was checked with `cargo fmt --all`, `cargo fmt --all -- --check`, `cargo build -p aether-runtime --bin aether-runtime --lib`, `cargo test -p aether-runtime --bin aether-runtime`, `cargo clippy -p aether-runtime --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all`. All passed; complete outputs are under ignored `data/test/20261113-native-command-result/`. The worker socket test checks private-token admission, stale-run rejection, one valid acknowledgement, malformed and duplicate outputs, reserved graph ports, and Stop proof. In a fresh worktree, the first full bin test run preceded the sibling library build and failed the demo activation test; the passing rerun after the library build is `runtime-test-after-build.log`.

The 2026-11-13 RoboCasa 2048-pixel boundary update raises the built-in image JSON limit to 20 MiB and the native graph delivery budget to 24 MiB. `cargo fmt --all -- --check`, strict Clippy for the affected crates, full `aether-node-profile`, runtime binary, and IsOS binary tests (with `ISOS_AETHER_BIN_DIR` set), UI typecheck/tests/local and hosted Web/TUI builds, and the bilingual link audit passed. Synthetic 2048 × 2048 RoboCasa and larger general RGB frames passed the Vision validator. Complete outputs are under ignored `data/test/20261113-robocasa-2048/`. The matching package candidate must be qualified with managed `pkg check` before publication.

## Simulator contract cleanup (2026-11-14)

The unused EOS simulator protobuf and the IsOS `adapter.settings.simulator_node` setup selector were removed. Generic managed service children and mock hardware remain covered. Reproduce the focused checks from the repository root with `cargo test -p isos --bin isos managed_process_admission_is_general_across_owners`, `cargo build -p aether-runtime --lib`, `cargo test -p aether-runtime --test package_observer`, `python3 scripts/check_doc_links.py --all`, and `git diff --check`. The IsOS test passed, both package observer tests passed, and the bilingual link audit checked 367 Markdown files with no broken relative links. Complete outputs are under ignored `data/test/20261114-eos-simulator-removal/`. The initial package observer run failed because the sibling native library had not been built; it passed after the explicit library build. After installing UI dependencies with `cd ui && npm ci --ignore-scripts --no-audit --no-fund`, the catalog test, full UI suite (236 tests), typecheck, and local, hosted, and TUI builds passed; `cargo clippy --workspace --all-targets -- -D warnings` and `ISOS_AETHER_BIN_DIR=$PWD/target/debug RUST_TEST_THREADS=1 cargo test --workspace` also passed after `cargo build -p aether-runtime --bins --lib`; the first workspace test run lacked those binaries. Outputs are under ignored `data/test/20261114-ei-layer-refactor/`.

## Source frequency and pooled Vision delivery (2026-11-15)

The camera output's optional official `frequency_hz` is for checks and diagnostics, while each destination input's `desired_rate_hz.max` can cap Vision delivery. A v1 or v2 native Vision source needs `node.adapter.settings.frequency_hz` within `(0, 1000]` Hz to schedule capture into the source pool. The EOS owner checks the declared Vision transfer and relays its latest validated sample without triggering capture from the browser. Reproduce verification with `cargo fmt --all -- --check`, `cargo clippy -p aether-node-profile -p aether-runtime -p isos --all-targets -- -D warnings`, `cargo test -p aether-node-profile --tests`, `cargo build -p aether-runtime --lib`, `cargo test -p aether-runtime --test package_observer`, `ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --bin isos`, `cargo test -p isos --test edge_admission`, `python3 scripts/check_doc_links.py --all`, and from `ui/`, `npm run typecheck`, `npm test`, and `npm run build`. These checks passed; the UI suite passed 226 tests. Complete outputs are under ignored `data/test/2026-11-15-source-frequency-pools/`. Build the sibling runtime library before the package observer test in a fresh worktree.
