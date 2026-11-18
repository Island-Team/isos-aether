# IsOS WebUI

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/core/ui</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS TUI](tui.md)

</details>
<!-- doc-navigation:end -->

## Table of contents

- [IsosUI documentation](#isosui-documentation)
- [Reading map](#reading-map)
- [Client configuration](#client-configuration)
- [Clients](#clients)
- [IsosUI design](#isosui-design)
- [Documentation layout verification](#documentation-layout-verification)
- [WebUI](#webui)
- [IsosUI architecture](#isosui-architecture)
- [Future targets](#future-targets)
- [Editor and dockable workbench design](#editor-and-dockable-workbench-design)
- [Type-driven configuration and selector](#type-driven-configuration-and-selector)
- [Wire projection and editing](#wire-projection-and-editing)
- [Panels and page kinds](#panels-and-page-kinds)
- [M1 reader compatibility evidence](#m1-reader-compatibility-evidence)
- [R07 browser result state](#r07-browser-result-state)
- [P07 log cursor design](#p07-log-cursor-design)
- [U04 primitive control qualification](#u04-primitive-control-qualification)
- [Graph catalog projection](#graph-catalog-projection)
- [First use: graph and workbench](#first-use-graph-and-workbench)
- [Current path](#current-path)
- [P07 log navigation](#p07-log-navigation)
- [Current R07 mock-base control](#current-r07-mock-base-control)
- [Current M4 workbench interaction](#current-m4-workbench-interaction)
- [U03 first use: workspace panels](#u03-first-use-workspace-panels)
- [Package configuration and hardware catalog](#package-configuration-and-hardware-catalog)


[中文](../../../../docs_cn/isos/core/ui/webui_cn.md)

## IsosUI documentation

[中文](../../../../docs_cn/isos/core/ui/webui_cn.md) · [Documentation](../../../README.md)

IsosUI comprises the browser workbench and terminal client for an IsOS deployment. The browser edits graph format `0.2.1` across the `isos`, `ei`, and `eos` cores and renders registered nodes in their declared layers; the terminal client presents owner state and operations. IsOS owns graph persistence, package admission, process state, and the [UI-facing node contracts](../../interfaces/api/README.md). EOS owns live control and observation. The clients use those owner interfaces without defining another graph format.

## Reading map

- [First use](webui.md) covers opening and operating the workbench.
- [Architecture](webui.md) separates current client boundaries from future UI targets.
- [Catalog projection](webui.md) explains how registered types appear in the picker.
- [Workspace design](webui.md) describes client state and owner boundaries; [browser results](webui.md#r07-browser-result-state), [log cursors](webui.md#p07-log-cursor-design), and [primitive control](webui.md#u04-primitive-control-qualification) cover focused flows.
- [Interface overview](../../interfaces/api/README.md) maps graph loading, control, logs, and panel contracts.
- [Delivery record](../../../changelog.md#isosui-delivery) records completed UI slices and their evidence.

The source is in [`ui/`](../../../../ui). Local and hosted builds are produced by the same TypeScript project.

## Client configuration

IsOS reads [`config/isosui.toml`](../../../../config/isosui.toml) for Web and terminal client defaults. `ISOSUI_CONFIG` selects another absolute TOML file. Process environment overrides TOML: `ISOSUI_DEFAULT_INTERFACE` (`web` or `tui`), `ISOSUI_LOCALE` (`en` or `cn`), `ISOSUI_STARTUP_TIMEOUT_MS` (1000–60000), and `ISOSUI_POLL_INTERVAL_MS` (500–10000). IsOS validates the resolved settings and serves non-secret values to both clients. Set persistent preferences in TOML; use environment variables for temporary overrides.

## Clients

- [WebUI design](webui.md#webui) and [interfaces](../../interfaces/api/README.md).
- [TUI design](tui.md), [interface](../../interfaces/api/README.md#tui), and [quick start](tui.md#tui-quick-start).

## IsosUI design

[WebUI architecture](webui.md) and [workbench design](webui.md) describe browser state and presentation. IsOS owns the [API](../../interfaces/api/http.md).

## Documentation layout verification

The owner reading map was checked with `python3 scripts/check_doc_links.py --all` (639 Markdown files; zero broken relative links) and `git diff --check` (passed). Complete command output is saved under ignored `data/test/2026-11-14-doc-layout/checks.log`.

Both clients read owner state through IsOS services. Neither client owns saved revisions or Stop outcomes.

## WebUI

The browser keeps unsaved graph edits and panel layout locally, then uses IsOS to validate and save a revision. It presents owner health, typed control, Vision, and logs only with current run and revision evidence. [Architecture](webui.md) describes ownership and deployment; [workbench design](webui.md) covers the detailed interactions.

## IsosUI architecture

[中文](../../../../docs_cn/isos/core/ui/webui_cn.md) · [Reading map](webui.md) · [Current design](webui.md)

The current workbench renders IsOS graph format `0.2.1` and owner catalogs. It keeps canvas geometry in browser storage, graph settings in IsOS, and network calls in `ui/src/services/`. The UI does not own actuator admission. Its control and monitor surfaces consume separate owner routes: a user action sends one fenced command, while observation is sampled and forwarded independently. IsOS owns the run and revision fence; EOS decides whether movement is admitted. Closing a monitor panel does not stop local EOS sampling.

The graph card marks the button's `activation` output as UI→EI: an event delivered over the run-fenced IsOS/EI Unix socket graph route. Primitive binding cards mark UI→EOS as a separate EOS Dispatcher control endpoint. The direct typed primitive request travels through the EOS loopback bridge and private owner IPC; it is not drawn as an executable graph port. The two labels identify different admission paths and do not imply that the button bypasses EI safety.

## Future targets

**Target design — not implemented.** The selector framework could cover additional verified package assets, including model weights, once an owning package contract declares their identity, validation, and lifecycle. The UI would display stable catalog IDs rather than accept arbitrary paths. This target does not imply model execution support in the current workbench.

**Target design — not implemented.** A future monitor view could expose per-stage timing and dropped-sample diagnostics for the asynchronous observation route. Such values need owner-defined counters and run-scoped semantics before the UI can present them as authoritative. The current UI shows the latest state, freshness, and fault/stopped status.

## Editor and dockable workbench design

[中文](../../../../docs_cn/isos/core/ui/webui_cn.md) · [First use](webui.md) · [Architecture](webui.md)

**Global inspector verification (2026-11-13).** `npm run typecheck --prefix ui`, `npm test --prefix ui` (226 passed across 54 files), `npm run build --prefix ui` (local and hosted Web bundles plus TUI), `python3 scripts/check_doc_links.py --all` (372 pages, no broken relative links), and `git diff --check` passed. Mounted tests cover package draft persistence, node, wire and multi-selection, package switching rejection, and inventory removal. Complete output is under ignored `data/test/20261113-graph-global-inspector/`.
**Adapter selector refresh verification (2026-11-13).** `npm run typecheck --prefix ui`, `npm test --prefix ui` (228 passed in 54 files), `npm run build --prefix ui` (local and hosted Web bundles plus TUI), `cargo fmt --all -- --check`, `cargo clippy -p isos --bin isos -- -D warnings`, `cargo test -p isos --bin isos domain::graph` (56 passed, one ignored), `python3 scripts/check_doc_links.py --all` (372 pages, no broken relative links), and `git diff --check` passed. Complete logs are under ignored `data/test/20261113-adapter-selector-refresh/`.

## Type-driven configuration and selector

Graph has two library views in its left sidebar. Node library derives a searchable `core → layer → group → node` tree from the IsOS catalog. Built-in nodes appear directly; package nodes show a stable presentation color and their exact `id@version` pin. If multiple package pins contribute to one group, their nodes sit in separate package folders. Package library reads the installed package inventory, including packages with no node types, and shows each package's catalog contribution using the same tree. Selecting a package displays its installed receipt and existing configuration form in the right panel. The inspector has an explicit selection target independent of the library view. Package configuration stays mounted while another target is displayed, preserving its draft; changing exact package pins still requires confirmation when that draft is dirty. A missing target resolves to the empty inspector state. The form's Reload rereads configuration; the library's Refresh rereads inventory and the revision-fenced catalog. Home owns installation and removal. The tree uses native disclosure controls and focusable type buttons; narrow screens stack the library, canvas and inspector. Package colors never affect graph admission, and the editor preserves exact pins and unsaved graph drafts.

Graph adds package nodes individually from these trees. Its former package node-set section and preview controls have been removed; the IsOS package API remains available to other clients. The UI no longer fetches node sets on Graph load or catalog refresh.

**Removal verification (2026-11-12).** UI typecheck, 217 tests across 49 files, and Web and TUI builds passed. The bilingual link audit checked 370 pages with no broken links; the IsOS documentation audit found no issues. Logs are under ignored `data/test/20261112-remove-graph-node-sets/`.

**Library verification (2026-11-12).** `cd ui && npm run typecheck`, `npm test` (217 tests in 49 files), and `npm run build` (Web and TUI) passed. `python3 scripts/check_doc_links.py --all` checked 370 pages with no broken links; `python3 crates/isos/tools/check_isos_docs.py` found no bilingual IsOS issues; `git diff --check` passed. Complete output is under ignored `data/test/20261112-generic-package-core-boundary/library-final-*.log`.

The canvas node offers an anchored Configure card as the first editing path. Basic contains required catalog settings without defaults and owner-backed adapter selectors. Advanced contains optional and defaulted catalog settings, effective default/override labels, reset controls, and the detailed adapter fields. The full inspector remains available for descriptions, ports, diagnostics, and graph wiring. The IsOS button node sends one run-fenced `button_activation@v0.2` envelope through the shared service; its event ID remains visible if the owner outcome is uncertain.

Adapter type and setting choices come from IsOS node options for the current draft. The editor first displays matching adapter families from the loaded catalog, keyed by exact package pin. While a changed draft is checked, prior owner choices remain visible but disabled; only a current successful response can authorize those choices or unblock Apply and Run. A failed refresh shows an error and keeps stale owner choices disabled. The editor renders every returned `adapter.settings.*` group by its field name and stable value, while existing non-private settings without an owner option group remain ordinary fields. A package-backed adapter exposes its declared `service_instance`; the editor never authors a private service socket. It does not infer required fields from an adapter name. IsOS and EOS validate the selected package, settings, and hardware route again at Apply.



## Wire projection and editing

Port channel shape, exact payload-type color, layer/group node marks, runtime fault outline, and wire priority/rate indicators remain distinct. The source output’s registered `frequency_hz` is fixed for that node type; a processing node may declare a different expected rate on its output. The destination input may carry a `desired_rate_hz` range with optional positive `min` and `max`. The node inspector edits this port range, while the wire inspector displays both declarations. The wire pattern reflects the destination maximum when present and shows an unbounded pattern when absent. Priority determines line color from red (nice -20) to blue (nice 19). Static topology links remain separate, with dashed ports and no payload. A live edge is offered only after IsOS confirms a runtime route.


## Panels and page kinds

Home, Graph and configuration are **work pages**: dense, aligned, scannable rows, bounded scrolling and persistent status context. Overview or explanatory landing content is a **visual page** with more spacing and imagery. Both share existing SCSS theme tokens, Island-web typography and transparent or lightly translucent surfaces; solid fill is reserved for legibility. Controls are approximately 30 px high in every layout. All controls expose focus-visible, disabled and error states; status never depends on color alone.

Implemented U03 workspace panels (`control`, `monitor`, `vision`, `logs`) each have `{open, docked, collapsed, x, y, width, height}` in browser-local storage, keyed by UI layout version and robot ID. Dragging or resizing modifies only geometry. Persist locally per robot and version; on load, reject malformed data, clamp to viewport and restore a safe default. Docking preserves panel content and subscriptions. Closing Vision disconnects its stream; hiding log follow pauses polling; monitor polling is bounded and stops when inactive. A stale active revision or lost API connection disables commands and remains visible even when panels overlap. Focus order follows the last active panel; Esc exits resize/drag without changing the graph. At narrow widths, force a stacked layout with all critical status and Stop controls visible.


On opening Home, fetch IsOS status for live readiness and the opt-in `view=current` log scope for history. Before any admitted run, pin the manager-session first package/Apply failure; after launch, pin the persisted deployment run's first ready or failure event before detail filters. While the anchor is pending, display waiting and no detail rows. Stop, PID loss, cleared active revision and manager restart do not replace the last-run scope. A log row presents timestamp, level, source, core, optional `node_path`, `event_code`, `run_id`, optional `correlation_id` and message when IsOS provides them. Browser-only errors remain separate from server provenance. Cursor polling follows IsOS's same-run rebase diagnostics, shows rotation gaps and a new-run boundary, and never concatenates unrelated runs. The [P07 log design](#p07-log-cursor-design) owns the detailed state machine; the first-use guide teaches package → graph → Apply → readiness → control → logs → stop in that order.

## M1 reader compatibility evidence

On 2026-11-01, from the IsosUI repository with Node 22.23.4 and npm 10.9.9, `npm test` passed all 62 tests in 17 files and `npm run build` passed TypeScript checking, both local and hosted production builds, and bundle verification. Complete output is saved in `data/test/20261101-g05-reader-compat/test.log` and `build.log`. The added reader tests cover server read-only guidance, unknown versions and types, unavailable catalog, and retention of additive graph fields. The UI still depends on IsOS to reject unsafe Apply requests; these controls prevent an unsupported graph from offering local edit and launch actions.

**U02 local evidence (2026-11-02).** On U02 branch `refactor(v2)/ui-edge-qos-inspector` initially committed as `cd7bd8b` and based on IsosUI master `5d39968`, with Node 22.23.4 and npm 10.9.9, `npm test` passed 135 tests in 28 files, `npm run typecheck` passed, and `npm run build` produced valid local and hosted bundles. `git diff --check` passed. Full output and exact runtime paths are in ignored `data/test/20261102-u02-edge-qos/SUMMARY.md`. The owner M4 live first-use replay remains separate.

**U03 verification (2026-11-02).** In the isolated `refactor(v2)/floating-panels` worktree with Node 22.23.4 and npm 10.9.9: `npm ci --no-audit --no-fund`; `npm test` passed 148 tests across 33 files; `npm run typecheck` passed; `npm run build` passed local and hosted webpack outputs and bundle manifests; `git diff --check` passed. The complete local output is under ignored `data/test/20261102-u03-floating/`. Focused tests cover malformed/robot-keyed layout, narrow viewport clamping, keyboard resize, mounted pending content through collapse/close/reopen, opaque cursor preservation on collapse and reset after close. This paragraph records the U03 slice gate; see the [delivery record](../../../changelog.md#isosui-delivery) for the merged workbench.

**U03 independent review fix (2026-11-02).** The Graph Stop path now creates the workspace fence synchronously before calling IsOS. A stale Home run ID cannot narrow the fence. The fence persists through pending or uncertain Stop and is released only by a fresh owner status proving a distinct known running run or a stopped-to-running transition; an unknown original identity without either proof stays fenced. Floating geometry clamps the entire panel inside the viewport, and narrow stacked layout disables Float. The focused unit tests cover those exact negative cases. `npm test` passed 153 tests across 34 files, and typecheck plus both builds passed on Node 22.23.4/npm 10.9.9. Final gate output is in `data/test/20261102-u03-floating-review-fix/`.

## R07 browser result state

`src/services/eos/primitive-command.ts` sends narrow `base.velocity` commands, and `src/services/eos/primitive-result.ts` performs guarded reads; both use the validated loopback origin and request options. The sender takes an active resolved binding, its one graph-edge target, active revision, checked `body_velocity` arguments, EOS monotonic time, positive sequence and fresh command ID; it produces exactly the public `primitive_typed` envelope, computes `payload_length_bytes` with `TextEncoder` over compact JSON, and rejects non-safe clock/deadline numbers. It never accepts a free-form target or private grant. The result client accepts only `{commandId, activeRevision}`, produces exactly the three public JSON keys `op`, `command_id`, `active_revision`, parses the six-status `DispatchResult`, verifies the returned command ID, and maps bounded transport/owner/protocol errors to a discriminated result. Both clients use one decoder so the initial send and follow-up have identical status semantics. Components use these services rather than calling `fetch` directly. A rejected HTTP or `ok:false` envelope is an error; it is never coerced to a `Rejected` command status. A service timeout or malformed success is unavailable, not `OutcomeUnknown`, because only EOS can assign that command status.

`src/components/isos/primitive-result-state.ts` supplies the pure transition model consumed by `PrimitiveBaseCard`. A record contains the captured IsOS `runId`, `activeRevision`, active binding path, generated `commandId`, last owner `DispatchResult` or null, browser phase (`sending`, `following`, `terminal`, `paused`, `unavailable`), last safe error and request generation. The only transition into `terminal` comes from a correlated EOS status in `Completed`, `Failed`, `Cancelled`, `Rejected` or `OutcomeUnknown`; `InProgress` remains `following` or `paused`. The model keeps `Failed` separate from `Rejected` and transport errors, preserves the command ID across errors, and never starts a second `primitive_typed` send after a timeout or uncertain result.

The result poll is one read at a time, with the 1-second initial delay, 10-second maximum error backoff, 60-second/12-attempt bound and hidden-tab pause in the UI contract. `InProgress` remains visible when the bound expires; manual Refresh rechecks IsOS before one read. A terminal status stops reads. If the first send response is lost, the UI may perform only this guarded result read using the generated ID, accepting `unknown command ID` as opaque unavailability. The service never retries `primitive_typed`. Vision keeps its separate read-only WebSocket and revision fence. The result card shows command ID, active revision, binding path, exact owner status or unavailable explanation and last update. It links to unfiltered current-run logs unless an EOS owner event supplies a valid matching `correlation_id`; then it may prefill that filter. It never assumes the command ID is a log correlation ID or uses a Graph edge animation as proof of release.

### Review and verification gate


For this revised design, `python3 tests/fixtures/browser_results/check.py` passes 46 unique send/read/state cases and all six statuses; a relative-link check passes 12 changed EN/CN design and fixture pages; `git diff --check` passes. Complete output is in ignored `data/test/20261102-r07-isosui-design-revision/`. This was the pre-implementation design gate; independent design and implementation reviews later passed.



### Stop terminal display correction

The R07 live replay found that the EOS card read `result.state` from an IsOS Stop `{stopped,terminal}` reply and that the workbench stored the reply as `ProcessStatus`. The separate `stopProcesses()` service type now prevents that mixup. A shared runtime decoder validates the whole terminal record, its fixed reason codes and the captured run ID before displaying confirmation. Home, Graph and EOS show the same pending, confirmed, unknown, unconfirmed and unavailable meanings. The EOS card fences pending result reads on click; each Stop path reads fresh `/runtime/process-status` after the reply without replacing the Stop outcome when status refresh fails. Component tests cover EOS outage and stale revision, late result reads, unknown and transport outcomes; pure tests cover null, missing, malformed and mismatched terminal evidence. Full gate commands and output are recorded in ignored `data/test/20261102-stop-terminal-display/`.

Independent review also required duplicate terminal process IDs to fail closed, a null terminal with nonempty `stopped` to remain unconfirmed, and explicit `IsosApiError` rejections to retain their owner identity. These cases have negative tests in the shared decoder and mounted EOS card; all three Stop entry points use the same failure projection.

## P07 log cursor design

The existing `IsosLogsPanel` uses a single mutable cursor, polls `/logs` every 1.2 seconds and clears rows on `rotated`. Its `LogLine` type only carries timestamp, level, source and message. P07 extends the service adapter in `src/services/isos/node-graph.ts` and a pure panel state reducer; React components never assemble log URLs. IsOS P07 supplies first-event and run-bound cursor semantics. Browser console lines remain in a separate section, rather than being sorted into the authoritative server stream by timestamp.

### State machine

Process status also reports `run_id_state:none|known|unavailable`. Only `none`, backed by IsOS's durable never-run marker, allows manager scope. If status says unavailable or `view=current` returns HTTP 503 `log_scope_unavailable` at `/scope`, retain prior rows as stale, stop claiming a current run and show the owner error; retry after owner recovery without guessing a manager session. A filter HTTP 400 `log_node_path_invalid` or `log_correlation_id_invalid` is attached to the respective input path. A cursor 400 at `/query/cursor` resets only the rejected query after explaining it. Alerts never echo an opaque token or filter value.

Use IsOS's accepted sequence/cursor order for display. Do not sort server rows by `timestamp` or combine browser rows into the run. Deduplicate by IsOS event identity when provided; otherwise rely on the opaque cursor and response boundary, never a timestamp/message key. Rows remain capped at 1,000. One request is in flight; visible Follow polls at a bounded interval, hidden or paused panels do not poll. A failed request preserves cursor and rows and retries after 2, 5, 10, then at most 30 seconds; returning to a visible tab rechecks immediately. On recovery, status is checked first. A `log_cursor_expired` or `log_collector_gap` response follows the server's same-run resume token and shows a persistent gap banner until the operator acknowledges it; `log_run_changed` starts a new scope and boundary instead. No blind reset to an unscoped first page. The `rotated` flag alone never causes silent row deletion.

On 2026-11-15, `npm run typecheck`, `npm test` (229 tests across 49 files), and `npm run build` (local, hosted, and TUI) passed after the fast retry, visibility recheck, and Run/Restart log-panel changes. Complete test and build output is under ignored `data/test/20261115-ui-runtime-reattach/`.

On process-control HTTP 500 `log_scope_publish_failed` at `/run_id`, show the typed failure before any child claim. A failed or uncertain durability boundary can leave either the prior scope or a surviving new run; invalidate the local request generation and re-read status plus `view=current` rather than assuming one outcome. If the new scope survived without a child, IsOS may return an unavailable anchor; show its loss diagnostic and no fabricated readiness.

The UI renders event fields as escaped text, displays source/core/run/correlation and the saved-graph pointer without treating it as a filesystem link, and copies only safe IDs. `role=log` has `aria-live=off` for rows; a separate `role=alert` reports collector errors and gaps, while `aria-live=polite` announces a new run or anchor once. Both language copies distinguish status, gap and filter-empty states. Keyboard focus remains on the filter or Jump to first control after a refresh. Source/node/correlation inputs are bounded to the IsOS contract, and invalid input never triggers a request.

Design evidence (2026-11-01): with Node 22.23.2 and npm 10.9.9, `python3 tests/fixtures/log_cursors/check_log_cursors.py` validates 31 unique cases and all ten linked bilingual documents. JSON syntax, Python compilation and `git diff --check` pass. `npm test` passes 62 tests in 17 files; `npm run build` passes type checking and both local/hosted bundles and manifests. These run against unchanged UI source, so they qualify the design artifacts and existing baseline, not P07 behavior. Complete commands and output are in ignored `data/test/20261101-p07-log-cursors-design-final/`.

## U04 primitive control qualification

Verification: `npm ci --no-audit --no-fund`; `npm test` (144 passing tests, including exact base/arm binding, ambiguity, stale run, result follow-up, Stop failure and viewport clamp); `npm run typecheck`; `npm run build`; `git diff --check`. Complete logs are local in `data/test/20261102-u04-primitive-control/`. The Node 22.23.4/npm 10.9.9 environment is the supported toolchain. This paragraph records the U04 slice gate; the [delivery record](../../../changelog.md#isosui-delivery) links the subsequent owner evidence.

### Workbench refresh qualification fix

Node 22.23.4/npm 10.9.9 verification: `npm ci --no-audit --no-fund`; `npm test` (39 files, 179 tests, including eight focused slow-timer, manual queue, mismatch, new-run health, unmount/discovery, callback identity and StrictMode cases); `npm run build` (typecheck, local and hosted bundles and manifests); `git diff --check`. The complete test and build output plus independent design review are in ignored `data/test/20261102-workbench-refresh/`. The final browser replay is an M4 qualification gate.

### Q01 command deadline order

A selected first-use run exposed a 13.9-second IsOS process-status read between the browser's EOS monotonic time sample and its four-second typed-command deadline. The request reached admission after expiry and the UI correctly showed it as unavailable. The sender now completes its final IsOS status fence before sampling EOS time, then immediately checks the latest local run/revision, binding, edit, Stop and abort state before one send. A late local change cancels the send; an expired or uncertain command ID is never retried. The owner-facing contract and mounted deferred-read tests cover this order.

## Graph catalog projection

**U01–U04 owner features are merged.** IsosUI displays the IsOS catalog; it does not register runtime node types or own their payload fields. Read the hierarchy in the same order as first use: choose a **core**, narrow to a **layer**, select an exact **package** pin or Built-in, choose a **group**, then inspect and add a **node type**. Package pin participates in catalog identity, so equal type IDs from two packages remain distinct; old package process types are omitted from authoring while saved graphs remain readable. The selected type's description explains what it does; the instance description remains editable and explains this placement. The exact type ID, settings, ports, capabilities, defaults, units, validation and compatibility are defined in the owner's `interfaces/` contract, not copied into UI data.

| Level | UI behavior | Owner and example |
| --- | --- | --- |
| Core | Select the graph owner. | IsOS owns the saved deployment graph. |
| Layer | Narrow the domain within the core. | A registered type declares its layer. |
| Group | Narrow the type family within the layer. | A registered type declares its group. |
| Node type | Show the catalog's nonempty type description, placement, settings, and typed ports; allow adding only when compatible. | Node and field definitions belong to each core's interface contract. |

The UI looks up registered records by exact `(core, layer, group, type_id)`, never by translated labels or folder names. When an older sidecar lacks `NodeType.description`, it shows the server compatibility description and marks the fallback. If a pinned package is missing or invalid, it retains the graph IDs, shows the reason, and blocks Apply. Compatible unknown fields remain in drafts; an unknown future graph version is read-only.

The node inspector follows **identity and description → package/version and availability → settings and selectors → ports and edges → validation → live status/log links**. Selectors preserve stable values; labels and descriptions are presentation only. Each edge shows nice priority and rate limit before Apply. A log link first opens the current run's earliest ready or failure event, then filters by node path and correlation ID. See [workspace design](#panels-and-page-kinds) for stale revisions, selector refresh, and panel persistence.

### Selected catalog compatibility

The picker uses exact registered placement and type identity. It keeps saved values visible when an option becomes unavailable so the operator can repair the draft before applying it.

If the first Stop transport fails while a Run is already in flight, the coordinator still waits for that Run and issues one final Stop. A failed final Stop keeps the workspace Stop fence active through a changed run ID until an explicit Stop confirms. An uncertain standalone Stop has no automatic retry.

### U01 implementation evidence (2026-11-02)


From this repository, run `npm ci`, `npm run typecheck`, `npm test`, `npm run build`, `git diff --check`, and the local-link check in `data/test/20261102-u01-catalog-selector/`. The full test suite passed (25 files, 126 tests); typecheck, local and hosted builds, manifest verification, and four-document local-link check passed. Complete outputs are under ignored `data/test/20261102-u01-catalog-selector/`. The final gates ran with Node.js 22.23.4 and npm 10.9.9.

### Exact package catalog selection evidence (2026-11-12)

The authoring catalog now keys types by placement, type ID, and exact package pin. The Package selector and quick-add show that pin; global search still works before selecting filters. Legacy managed-process contributions are absent from both authoring menus while saved graphs remain readable. `data/test/20261112-package-catalog/` records `npm run typecheck --prefix ui`, `npm test --prefix ui` (207 passed), `npm run build --prefix ui`, `cargo fmt --all -- --check`, `cargo test -p isos --bin isos domain::graph`, `git diff --check`, and the bilingual local-link audit. The backend rejects conflicting EOS runtime IDs at graph selection until the EOS registry can key by package pin.

### Graph Vision frame details evidence (2026-11-13)

The Vision page now pairs each accepted frame with its received pixel size, bound camera source, encoding, and browser-observed delivered FPS. Metadata clears with the frame on rejection, disconnect, hiding, stale revision, or Stop. Mounted tests cover native and larger frame metadata, duplicate sequence handling, FPS, and clearing. From the repository root, run `npm --prefix ui run typecheck`, `npm --prefix ui test`, `npm --prefix ui run build`, `python3 scripts/check_doc_links.py --all`, and `git diff --check`. All passed; complete output and commands are under ignored `data/test/20261113-vision-frame-details/`.

For every node family, the inspector also exposes the exact current graph instance and selected catalog declaration as read-only JSON. The declaration includes output frequency and port contracts when supplied by the owner. Vision displays the active monitor, transfer, and camera paths and the transfer interval separately from measured frame size, frame ID, capture output sequence, source sample sequence, source clock, encoding, delivered FPS, cumulative gaps in the capture output sequence, and the interval between received source timestamps. The simulator's source sample sequence is not used to count skipped capture outputs. An unset transfer interval is shown as unset; the UI does not substitute a rate or pixel size.

During Run or Restart, Home, Graph, and EOS show IsOS's `startup_phase` (`building_source`, `validating_graph`, or `starting_owners`) while status is `starting`. An unknown phase remains visible as text. Home and Graph open Logs as soon as the user starts Run; Graph keeps a direct Logs button available while the request is pending. Status polling continues during source builds so the UI can show progress before the Run response arrives.

The successful Run or Restart control reply is a launch receipt with `started`, `active_revision`, and `run_id`. Graph fetches `/runtime/process-status` after that reply before rendering process counts or feedback. The receipt is never stored as a process-status snapshot.

The graph workbench keeps warning diagnostics separate from fatal validation results. It shows the missing input path in the diagnostics panel on Check graph and saved graph load; warnings do not disable Apply. A draft edit invalidates the prior warning display until the next owner check.

## First use: graph and workbench

[中文](../../../../docs_cn/isos/core/ui/webui_cn.md) · [Design](webui.md) · [Delivery record](../../../changelog.md#isosui-delivery)

The Q03 qualification design fixes the selected installed browser replay and initial UI baseline. For current product distribution, see the [product release guide](../../../release.md). Install the unified IsOS bundle; source builds below remain useful for development.

## Current path

For the button-to-arm demo, load the package-free 17-node starter graph from **New graph**, save it, and Run. The repository's 17-node mixed demo leaves the activation input disconnected to demonstrate a warning; it cannot command the arm. In the starter graph, the button node's **Activate** control is available only while the saved revision is the active running revision. One click sends one activation through IsOS, EI, and EOS to the mock six-joint arm. The node shows the event ID and returned status and joint positions. If the outcome is uncertain, keep the displayed ID for log inspection; the UI does not replay the event.

Click **Configure** on any node to edit its basic required values and selectors in a card beside that node. Open **Advanced** there for optional settings and effective catalog defaults. An override appears only after a value is changed; **Reset to default** removes it. The side inspector retains full node, port, and diagnostic details. After an edit, adapter families appear from the loaded catalog while owner choices refresh. Previous owner choices remain visible but disabled; Apply and Run wait for the current draft response. A failed refresh shows a repair message instead of an endless loading state. Keyboard focus and Enter or Space operate the controls; Escape closes the card, and touch uses the same buttons.

Graph opens the last pool file selected in this browser, falling back to `demo` or the first available file. Graph → Advanced → **Graph file** switches files after confirming unsaved edits. **Save file** writes the bound file at its exact revision; the workbench says that saving replaces the same named file. If the file has disappeared, Save asks for a new name. **Apply graph** or **Save and Run** validates the draft, saves the pool file, and applies that exact file revision before running. If the deployment owner is unavailable, Graph still opens the selected pool file for read-only inspection. The dependency panel lists exact missing package pins and owner readiness errors; it offers a catalog release preview and approved install, or an explicit local source build in dev mode. Installing a package does not Apply or Run the graph; use Recheck dependencies after repair. It uses the global catalog where available, marks missing port details, and keeps editing, Apply, and Run disabled until the owner recovers. If a type catalog cannot be loaded, the canvas still shows the layer and group from the saved path and explicitly says that port details are unavailable.

Dependency repair check (2026-11-18): `cargo fmt --all -- --check`, `cargo test -p isos source_install`, `cargo clippy -p isos --bin isos -- -D warnings`, and from `ui/`, `npm run typecheck`, `npm test`, `npm run build:local`, and `npm run build:tui` passed. A browser replay with an unavailable owner showed the exact pinned package and both EOS/EI readiness errors. The release path previews the exact asset and requires approval; the local source path requires an absolute checkout and dev mode. Installation does not Apply or Run the graph.

If Graph rendering fails, the workbench shows the caught error text and keeps **Logs** and **Retry display** available. Retry attempts to render the current graph again without reloading the page; browser logs retain the error stack for diagnosis.

Graph render recovery check (2026-11-17): from the repository root, `(cd ui && npm run typecheck && npm test && npm run build)`, `python3 scripts/check_doc_links.py --all`, and `git diff --check` passed. The UI suite passed 243 tests across 51 files. Complete output is under ignored `data/test/20261117-graph-display-retry/`.

1. Install Node.js 22 and npm 10, then run `npm ci` once in `ui/`. Run `npm run dev` there for the standalone frontend on `127.0.0.1:16001`. It watches edits, recompiles affected modules, and reuses its local webpack cache across restarts; no production bundle build is needed for this development path. Start the IsOS loopback API and its managed EOS owner for graph and live controls. Use `npm run build` only when checking or preparing release assets; IsOS serves the resulting `dist/local` bundle.
The workbench uses one self-hosted Noto Sans SC face for UI text. The browser synthesizes bold weight; code and numeric diagnostics use the system monospace face. The font declarations load separately from the initial app bundle.

Development server check (2026-11-17): `npm run typecheck`, `npm run build`, and `npm test` passed in `ui/` (233 tests). `ISOSUI_PORT=16011 npm run dev` compiled and returned HTTP 200 on an alternate port because the IsOS server already occupied 16001. The production build verified both Web manifests and the TUI output. Complete output is in ignored `data/test/20261117-frontend-dev/`.
2. Start IsOS with `isos serve` and open `/en/isos` or `/cn/isos`. Home shows manager setup, installed package receipts, core readiness, process state, and all-source logs. An offline API is an explicit blocked state, not an empty graph.
3. Install or link an exact package with IsOS. In Graph, search the node or package library by package ID or node type and add the needed nodes individually. The search spans all cores and layers. Adding a node changes the draft graph; Apply validates the exact package pin and generated owner inputs before Run.

If EOS controls are still loading while the owner catalog is slow, wait for the current refresh to finish. The five-second background timer does not cancel an active read. Click the workbench **Refresh** to request a new owner snapshot; Invoke stays disabled until the fresh saved graph, selected catalog, process status and discovery agree. An unavailable or mismatched selected catalog leaves the control read-only with a visible reason.

The repository [README](webui.md) gives the development commands and build outputs. IsOS owns package installation and process operations.

## P07 log navigation

Open Home Logs after a package or Apply failure. The panel asks IsOS for `view=current` and shows the manager-session first package/Apply failure above the source, level, node and correlation filters. While no such event has arrived, it says the anchor is pending; an empty list is not evidence of success. After IsOS admits a deployment run, the panel switches to that run's first ready or failure event, then shows later detail rows. It keeps that run selected after Stop, process exit or manager restart until IsOS admits another run. Use Home status for live readiness and the saved/active revision; the log event explains what happened at its own time.


An empty filtered page with an advancing cursor and no diagnostic means no matching detail row yet; it is not a gap. If status reports `run_id_state:unavailable` or current logs return `log_scope_unavailable`, keep prior rows marked stale and use live status while IsOS recovers the run identity. A `log_scope_publish_failed` Run error means no child was spawned, but the durable run record may have survived; refresh status and current logs before interpreting the scope. Neither condition authorizes the UI to choose a manager session or declare readiness.

## Current R07 mock-base control


### R07 first use with a managed mock base

Apply an IsOS graph with one `base.velocity` primitive binding connected to one `mock-cxx` base, then Run it. Open EOS and wait for the base status and health to be ready. Check the displayed binding and target paths and the SI limits. Enter a bounded velocity and lease, then click **Invoke** once. The card keeps its command ID, shows the exact EOS status and follows `InProgress` only while IsOS confirms the same run and revision. Use **Refresh result** if automatic reads pause. Click **Stop deployment** through IsOS, then open **Current run logs**. The log view opens without a command-ID filter; the command ID stays on the card for comparison.



If IsOS explicitly rejects Stop, the alert says **Stop rejected by IsOS** and shows its owner message. A null terminal with reaped children is still unconfirmed; duplicate child outcomes cannot confirm Stop.

## Current M4 workbench interaction



Graph can display its saved revision while process status or readiness is still loading. Use **Reload saved graph** to request a new selected snapshot; the current canvas stays visible until the reply passes revision checks. Returning focus to Graph refreshes package types and installed packages after an external CLI install without replacing unsaved graph edits. Use **Refresh** after an external graph change to reload the full owner view. See the [graph loading contract](../../interfaces/api/client.md#graph_loading).


Pause the pointer over a node for about half a second to open its floating info card beside the pointer. Keyboard focus or the information button opens it immediately; clicking or dragging brings the selected node above neighboring nodes. Show registered type description and instance description separately, plus placement, ports and units; show live health only when reported by its owning core. Clamp the card inside the viewport, make it available on touch, and keep dragging and wire editing usable.

Graph node cards show the registered type's publisher, subscriber, service, and action roles with distinct borders and bilingual badges. A card can show multiple roles. When an older catalog omits `interaction_roles`, the canvas infers publisher or subscriber only from live output or input ports; it does not infer service or action from a name. Role styling describes communication behavior and does not grant an IPC route.

For this role and binary Vision change, from the repository root run `cargo fmt --all -- --check`, `cargo clippy -p isos -p aether-system --all-targets --offline -- -D warnings`, `cargo build -p aether-runtime --bins --offline`, `ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --offline`, `python3 scripts/check_doc_links.py --all`, and `git diff --check`; from `ui/` run `npm run typecheck`, `npm test -- --run`, and `npm run build`. These gates passed (IsOS: 360 unit and 20 integration tests; UI: 247 tests). The full `cargo test -p aether-system --offline` still fails in the unchanged `reserved_event_is_durable_and_repeated_once` test, with a subsequent poisoned-lock failure in `running_journal_failure_fences_movement`; the new Vision tests pass. End-to-end 30 FPS at the browser has not yet been measured on this branch.

Input ports sit inside the node frame with their labels clear of the handle; output ports sit outside it. Select a node and press **R** to swap its input and output sides. The labels and connected wire endpoints follow the handles. This changes only the canvas layout, not graph connections.

Port layout verification (2026-11-15): from `ui/`, run `npm run typecheck && npm test && npm run build`. Typecheck passed, 227 tests passed across 49 files, and the browser and TUI builds passed. Complete output is in ignored `data/test/20261115-graph-port-rotation/ui.log`.

Each wire exposes `priority` (nice `-20..19`, default `0`). Its source output has a fixed registered `frequency_hz`; the destination input may request a `desired_rate_hz` range with optional positive `min` and `max`. Select the destination node to edit that range. The wire inspector displays both port rates, and its line pattern reflects the destination maximum when present. Port shape identifies the channel, port color identifies the exact payload type, and line color identifies priority (red is higher; blue is lower). Static topology ports are dashed and carry no payload. A live connection is offered only after IsOS confirms an owner-supported route. Priority affects scheduling, never dispatcher safety admission.

The workspace docks or floats **control**, **monitor**, **Vision**, and **logs** independently. A floating panel has a drag handle, resize handle, focus order, close/dock action, bounded viewport, and local layout persistence. When reopening, clamp saved geometry to the viewport. On a narrow screen, stack panels and preserve the Stop action and significant connection or revision warnings. Routine messages appear briefly and remain in the browser section of Logs. Work pages use compact aligned rows and transparent or lightly translucent surfaces; visual overview pages may use wider spacing. Both share Island-web-inspired tokens, restrained borders, visible keyboard focus, controls around 30 px high in every layout.

For the full U04 primitive demo, inspect a node, check and apply its graph, then wait for EOS readiness. In the EOS control panel select one active primitive binding and inspect its single graph-resolved target; changing that target requires a graph edit and Apply. The panel shows both node descriptions and the typed primitive fields from the owner contract. Invoke through the existing loopback `primitive_typed` bridge. Show browser-local sending, then the exact `Completed`, `Failed`, `Cancelled`, `Rejected`, `InProgress` or `OutcomeUnknown` status from the correlated response; `InProgress` stays open until the authorized result read confirms an outcome. Show the current-run log and live health separately; filter logs by correlation only when an EOS owner event supplies a valid matching ID. A graph edge alone never means the command ran. Stop through IsOS.

### Edge QoS inspector

Select a wire, then choose **Check graph**. The inspector uses IsOS’s exact source pointer and shows its declared route class. Only an IsOS admitted, routed, QoS eligible, nonfixed edge offers editable priority. Nice priority is an integer from -20 (highest) to 19 (lowest), with effective weight `20 − priority`. The source expected rate and destination desired range are shown as port properties; edit the range in the destination node inspector. Enter a value and press Enter or leave the field to commit it to the draft. Each edit clears the prior route authority; choose **Check graph** again before another QoS edit. Invalid values block Save and Save/Run. The editor validates the exact draft again before Apply; if IsOS rejects it or the draft changes while validation is pending, no Apply request is sent. A design only or fixed edge remains viewable without QoS controls.

## U03 first use: workspace panels

Open Graph, then use the workspace toolbar to show Control, Monitor, Vision and Logs. Each panel can dock, float, collapse or close independently. Drag a floating title to move it; use its corner handle or arrow keys on that focused handle to resize. On narrow screens the panels stack. The Stop deployment button, IsOS connection state and active revision stay in the fixed strip above them. Stop is available when EOS or a panel is unavailable. A Stop click fences Invoke and result follow-up for that run immediately; an unknown Stop outcome remains an unknown outcome until IsOS confirms another run.

Use the EI and EOS page buttons to inspect their status and related information; the graph View menu links to those same pages. Monitor polling and Vision streaming pause when their panels are closed or collapsed. The Logs panel keeps its current-run cursor while docking or collapsing, pauses Follow while hidden, and starts a fresh status and scope check after closing and reopening. Panel geometry is stored locally for this robot and is clamped to the current viewport. See the [workspace contract](../../interfaces/api/client.md#workspace_panels).

When a saved graph contains `isos.ui.vision.monitor@v0.2`, the top navigation shows a Vision page. Each monitor node gets its own labeled view. Connect a camera output through `eos.interface.egress.vision_flow@v0.2` to the monitor, Apply, and Run. The page reads the active graph bindings from IsOS and subscribes separately for each monitor; it rejects frames from another run, revision, transfer, source, or monitor. Beside each image it shows the accepted frame's actual pixel dimensions, camera source, encoding, and delivered FPS. “Received frame” is the stream's dimensions, which can differ from package settings or the image's on-screen size; the page does not infer latency from source timestamps with different clocks. Leaving the page, editing the graph, stopping, or disconnecting clears the displayed frame details.

## Package configuration and hardware catalog

On Home, choose **Configure** beside an exact installed package pin. IsOS supplies the package's verified bilingual field descriptor and the current local configuration revision. The form edits typed fields; **Raw TOML** starts from the saved document. Validate before saving. Save sends the observed revision, so a concurrent edit must be reloaded. Configuration is local to the deployment and exact pin; saving does not change an active run. Apply again before Run when selected service inputs change.

In Graph, use the two icons at the left of the library to switch between **Node library** and **Package library**. Node library is a searchable `core → layer → group → node` tree. Built-in types appear directly; package types carry a stable package color and exact-pin badge. When multiple packages contribute to one group, that group gains package folders. Package library lists every installed exact pin, including releases without nodes; expand one to browse only its `core → layer → group → node` contributions. Select a package to inspect its receipt and configuration in the right panel. The right inspector follows the selected package, catalog type, canvas node, wire, or node group regardless of the left library view. Switching the library view leaves the inspector unchanged; selecting another item shows that item immediately. Unsaved package configuration remains available when you return to its package, and changing packages asks before discarding those edits. Refresh rereads package inventory and catalogs; configuration Reload rereads saved configuration. A node added from a package carries that exact pin. Package process nodes from old releases are absent from the authoring library; existing pinned graphs remain readable. The canvas quick-add menu also shows each exact package pin. IsOS starts only selected services during Run.

The built-in mock-arm demo runs only as its complete chain. Use **New package graph** to start an empty draft, add package hardware nodes individually, then save it as a separate graph file.

For a `0.2.1` service package, first inspect its exact release and approve the download in IsOS. Configure the installed pin on Home, then add the required package nodes in Graph. Check the graph, save and Apply, then Run. EOS shows each selected device's health; use its active binding card to send one bounded command and inspect the correlated result. Read current-run logs and Stop through IsOS. A local source maintainer can reproduce both presets with `isos pkg check --source <absolute-checkout> --version 0.2.1 --output <empty-absolute-directory> --package-config <absolute-package.toml> --graph <absolute-native-graph.json> --graph <absolute-robocasa-graph.json>` before `pkg release --checked`.

Pause over a Graph node, focus it with the keyboard, or tap its information button to inspect separate catalog type and instance descriptions, typed ports and any EOS owner-reported health. Escape or Close dismisses the clamped card. In EOS, each supported active `@v0.2` base or manipulator binding has its own control card. Check its one graph-resolved target, primitive, units and bounds; edit and Apply the graph to change that target. Wait for a known IsOS run, equal saved/active revisions and EOS ready/healthy status. Click Invoke once. Keep the displayed command ID while the browser shows Sending, then the exact EOS status. An `InProgress` result is followed by the authorized public result read for the same ID; a lost reply never resubmits. Stop deployment remains available through IsOS even if EOS control fails. See the [U04 contract](../../interfaces/api/client.md#control).

Check graph lists missing required inputs as warnings with their node and input paths. Save and Apply remain available; a disconnected path carries no data during Run. Repair the connection and Apply again to activate that path.
