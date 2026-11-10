# Develop EOS hardware adapters

[中文](../../../../docs_cn/eos/design/runtime/hardware-guide_cn.md) · [Hardware interface](../../interfaces/hardware.md) · [Runtime design](hardware.md) · [Dispatcher design](dispatcher.md)

Start with the registered node type and its owning interface contract. The current demo EOS hardware node lives under `eos/hardware/mock/arm` in graph format `0.2.0`. Declare its capabilities, typed ports, settings, limits, health timing, and adapter family in the registry; the graph instance supplies identity and selected values. Do not create a second manifest or actuator route.

## Add or change a node

1. Update the [hardware contract](../../interfaces/hardware.md) and its Chinese mirror with each public field's type, unit, purpose, default, and validation.
2. Implement one adapter per node. Keep device communication behind the Rust HAL or its narrow CXX/C ABI boundary; transfer bounded typed samples and commands.
3. Validate settings and references before device construction. Return explicit health and status, reject stale or malformed observations, and correlate every command with its durable ID.
4. Route all movement through Dispatcher. Implement independent physical Stop at the adapter write boundary and verify the watchdog for leased movement.
5. Add mock unit tests and an integration boundary covering success, rejection, failure, restart, and Stop.

## Checks and logs

Run `cargo fmt --all`, `cargo fmt --all -- --check`, workspace Clippy, focused HAL/Dispatcher/runtime tests, applicable simulator or hardware integration, and `python3 scripts/check_doc_links.py --all`. Save complete outputs under ignored `data/test/<run-id>/`; document reproducible commands and results in the owning guide. A benchmark uses a fixed workload and reports throughput, median, p99, and environment before setting a performance target.

The built-in EOS catalog is constructed directly from the current `eos.*@v0.2` node definitions in `crates/aether-node-profile/src/registry.rs`. Verify registry and edge admission with `cargo test -p aether-node-profile`, command admission with `cargo test -p aether-dispatcher`, and runtime integration with `cargo test -p aether-runtime --lib`.

For the owner-first file layout documentation change on 2026-11-07, `python3 scripts/check_doc_links.py --all` passed (464 Markdown files, no broken relative links). Complete output is in ignored `data/test/20261107-eos-paths/doc-links.log`. EOS runtime path inputs are required or explicitly injected; IsOS owns the source, portable, and system defaults.

For the 2026-11-08 node mismatch event and sidecar placement alignment, `cargo fmt --all -- --check`, `cargo test -p aether-common -p aether-runtime --bin aether-runtime --bin aether-profile`, `cargo clippy -p aether-common -p aether-runtime --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all` passed. Complete output is in ignored `data/test/20261108-eos-node-alignment/`.

Contributed node types use the same `isos.<layer>.<name>@v0.2` identity and a nonblank description with fully described typed ports and settings. The registry rejects other graph type identities before they can enter the catalog.

## Button-to-mock-arm demonstration, 2026-11-09

The EOS projection has one live ingress, skill, dispatcher, and mock arm node. The mock arm is registered as a manipulator adapter with a `mock_arm` safety adapter. The worker stores six joint positions for one run. The built-in skill transforms an admitted activation to a fixed relative delta, and Dispatcher checks per-command and resulting position bounds before the mock adapter updates state. See the [arm control contract](../../interfaces/arm-control.md) for the wire fields and units.

Verify with `cargo test -p aether-node-profile`, `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`, `cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all`. The socket integration test covers Run, two activations, duplicate replay, stale revision rejection, and Stop. Complete command output is saved under ignored `data/test/20261109-eos-arm-demo/`.

## Mock arm observation return, 2026-11-09

The [arm monitor contract](../../interfaces/arm-monitor.md) defines four return-path nodes sourced from the mock arm sampler, one typed observation payload, and `POST /v1/monitor`. The worker samples at 100 Hz by default and immediately after movement. Advanced mock arm settings expose a 10 ms sampling period and 50 ms freshness timeout; bounded graph overrides reach the worker. Stale and invalid samples block movement until a fresh valid sample restores readiness.

## Independent mock observation source, 2026-11-10

The new demo uses `eos.hardware.mock.arm_observer@v0.2` as the zero-input first node of the return chain. It reads the existing adapter's periodic sampler; the command chain ends at the sole mock-arm actuator. The old input-bound `arm_output@v0.2` route remains valid for saved graphs. From the repository root, run `cargo fmt --all -- --check`, `cargo test -p aether-node-profile --tests`, `cargo test -p aether-runtime --test arm_activation`, `cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all`. Profile tests cover both graph routes and reject a command-chain input on the observer. The owner/worker socket test proves the observation sequence advances before activation, then continues after movement and ends on Stop. Full output is saved under ignored `data/test/eos-observer-20261110/`.

From the repository root, run `cargo fmt --all`, `cargo fmt --all -- --check`, `cargo test -p aether-node-profile --tests`, `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`, `cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all`. The unit tests cover initial zero, stale and invalid samples, recovery, immediate post-move observation, and Stop. The real owner/worker socket test covers Run, sampling progress, monitor reads, movement, stale revision rejection, and Stop. Complete output is saved under ignored `data/test/20261109-eos-arm-monitor/`.

## Monitor health and timing controls, 2026-11-09

The mock arm catalog exposes optional Advanced `sample_interval_ms` (default 10) and `freshness_timeout_ms` (default 50). The profile bounds each and requires freshness to be at least twice the sample interval. EOS passes effective values to its worker. Owner health and Status preserve the worker's `degraded` or `stale` state, while Monitor remains readable and movement stays blocked. Verify with `cargo test -p aether-node-profile --tests`, `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`, `cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all`. Complete output is in ignored `data/test/20261109-eos-monitor-health/`.

## Asynchronous demo verification (2026-11-10)

`cargo fmt --all -- --check`, `cargo clippy -p aether-runtime --all-targets -- -D warnings`, `cargo test -p aether-runtime`, and `python3 scripts/check_doc_links.py --all` passed. The real owner/worker socket test covers the acceptance receipt, result lookup, duplicate ID, monitoring, and Stop; a reservation unit test checks journal writes and single movement. Complete output is in ignored `data/test/async-eos-20261110/`.
