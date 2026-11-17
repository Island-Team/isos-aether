# IsOS server

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS core](README.md)
  - [IsOS architecture](architecture.md)
  - [IsOS CLI](cli.md)
  - [IsOS configuration](config.md)
  - [Local core logs](logger.md)
  - [IsOS packages](pkg.md)
  - [IsOS roadmap](roadmap.md)
  - [IslandOS services](service.md)
- Subfolders:
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

## Table of contents

- [IsOS server and owners](#isos-server-and-owners)
- [Server and launcher design](#server-and-launcher-design)
- [IsOS local server](#isos-local-server)
- [Three-core button runtime](#three-core-button-runtime)
- [Emergency shutdown](#emergency-shutdown)
- [Replay evidence](#replay-evidence)
- [IsOS monitoring](#isos-monitoring)
- [IsOS UI nodes](#isos-ui-nodes)
- [Button group](#button-group)
- [Trigger node](#trigger-node)
- [Monitor group](#monitor-group)
- [Arm monitor node](#arm-monitor-node)
- [Presentation selection](#presentation-selection)
- [Vision group](#vision-group)
- [Vision monitor node](#vision-monitor-node)
- [Vision panel consumer](#vision-panel-consumer)
- [Verification](#verification)
- [Current-design documentation alignment (2026-11-10)](#current-design-documentation-alignment-2026-11-10)
- [Asynchronous demo API and UI (2026-11-10)](#asynchronous-demo-api-and-ui-2026-11-10)
- [Package configuration editing (2026-11-12)](#package-configuration-editing-2026-11-12)
- [Node catalog verification evidence](#node-catalog-verification-evidence)
- [Core documentation layout (2026-11-16)](#core-documentation-layout-2026-11-16)
- [Documentation layout verification (2026-11-16)](#documentation-layout-verification-2026-11-16)
- [Documentation verification, 2026-11-16](#documentation-verification-2026-11-16)


[中文](../../../docs_cn/isos/core/server_cn.md)

## IsOS server and owners

[中文](../../../docs_cn/isos/core/server_cn.md) · [Core index](README.md)

## Server and launcher design

This page explains the current source boundaries for contributors. The [HTTP contracts](../interfaces/api/http.md) and [owner contracts](../interfaces/manager.md) define public fields; [verification](server.md) records checks.

```text
src/
  main.rs               starts the application
  app/                  CLI, local HTTP, UI bundle
    server/routes/       HTTP handlers by domain
  domain/               graph, registry, packages, launch decisions
  runtime/              manager IPC, children, owner exchanges, logs
  interfaces/           typed boundary values
```

`app` admits local requests and parses them, then delegates decisions to `domain` and live operations to `runtime`. `domain` verifies exact pins and registered declarations without owning processes. `runtime` serializes manager mutations, supervises only IsOS-owned direct children, and retains run and stop state. `interfaces` holds exchanged values rather than a second state store. This split keeps package verification and revision checks out of transport handlers and maps typed errors once at the HTTP boundary.

The current Start/Restart wrapper contains a saved revision, ordered process specifications, resolved inputs, and ownership mode. The supervisor creates and binds the run ID; readiness and stop remain runtime responsibilities. The sections below explain package admission and output selection; future admission consolidation is described in the [architecture targets](roadmap.md#target-architecture-not-implemented).

The root `isos.sh` chooses `dev` or `product` before entering the Rust CLI. In `dev`, it selects this checkout's debug binary, EOS programs, EI programs, UI build, configuration, and portable home; `ISOS_HOME` changes the home while explicit configuration variables change their selected files. In `product`, it invokes the installed launcher under the configured prefix and selects checkout configuration and home unless explicitly overridden. The direct installed launcher detects a checkout from its current working directory when no home was selected. See [installation](config.md) for operator commands.

The CLI entry point loads runtime configuration and paths before dispatch. Bare `isos`, `web`, and `tui` ensure setup and reuse or auto-start a server. `run` starts a foreground server and deployment only when no manager IPC exists; otherwise it asks the existing manager to start the deployment. `serve` starts the control plane and persistent EI and EOS owner services in the background without starting their deployment workers. `start` requires a running manager.

Before reusing a server, the CLI checks loopback HTTP health, service and API schema, then obtains manager identity over the selected owner-only Unix socket and compares its API origin. Development mode also compares the executable path and SHA-256, so a rebuilt local binary cannot silently reuse a stale manager. When no server is healthy, the CLI refuses an active orphaned manager socket, launches the current executable as a detached `serve` child, captures its output, and waits for both HTTP health and manager IPC up to the configured startup timeout. A startup timeout is a failed readiness observation; it does not prove that the child exited.

Foreground `serve` prepares paths and a default graph, verifies the active IsOS executable, starts EI and EOS owner connection tasks, and binds separate loopback UI and API listeners plus manager IPC. Owner process startup does not block the IsOS listeners or workbench readiness. A missing or incompatible UI bundle leaves API and manager available. Readiness requires the UI listener when available and both API and manager listeners before the workbench reports ready. IsOS first probes each owner with its bounded health and run-identity reply, publishes that overview, and only asks for catalogs or other inner configuration when the corresponding operation needs it. A surviving owner socket is reattached using its exact run identity; an identity conflict remains unavailable for Run. The server then monitors its threads and owner services until interrupt or failure; Stop is coordinated by the manager and owner proof. See the [manager IPC](../interfaces/manager.md#manager_protocol), [HTTP routes](../interfaces/api/http.md), and [dependent Stop](../interfaces/core.md#core_dependent_stop) contracts.

The graph pool is the only store of editable graph bytes. A deployment binding under `config/deployments/` contains just a pool name; `Paths::node_graph_path` resolves that binding for runtime and generated-revision checks. An older deployment graph is migrated at path preparation under a migration lock: exact-byte copies bind the existing demo, while differing bytes are imported to a unique pool file before the old file is removed. Apply reads a named pool file under the graph lock, checks its exact revision, validates it, publishes generated configuration, then updates the binding. Saving a bound pool file changes its saved revision; readiness reports `pending_apply` and Run stays fenced until Apply. A malformed binding or missing file never silently selects a different graph. The [graph API](../interfaces/api/http.md#http_graph) defines the client request and errors.


For changes, update the owning contract and Chinese mirror, test success and failure, and save full output under ignored `data/test/<run-id>/`. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and relevant focused and integration tests.

The 2026-11-15 persistent-owner replay used `python3 data/test/20261115-persistent-owner-reattach/reattach_replay.py` with an isolated home and API port. It started `serve --no-ui`, observed one EOS and one EI owner and a pooled startup log, stopped IsOS, and started it again. The owner PIDs remained unchanged and the restarted IsOS API returned the retained EOS line. The replay script and server output are under ignored `data/test/20261115-persistent-owner-reattach/`. IsOS tests excluding the separately modified repository demo fixture, all 37 EOS owner tests, strict IsOS/EOS Clippy, formatting, and bilingual documentation checks passed.

The 2026-11-10 pool-source slice ran `cargo fmt --all -- --check`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" CARGO_TARGET_DIR="$PWD/target" cargo test -p isos --no-fail-fast`, `cargo clippy -p isos --all-targets -- -D warnings -A dead_code` with the same environment, and `python3 scripts/check_doc_links.py --all`. The tests cover exact and differing legacy bytes, concurrent migration, malformed bindings, absent pool files, stale Apply revisions, and pending Apply readiness. An isolated temporary-home CLI replay ran `isos config init`, then `isos config apply <home>/config/graph-pool/demo.nodes.json --revision <exact SHA-256>` with selected EOS/EI binaries and defaults; Apply succeeded, only the demo pool graph file existed, and a stale revision was rejected. Complete outputs are under ignored `data/test/graph-pool-source-20261110/`.

The 2026-11-10 source-beta replay found a test race in the headless `serve` boundary: the test sent SIGTERM after its first successful API reply, sometimes before the workbench recorded readiness, causing an expected shutdown to be reported as startup failure. The test now waits for `workbench_ready` before sending SIGTERM, and still asserts the HTTP version reply and clean exit with an unavailable UI bundle. After building the selected EOS binaries, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace --all-targets -- --test-threads=1` passed; `cargo clippy --workspace --all-targets -- -D warnings -A dead_code` passed. Strict Clippy without the dead-code allowance still fails on 18 unused-code diagnostics and remains a release qualification gap. Complete output is under ignored `data/test/20261110-v021-source-beta/`.

A 2026-11-09 graph-edit replay launched the detached control plane with `isos web`, confirmed `isos status --json` reported `stopped` with no deployment workers, dragged the EI monitor governance node, and observed `GET /api/isos/v1/graph` return 200 before and after a browser reload. The earlier foreground `isos run` process had disappeared without a Stop or owner-failure log event; its shell session had ended, which removed the UI server independently of the drag. The reproducible checks were `cargo fmt --all -- --check`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos`, `python3 scripts/check_doc_links.py --all`, and `python3 crates/isos/tools/check_isos_docs.py`; all passed. Complete outputs are in ignored `data/test/20261109-graph-edit-stability/`.

A follow-up click replay found a separate browser failure: opening EI monitor governance's Configure card rendered a missing `data_type` setting descriptor and React unmounted the graph. The EI owner now emits `data_type: "number"` for `forward_rate_hz`. The Web editor tolerates an incomplete descriptor, and a graph render error boundary keeps the workbench shell and browser log panel available. IsOS HTTP logs cannot diagnose a React render exception; the browser error log records its stack and component path. The replay opened Configure → Advanced and confirmed a numeric `forward_rate_hz` control with no browser error while the deployment remained stopped. EI focused and full tests, Ruff, Pyright, UI typecheck, tests, and browser/terminal builds passed; outputs are in ignored `data/test/20261109-node-settings-crash/`.

A graph-pool replay found that the saved `demo` file already matched the 16-node default graph, while the Web pool loader discarded the selected owner catalog. The global catalog contains only 10 IsOS/EOS types, leaving six EI nodes without definitions in a loaded pool draft. The loader now combines the revision-checked selected and global catalogs, restores browser-local positions, and treats an identical pool graph as saved. A different pool draft stays read-only when its owner types are unavailable. The browser replay loaded `demo` and found all 16 definitions, no unavailable type or missing-port cards, no browser errors, and stopped deployment. UI typecheck, tests, local/hosted browser and terminal builds, and bilingual link audits passed; outputs are in ignored `data/test/20261109-pool-demo-load/`.

The 2026-11-12 camera-editor replay selected the saved incomplete MuJoCo camera in the Web graph. A node click opened its configuration inspector. While options loaded, the inspector showed loading text; an unselected source no longer produced a catalog-unavailable notice. Selecting `native` before an adapter family exposed `cam-0`; selecting that device copied its adapter, frame, timing, and observation profile into the unsaved draft. Private-network preflight returned permission for the exact approved local and hosted UI origins. Rust formatting and strict IsOS Clippy, UI typecheck, 218 tests, browser and terminal builds, and the bilingual link audit passed. The graph was not saved or run during this replay. Evidence is under ignored `data/test/20261112-camera-editor-config/`.

The node-card action cleanup removes the Info and Configure buttons from graph cards. Selecting a node opens the persistent configuration inspector; Enter or Space selects a focused card, and the inspector exposes type, port, and data-contract details for keyboard and touch users. Runtime actions such as Activate remain on their owning node. The removed Configure popover has no remaining entry point.

The 2026-11-15 demo migration made the repository `config/graph-pool/demo.nodes.json` match the current 17-node `/graph/new` draft, including EI context inputs. The package camera draft moved to `examples/mixed-mujoco.nodes.json` so a new editor session opens an editable package-free demo. An isolated home replay ran `config init`, validated and applied the tracked demo with its exact SHA-256, then served the IsOS API: `/graph` returned `read_only:false` and no diagnostics, the selected catalog had all 17 node types at the same revision, and `/graph/validate` returned valid with no warnings. Run reached EI `running` and EOS `ready`; a button activation succeeded with joint positions `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`, the monitor returned those positions, and deployment Stop was confirmed with no active run or children. The Rust parity test, IsOS tests, UI typecheck/tests/build, formatting, strict Clippy, and bilingual documentation audits were run. Reproduction output is under ignored `data/test/20261115-demo-migration/`.

## IsOS local server

[Services](service.md) covers managed service execution. HTTP routes belong to [HTTP interfaces](../interfaces/api/http.md).

The local server exposes the loopback IsOS API and hosts the workbench. The server authenticates local requests by Host and Origin, while the manager owns graph revisions, package admission, and run identity. Selected service children are launched from verified handoffs and monitored for readiness and Stop proof. The [HTTP contracts](../interfaces/api/http.md) own request and response shapes.

## Three-core button runtime

[中文](../../../docs_cn/isos/core/server_cn.md) · [Node contract](../interfaces/node/README.md)

Graph `0.2.1` has three peer cores. IsOS owns the saved revision, Apply, launch, and run-fenced button ingress. EI and EOS expose private Unix-socket owner services and supervise their workers. EI passes a typed activation through its input, cognition, and safety stages. EOS receives it through its interface, maps it to a fixed six-joint delta in the demo skill, admits it through the dispatcher, and updates the mock arm. Duplicate event IDs are idempotent within a run; Stop fences later movement.

The owner services remain live when IsOS exits. IsOS persists the run ID and graph revision under its private runtime directory. On startup it launches or reattaches both owners asynchronously: each quick health probe gives core state and the nullable selected run identity before any catalog request. After a manager restart it compares that identity with the retained run receipt, reconnects to both services, and reports each worker independently. Explicit deployment Stop sends both Stop requests and records confirmation before deleting the run identity. The owner log pump resumes from retained byte cursors and fills IsOS's durable per-core pools for later UI connections.

If a later owner process no longer recognizes that run, the saved receipt keeps IsOS degraded and stops monitor access. The [dependent stop contract](../interfaces/core.md#core_dependent_stop) defines both proof-based Stop and local emergency shutdown.

Deployment Status checks EI, EOS, IsOS child processes, and package services concurrently, then reports their results in stable order. Its `last_error` names the failing check when available. A new Run checks available owner health identities immediately after reading the exact saved graph, before source refresh and graph admission. It checks every selected owner again after validation, before writing a run receipt or starting package services; an owner still attaching during the first check is resolved at this final fence. An owner holding an older run is reported with that run ID and revision so its Stop proof can be resolved first. Restart retains its Stop-first order before this final owner check.

The manager hashes its running executable once per process and reuses that digest in later identity replies. This keeps repeated CLI status reads from hashing the full executable each time while preserving the identity of the running binary. In a local 2026-11-17 timing replay, `./install_dev.sh` took 6.28 s with a UI rebuild and 0.47 s when unchanged; the commands and output are under ignored `data/test/20261117-owner-startup-latency/`.

The 2026-11-17 Run-stage check used `cargo test -p isos owner_deployment::tests -- --test-threads=1` (9 passed) and `cargo clippy -p isos --all-targets -- -D warnings` (passed). It also measured a stopped live deployment: `/runtime/process-status` answered in 0.032 s, while `/cores/readiness` took 21.97 s and `/cores/setup-status` took 8.99 s. Those catalog/setup costs are separate from the manager's deployment-status path. No live Run or Stop was issued for this check.

The local health overview at `/system/health` reads EI and EOS owner identities without full graph admission; a 2026-11-17 read answered in 0.002 s while EOS reported its retained failed run and EI reported idle. The Web Home displays that owner service state and process status independently of its graph, package, readiness, and setup requests. Readiness overlaps the selected catalog and EI/EOS setup identity checks, verifies generated launch inputs once, and runs on the blocking pool. Setup status runs graph validation on the blocking pool; `validate_value` makes one full admission pass. These checks retain the package source rebuild and exact admission requirements.

The browser's shared [runtime status WebSocket](../interfaces/api/http.md#http_runtime--get-apiisosv1runtimestatusws) projects the same manager and owner status as the two quick HTTP routes. Each connection probes on a 200 ms interval using blocking workers, sends only changes, and stops probing on disconnect. Browser views fall back to HTTP while disconnected; graph, readiness, logs, and command-time fences retain their separate authoritative reads. The stream carries observations, never control commands or Stop proof.

A 2026-11-17 isolated replay used `ISOS_HOME=data/test/20261117-runtime-status-stream/isolated`, `ISOS_HTTP_PORT=16130`, `ISOSUI_PORT=16131`, `target/debug/isos config init`, and `target/debug/isos serve`. A Node `ws` client with the approved Origin received an immediate stopped snapshot with EOS and EI idle, and no duplicate event over two seconds. An isolated `isos start` then produced starting and stopped stream updates; it failed during graph validation with a missing local file, so this replay does not claim a successful Run transition. The earlier isolated Run/Stop replay above supplies the owner boundary evidence. Focused WebSocket test, Rust Clippy, UI typecheck, 239 UI tests, local and TUI builds, and documentation checks passed.

An isolated 2026-11-17 replay used `ISOS_HOME=data/test/20261117-status-convergence/isolated`, `ISOS_HTTP_PORT=16120`, and `ISOSUI_PORT=16121` with `target/debug/isos config init`, `serve`, exact-revision `config apply`, `start`, and `deployment stop`. On the rebuilt manager, `/system/health` took 0.002 s, `/runtime/process-status` 0.045 s, `/cores/readiness` 1.226 s (0.954 s on repeat), and `/cores/setup-status` 1.160 s. EI and EOS both reported the new run, monitor returned a ready observation, and Stop confirmed both owners. The isolated server and idle owner services were then shut down; the existing deployment was not changed. Evidence is under the ignored isolated home.

## Emergency shutdown

The CLI `isos stop` command works without a running IsOS server and without matching a saved run ID. It sends SIGTERM to checkout-owned processes and escalates to SIGKILL if necessary. Once no matching process remains, it retires the stale owner-run receipt with an emergency audit record. A successful process exit does not prove that an actuator is physically safe. The command warns that it may interrupt active hardware and that the operator must verify the hardware state.

Local OS permissions control which processes can be signaled. `config/isos/emergency-stop.toml` or its `ISOS_STOP_CODE_SHA256` environment override can add a code check, supplied through standard input with `--code-stdin`; the code does not enter process arguments. In a source checkout, `isos stop` invokes `kill.sh` for process signaling; the script also supports a dry-run preview. The [contract](../interfaces/core.md#core_dependent_stop--local-emergency-stop) defines the command's scope and warning.

The 2026-11-17 cross-checkout owner replay used `./kill.sh --dry-run`, `./isos stop`, another dry run, and `./isos status --json`. The first preview selected the manager, both owners bound to this workspace's sockets, and the EOS worker descendant. Stop completed; the second preview selected no processes and status reported `stopped` with `emergency_signal_unproven`. This verifies process exit, not physical safety.

## Replay evidence

The 2026-11-14 headless demo replay is saved under ignored `data/test/2026-11-14-headless-demo/`. In a fresh short `ISOS_HOME`, set the executable and EI defaults variables listed below, then run `isos config init`, `isos config validate`, `isos config apply --file <pool-file> --revision <exact SHA-256>`, and `isos run --no-ui`. The API answered on `ISOS_HTTP_PORT`; `ISOSUI_PORT` had no listener. `isos status --json` reported EI running and EOS/mock hardware ready. POST `/api/isos/v1/nodes/events` accepted `headless-demo-1`; `/nodes/events/result` and `/nodes/monitor` both reported `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`. A duplicate event ID was idempotent; a stale run ID returned HTTP 409. The two warning records were from that intentional rejection. `isos deployment stop` confirmed both owners stopped. The replay also found and fixed a misleading UI URL printed by headless Run.

The 2026-11-14 CLI-only replay is saved under ignored `data/test/20261114-integrated-demo/`. With a short isolated `ISOS_HOME` (to fit the Unix socket path), `ISOS_MODE=dev`, built `target/debug` binaries, `ISOS_AETHER_BIN_DIR=$PWD/target/debug`, `ISOS_EI_BIN_DIR=$PWD/ei/bin`, `ISOS_EI_DEFAULTS_DIR=$PWD/config/ei`, `ISOS_EI_FACTORY_DEFAULTS_DIR=$PWD/config/defaults/ei`, and `EI_PYTHON=$PWD/ei/.venv/bin/python`, `isos config init`, `config validate`, exact-revision `config apply`, `serve`, and `start` reached EI running and EOS/mock hardware ready. A CLI HTTP request to `/api/isos/v1/nodes/events` succeeded; the correlated result and `/nodes/monitor` both reported joint positions `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`. `isos logs --current --level warn --format jsonl --lines 100` returned no warnings or errors for the clean run. `isos deployment stop` confirmed both owners stopped. This replay exposed and fixed a stale EI factory-default link to the old sandbox location.

The 2026-11-09 replay is saved under ignored `data/test/20261109-arm-button-integration/`. It ran `isos config init`, validation, Apply, Run, two distinct activations, duplicate retry, stale-revision rejection, unified logs, and confirmed Stop. The mock arm positions were `[0.05,-0.04,0.03,-0.02,0.01,-0.01]` then `[0.1,-0.08,0.06,-0.04,0.02,-0.02]`. Duplicate retry preserved the first positions; stale revision returned 409. Rust workspace tests, UI checks and builds, formatting, Clippy, and bilingual link audits passed. Reproduce with a fresh `ISOS_HOME`, built `target/debug` owner binaries, `ISOS_AETHER_BIN_DIR=$PWD/target/debug`, `ISOS_EI_BIN_DIR=$PWD/ei/bin`, and `EI_PYTHON=$PWD/ei/.venv/bin/python`.

The 2026-11-10 emergency Stop replay is saved under ignored `data/test/emergency-stop-20261110/`. `cargo fmt --all -- --check`, `cargo test -p aether-runtime --bin aether-profile`, focused IsOS recovery tests, `ei/.venv/bin/python -m pytest ei/tests/ei/test_owner_service.py -q`, Ruff, and `python3 scripts/check_doc_links.py --all` passed. `target/debug/isos stop` cleared stale owners without a manager, a fresh Apply and Run reached EOS ready and EI running, `/api/isos/v1/nodes/monitor` returned HTTP 200 with the observer source, and emergency Stop cleared the active mock run. The broad IsOS test run failed two catalog queue/admission cases under the selected test environment; Clippy with `-D warnings` reports unused-code warnings in IsOS test targets. The complete outputs are in the evidence directory.

## IsOS monitoring

[Logging](logger.md) describes live evidence. The API contracts are [runtime](../interfaces/api/http.md#http_runtime), [owners](../interfaces/api/http.md#http_owners), and [log cursors](../interfaces/manager.md#manager_log_cursors).

Monitoring begins with the selected run and revision. IsOS combines owner readiness and child process status, exposes current-run logs with cursors, and treats missing or stale owner evidence as unavailable. Stop remains a manager operation that requires owner proof and child exit; a visible log line alone is not completion evidence.

## IsOS UI nodes

[中文](../../../docs_cn/isos/core/server_cn.md) · [Core index](README.md)

## Button group

The `isos/ui/button` group turns a workbench activation into one run-fenced graph event.

## Trigger node

`isos.ui.button.trigger@v0.2` is the built-in workbench button. A user activation is accepted only for the active run and graph revision; IsOS emits the typed activation through its declared output. The [node event contract](../interfaces/api/client.md#button) owns the request and failure fields. The [button-to-arm path](server.md) shows the downstream owners.

## Monitor group

The `isos/ui/monitor` group displays read-only return-chain observations for the active run.

## Arm monitor node

`isos.ui.monitor.arm@v0.2` presents the return-chain arm observation. It reads the current run without issuing actuator commands and clears stale observations when the run or revision changes. The [node monitor contract](../interfaces/api/client.md#monitor) owns the response fields.

## Presentation selection

This design explains why IsOS selects presentation metadata without becoming a producer. The [data interfaces](../interfaces/api/README.md) define output types and selection; the [HTTP presentation route](../interfaces/api/http.md#http_presentations) defines the current response.

A registered output port and its `DataType` identify what a client could display. A descriptor from the owning producer would also bind representation, delivery method, endpoint identity, run ID, and graph revision. The producer owns payload bytes and endpoint authorization. IsOS does not infer image, depth, detection, or audio semantics from a package ID or type name.

The current route rejects malformed or stale requests and returns unavailable while no verified public owner descriptor is registered. It does not return a fabricated endpoint. Owner descriptor integration is an [outstanding gate](roadmap.md#target-architecture-not-implemented).

## Vision group

The `isos/ui/vision` group presents a selected graph Vision stream and its provenance.

## Vision monitor node

`isos.ui.vision.monitor@v0.2` presents the selected graph Vision stream. IsOS binds the monitor to the active graph, run, and source; the [Vision stream contract](../interfaces/api/client.md#vision) owns frame admission. The panel behavior below covers user-facing rendering.

## Vision panel consumer

The IsosUI-owned Vision node can reference an applied graph output. It is listed here so readers can follow the core → layer → group → node graph hierarchy; IsOS does not register or render this UI type itself. The selected package or owner catalog must provide any registered type declaration.

The graph reference identifies a source node path and output port. IsOS has a pure [data selector](../interfaces/api/client.md#selection), but its current [presentation route](../interfaces/api/http.md#http_presentations) returns 503 because no owner publishes a verified public descriptor. IsosUI owns layout and subscription lifetime; the producer owns payload bytes and endpoint authorization. Closing a panel does not stop the producer.

[中文](../../../docs_cn/isos/core/server_cn.md)

## Verification

This page is for contributors checking the current IsOS tree. Save complete command output under ignored `data/test/<run-id>/`; record the source commit and toolchain with each run.

The 2026-11-15 first-use consolidation moved EI and EOS preparation, diagnostics, and hardware guidance into `docs/isos/start/quick-start.md` and removed the redundant IsOS, EI, and EOS start pages. From the repository root, run `python3 scripts/check_doc_links.py --all`, `python3 crates/isos/tools/check_isos_docs.py`, `python3 tests/fixtures/jsonl_events/check_events.py`, `python3 crates/isos/tests/fixtures/reader_features/check_docs.py`, the repository-wide `check_file` audit for tracked Markdown, and `git diff --check`. All passed: 355 documentation pages and 400 tracked Markdown files had zero broken relative links, 64 English/Chinese IsOS page pairs aligned, and both fixtures passed. Full output is in ignored `data/test/20261115-unified-first-use/`.

The 2026-11-11 public-documentation release follow-up on `refactor/v2/owner-architecture-design` ran `python3 scripts/check_doc_links.py --all` (380 Markdown files, zero broken links), `python3 crates/isos/tools/check_isos_docs.py` (65 English and 65 Chinese pages, zero issues), and `git diff --check`. Complete output is in ignored `data/test/20261111-public-architecture-release/`. This documents the public-snapshot edit gate; it does not publish or qualify a public snapshot.

The 2026-11-11 owner architecture design slice on `refactor/v2/owner-architecture-design` ran `python3 scripts/check_doc_links.py --all` (380 Markdown files, zero broken links), `python3 crates/isos/tools/check_isos_docs.py` (65 English and 65 Chinese pages, zero issues), `git diff --check HEAD`, and a three-pair architecture heading-level parity check. Complete outputs and the reviewed patch are in ignored `data/test/20261111-owner-architecture-design/`. This slice changes documentation only; runtime replay and code tests are not claimed.

The 2026-11-09 documentation navigation slice on `refactor(v2)/docs-navigation-mermaid` ran `git diff --check`, `python3 scripts/check_doc_links.py --all` (376 Markdown files, zero broken links), and `python3 crates/isos/tools/check_isos_docs.py` (58 English and 58 Chinese IsOS pages, zero issues). Both node-type JSON examples parsed successfully. Complete outputs are under ignored `data/test/20261109-docs-navigation-mermaid/`.

The 2026-11-09 documentation consolidation follow-up on the same branch checked all 175 fenced JSON examples for parseability and readable multiline formatting, ran `python3 scripts/check_doc_links.py --all` (364 pages, zero broken links), `python3 crates/isos/tools/check_isos_docs.py` (57 pages per language, zero issues), and `git diff --check`. Complete output is under ignored `data/test/20261109-docs-consolidation/`.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
XDG_CONFIG_HOME=/tmp/isos-generic-node-empty cargo test --all-targets -- --test-threads=1
python3 tools/check_isos_docs.py
python3 tests/fixtures/control_grants/check_cases.py
cargo build --workspace
python3 tests/reader_features_cli.py
git diff --check
```

The 2026-11-05 generic-node run passed formatting, strict Clippy, 324 unit tests and 24 integration tests; one owner-dependent test was ignored. Focused two-family tests also cover registered declarations, selector IDs, typed edges, changed resources, unsafe paths, and oversized metadata. The control-grant and reader-feature documentation fixtures passed. The documentation checker found 55 English and 55 Chinese pages with zero tree, heading, relative-link, fragment, or JSON-example issues. Full outputs are under `data/test/2026-11-05-generic-node/`. A temporary empty `XDG_CONFIG_HOME` keeps tests independent of an operator's active IsOS configuration. The clean-home owner replay passed for two simulator scenes in `data/test/20261105-generic-node-bridge/replay-r29.log`: generic package install, graph Validate and Apply, Run readiness, IsOS key press/refresh/release through EOS, 20 camera frames per scene, a completed primitive command, and confirmed Stop with reaped simulator. The EOS and package-side changes used for that replay remain local and uncommitted.

The 2026-11-06 configuration run used `cargo test --all-targets --no-fail-fast`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `python3 -m unittest discover -s tests -p 'test_install_user_service.py'`, `python3 tools/check_isos_docs.py`, and `git diff --check`. All passed; the documentation checker found 56 pages per language and zero issues. It also ran the installed `isos --help` and `isos version check --json` against the selected local home. Complete Cargo and installer output is in `data/test/2026-11-06-config/`.

The config unit test checks that selected TOML wins while the example fills missing fields.

The 2026-11-08 Cargo-path repair ran `python3 crates/isos/tests/test_product_installer.py` (5 passed), including a stubbed build with Cargo only in `~/.cargo/bin`; `bash -n install.sh`; `cargo fmt --all -- --check`; workspace Clippy; and `python3 scripts/check_doc_links.py --all` (462 Markdown files, zero broken links). Complete outputs are in ignored `data/test/20261108-cargo-path/`.

The 2026-11-08 TOML/environment configuration slice on `refactor(v2)/toml-env-config` ran `cargo fmt --all -- --check`, `cargo test -p isos --bin isos app::config::tests` (8 passed), `cargo test -p isos --bin isos app::isosui::tests` (4 passed), `cargo test -p isos --bin isos app::cli::product::tests` (1 passed), `cargo test -p isos --test active_home_cli` (5 passed), and `cargo clippy --workspace --all-targets -- -D warnings` (passed). `PYTHONPATH=ei/src:ei/libexec ei/.venv/bin/python -m pytest ei/tests -q` passed 751 tests; EI Ruff checks passed. The bilingual link audit checked 462 files with zero broken links, and the IsOS documentation checker found 60 pages per language with zero issues. Complete outputs are in ignored `data/test/20261108-toml-env-config/`.


The 2026-11-08 launch recovery and graph pool slice ran `./install_dev.sh`, `cargo fmt --all -- --check`, `cargo test -p isos --bin isos` (347 passed, 3 ignored), `cargo clippy -p isos --all-targets -- -D warnings`, `npm --prefix ui run typecheck`, `npm --prefix ui test -- --run` (211 passed), and `python3 scripts/check_doc_links.py --all` (466 Markdown files, zero broken links). An isolated HTTP replay in `data/test/20261108-launch-readiness-recovery/replay.py` rejected an obsolete graph, created a blank draft with its saved-byte revision, stored and listed a separate named pool draft, rejected a stale pool revision, replaced the active graph by exact revision, and rejected Run of the empty graph with `graph_has_no_processes` in both API diagnostics and the unified event log. A second isolated replay changed a copied EOS binary after Apply; readiness, Run, and the unified event log all reported `generated_launch_input_mismatch`. Complete outputs are under `data/test/20261108-launch-readiness-recovery/`.
The 2026-11-08 server launcher and documentation consolidation slice on `refactor(v2)/server-launcher-docs` ran `cargo fmt --all`, `cargo fmt --all -- --check`, `cargo clippy -p isos --all-targets -- -D warnings`, and `cargo test -p isos --test cli_guide` (5 passed). `python3 crates/isos/tools/check_isos_docs.py` found 56 English and 56 Chinese IsOS pages with zero issues; `python3 scripts/check_doc_links.py --all` checked 454 Markdown files with zero broken relative links. `cargo test -p isos --bin isos app::server::tests -- --test-threads=1` passed 19 tests and failed two bounded-worker queue tests (`global_type_and_node_set_reads_share_bounded_workers` and `selected_catalog_queue_is_bounded_and_does_not_block_runtime`); the same two failed in the concurrent run. This slice did not change those handlers. Complete outputs are under ignored `data/test/20261108-server-launcher-docs/`.

Independent review found that the UI could exit during the API/manager readiness wait. The shared wait now checks UI liveness before reporting ready; formatting, strict Clippy, and the CLI guide tests passed again after that correction.

After rebasing this slice onto `master` on 2026-11-08, `cargo fmt --all -- --check`, `cargo clippy -p isos --all-targets -- -D warnings`, `cargo test -p isos --test cli_guide` (5 passed), and `cargo build --workspace` passed. `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos --bin isos -- --test-threads=1` passed 347 tests with 3 ignored, including all 22 active server tests. The first server run in this separate worktree omitted `ISOS_AETHER_BIN_DIR` and failed two catalog tests because the selected EOS build was absent there; rerunning after `cargo build --workspace` passed. The IsOS checker found 57 English and 57 Chinese pages with zero issues; the bilingual link audit checked 456 Markdown files with zero broken relative links. Complete outputs are under ignored `data/test/20261108-server-launcher-rebase/`.

[中文](../../../docs_cn/isos/core/server_cn.md)

## Current-design documentation alignment (2026-11-10)

On `refactor(v2)/docs-current-design`, `python3 scripts/check_doc_links.py --all` checked 378 Markdown files with zero broken relative links, `python3 crates/isos/tools/check_isos_docs.py` checked 61 English and 61 Chinese IsOS pages with zero issues, and `git diff --check` passed. Complete output is in ignored `data/test/20261110-docs-current-design/`. The graph template was parsed to verify 16 nodes and 15 declared inputs across the three cores. This was documentation-only; runtime behavior was not changed or replayed.

## Asynchronous demo API and UI (2026-11-10)

On `refactor(v2)/async-isos-ui`, `cargo fmt --all` and `cargo fmt --all -- --check` passed. `cargo test -p isos --bin isos app::server::routes::nodes::tests` passed 7 tests. Strict `cargo clippy -p isos --all-targets -- -D warnings` failed on 18 existing dead-code diagnostics outside this slice; rerunning with `-A dead_code` passed. `npm run typecheck`, `npm test` (189 passed), and `npm run build` (local, hosted, and TUI) passed in `ui/`. The IsOS documentation checker found 61 pages per language with zero issues; the link audit checked 378 Markdown files with zero broken links. Complete outputs are in ignored `data/test/async-isos-ui-20261110/`.

The combined `refactor(v2)/async-graph-test` replay used `cargo build --workspace`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace`, EI's full `pytest` (745 passed), Ruff, Pyright, and import contracts, UI tests (189 passed) and local/hosted/TUI builds, formatting, and bilingual documentation checks. Strict workspace Clippy hit 18 existing dead-code diagnostics; `cargo clippy --workspace --all-targets -- -D warnings -A dead_code` passed. An isolated real-owner replay validated and applied the starter graph, reached EI/EOS readiness, accepted three concurrent activations, read their successful results while eight monitor readers ran, confirmed the final joint position and logs, and received confirmed Stop from both owners. Complete commands, output, and the replay script are in ignored `data/test/async-graph-20261110/`.

The follow-up demo data and visual-lane slice rechecked `cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test -p isos starter`, `npm test` (190 passed), `npm run build`, and `python3 scripts/check_doc_links.py --all` (378 pages, zero broken links). A fresh isolated `isos config init` generated the revised graph, and `isos config validate` plus revision-fenced `isos config apply` accepted its explicit 10 Hz EI and 5 Hz UI settings. The prior real-owner replay covers the unchanged command and Stop path. Focused output and reproducible commands are in ignored `data/test/async-demo-visual-20261110/`.

## Package configuration editing (2026-11-12)


A follow-up rejection replay appended a duplicate `[package]` heading through a test editor. `pkg configure` rejected the TOML, retained the edited draft with mode 0600, and left `pkg config show` byte-for-byte unchanged. Commands and output are under ignored `data/test/20261112-config-draft/`.

After removing pre-editor terminal messages, a PTY smoke run opened nano directly and Ctrl+X returned `package config unchanged`; no guidance text appeared over the editor.

## Node catalog verification evidence

### Owner selector implementation evidence


From the repository root, run `cargo fmt --all && cargo fmt --all -- --check`, `cargo test -p isos --bin isos --offline`, `cargo test -p aether-runtime --bin aether-profile --offline`, `cargo clippy -p isos -p aether-runtime --bins --offline -- -D warnings`, `cargo build --workspace --bins --offline`, `ei/.venv/bin/python -m pytest ei/tests/ei/test_config_cli.py -q`, `cd ui && npm run typecheck && npm test -- --run && npm run build`, `python3 scripts/check_doc_links.py --all`, and `ei/.venv/bin/python -m unittest discover -s crates/isos/tests -p unified_owner_replay_cli.py -v`. The 2026-11-09 run passed: 349 IsOS tests, 16 EOS profile tests, 16 EI setup tests, 215 UI tests, zero broken bilingual links, and four real-owner replay cases. Full output is in ignored `data/test/20261109-owner-options-*.log`.

[中文](../../../docs_cn/isos/core/server_cn.md)

### Isolated node library integration evidence

The 2026-11-15 testing branch `test/isolated-node-library` combines shared descriptor admission, EI and EOS direct libraries, scene metadata types, and the caller audit in dependency order. Run `cargo build -p aether-runtime --bin aether-profile` first, then run `cargo fmt --all && cargo fmt --all -- --check`, `cargo clippy -p isos -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`, `cargo test -p aether-node-profile`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos -- --test-threads=1`, and `cargo test -p aether-runtime -- --test-threads=1`. Run `ei/.venv/bin/python -m pytest ei/tests/ei -q`, `ei/.venv/bin/ruff check ei/src/ei ei/tests/ei/test_isolated.py ei/tests/ei/test_existing_isolated.py ei/tests/ei/test_isolated_native.py ei/tests/ei/test_stages.py`, `ei/.venv/bin/pyright -p ei/pyproject.toml`, `PYTHONPATH=ei/src:ei/libexec:ei ei/.venv/bin/lint-imports --config ei/pyproject.toml`, `python3 scripts/check_doc_links.py --all`, and `python3 crates/isos/tools/check_isos_docs.py`. The final run passed: 686 EI tests, 339 IsOS binary tests plus its integration tests, all EOS profile and runtime tests, four import contracts, strict Clippy, formatting, relative-link checks across 571 Markdown files, and 74/74 IsOS translation pages. Complete outputs are under ignored `data/test/20261115-isolated-node-integration/`. The initial combined run exposed catalog count assertions and an unintended camera route; the declaration-only port fix and rebuilt EOS reader passed the final rerun.

### Legacy IsOS node cleanup audit

The 2026-11-15 isolated library audit found no IsOS or shared legacy path that can be removed without first replacing a current caller or changing a supported package contract. This is a source, installation, runtime, and test caller result, not an availability claim for historical node types.

| Candidate | Current caller or contract | Removal prerequisite |
| --- | --- | --- |
| `isos.eos_*` selectors and binding branches | `node_options.rs`, `edge_admission.rs`, `NodeGraphManager.tsx`, and `node-graph-edits.ts` still interpret these IDs. The checked-in `etc/eos/backends/mock-cxx.toml` is used by `tests/reader_features_cli.py` and `tests/r02_egia_client_replay.py`; typed graph fixtures and UI tests also use the IDs. | Replace the checked-in profile, replay callers, fixtures, and editor behavior with owner-correct identities, then prove the installed and runtime paths. |
| Historical package process admission | `release.rs` accepts existing exact `0.1.x` pins; the [package release contract](../interfaces/package/README.md#release) keeps schema `0.1.0` and `0.2.0` readable. | Change the package contract and qualify migration of existing pins before deleting the compatibility branch. |
| Test-only `Supervisor` and candidate skill preflight | `process/supervisor.rs` and `candidate_skill_preflight.rs` are compiled only in tests, but their regression tests exercise child lifecycle, replacement, and uncertain Stop. | Replace that boundary coverage with tests of the active owner supervisor before deleting the harness. |

Reproduce the caller audit with `rg -n 'isos\.eos_|eos_hardware|eos_process|eos_dispatcher' crates/isos/src ui/src etc/eos tests` and `rg -n 'candidate_skill_preflight|Supervisor|reader_features_cli|mock-cxx.toml' crates/isos/src tests docs`. Run `cargo fmt --all -- --check`, `cargo test -p isos --bin isos historical --offline`, `cargo test -p isos --test edge_admission --offline`, `python3 scripts/check_doc_links.py --all`, and `python3 crates/isos/tools/check_isos_docs.py`. The audit passed one historical package test, eight edge tests, formatting, and both documentation checks. Complete output is under ignored `data/test/20261115-isos-dead-code-audit/`.

### Strict port contract evidence

The 2026-11-15 isolated data-type slice checked the new shared JSON descriptor admission rules and bilingual contract. From the repository root, run `cargo fmt --all && cargo fmt --all -- --check`, `cargo test -p isos --test edge_admission --offline`, `cargo test -p isos --bin isos data_type --offline`, `cargo clippy -p isos --test edge_admission --bin isos --offline -- -D warnings`, `python3 scripts/check_doc_links.py --all`, and `python3 crates/isos/tools/check_isos_docs.py`. All passed; complete output is under ignored `data/test/20261115-isolated-data-types/`.

Run `cargo fmt --all -- --check`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos`, `cargo test -p aether-node-profile`, `cd ui && npm run typecheck && npm test && npm run build`, `python3 scripts/check_doc_links.py --all`, and `python3 crates/isos/tools/check_isos_docs.py`. The 2026-11-09 run passed the Rust and UI suites, both UI builds, formatting, and both documentation audits. Complete command output is in ignored `data/test/20261109-strict-node-ports/`. A nonfatal Clippy run passed with existing IsOS dead-code warnings; `-D warnings` remains blocked by those warnings.

### Connected button demo evidence

The first connected button slice contained eight nodes and seven typed edges. The current demo also includes the eight-node observation path described below. IsOS admits a run-fenced button event, EI passes it through input, cognition, and safety, and EOS converts it to a bounded six-joint delta for the mock arm. The built-in type catalogs replaced the old health-only demo IDs. From the repository root, build `isos` and `aether-runtime` binaries, then run `ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --all-targets`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets`, `cd ui && npm run typecheck && npm test && npm run build`, `python3 scripts/check_doc_links.py --all`, and `python3 crates/isos/tools/check_isos_docs.py`.

The 2026-11-09 isolated replay ran init → validate → Apply → Run → readiness → control → logs → confirmed Stop. One click returned `[0.05,-0.04,0.03,-0.02,0.01,-0.01]` radians; retrying its event ID did not move twice; a new ID accumulated the delta; a stale revision returned 409. Complete command output is under ignored `data/test/20261109-arm-button-integration/`.

### Mock arm monitor replay evidence

The 2026-11-12 node-set collision slice restored the saved demo to its valid 16-node chain. The Graph editor now offers an empty package draft so a package set does not inherit demo execution nodes. Preview keeps existing nodes and assigns deterministic suffixes to added path or EOS hardware ID collisions; it reports bounded rejection reasons when the combined execution mode is unsupported. Verification used `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace --all-targets --quiet -- --test-threads=1`, UI typecheck/tests/build, and both documentation audits. An installed generic package set preview succeeded on an empty draft and on an existing package graph with renamed additions; complete outputs are under ignored `data/test/20261112-generic-package-core-boundary/`.

The 16-node starter adds eight read-only return nodes bound to the same mock arm. The combined replay used `cargo fmt --all -- --check`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --all-targets`, `ei/.venv/bin/python -m pytest ei/tests -q`, `cd ui && npm run typecheck && npm test && npm run build`, and the bilingual link audits. An isolated `config init` → validate → Apply → Run → readiness → button activation → monitor read → logs → confirmed Stop replay returned six zeros before movement, the exact six commanded positions afterward, and null monitor values after Stop. The active saved graph was then replaced directly and the browser showed the same positions on both the node and monitor panel after a click. Complete outputs and payloads are in ignored `data/test/20261109-arm-monitor-integration/`.

## Core documentation layout (2026-11-16)

The IsOS design pages were consolidated under `docs/isos/core/*.md`, with aligned Chinese pages under `docs_cn/isos/core/`. From the repository root, run `python3 scripts/check_doc_links.py --all`, `python3 crates/isos/tools/check_isos_docs.py`, `python3 crates/isos/tests/fixtures/control_grants/check_cases.py`, `python3 crates/isos/tests/fixtures/reader_features/check_docs.py`, and `git diff --check`. All passed: 341 Markdown files had zero broken relative links, 57 IsOS page pairs had zero issues, both fixture checks passed, and the diff had no whitespace errors. Complete output is under ignored `data/test/20261116-isos-core-docs/checks.log`.

## Documentation layout verification (2026-11-16)

The root conventions and release guides, IsOS core server and UI guides, and frontend interfaces were reorganized without changing runtime behavior. From the repository root, run `python3 scripts/check_doc_links.py --all`, `python3 crates/isos/tools/check_isos_docs.py`, `python3 crates/isos/tests/fixtures/control_grants/check_cases.py`, `python3 crates/isos/tests/fixtures/reader_features/check_docs.py`, `python3 ui/tests/fixtures/log_cursors/check_log_cursors.py`, `python3 ui/tests/fixtures/browser_results/check.py`, and `git diff --check`. All passed: 324 Markdown files had zero broken relative links, 59 IsOS English/Chinese pairs had zero issues, four focused documentation fixtures passed, and the diff had no whitespace errors. Complete output is under ignored `data/test/20261116-isos-doc-layout-round2/checks.log`.

## Documentation verification, 2026-11-16

From the repository root, `python3 scripts/check_doc_links.py --all`, `python3 crates/isos/tools/check_isos_docs.py`, and the focused R03, R07, P06, control-grant, reader-feature, and UI fixture check scripts passed after the interface-contract edit. Full output is in ignored `data/test/20261116-interface-contract-focus/checks.log`.
