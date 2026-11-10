# First use: graph and workbench

[中文](../../docs_cn/isosui/usage_cn.md) · [Design](design.md) · [TODO](todo.md)

The Q03 qualification design fixes the selected installed browser replay and initial UI baseline. For current product distribution, see the [product release guide](../island/release.md). Install the unified IsOS bundle; source builds below remain useful for development.

## Current path

For the current button-to-arm demo, load the current node demo, save it, and Run. The button node's **Activate** control is available only while the saved revision is the active running revision. One click sends one activation through IsOS, EI, and EOS to the mock six-joint arm. The node shows the event ID and returned status and joint positions. If the outcome is uncertain, keep the displayed ID for log inspection; the UI does not replay the event.

Click **Configure** on any node to edit its basic required values and selectors in a card beside that node. Open **Advanced** there for optional settings and effective catalog defaults. An override appears only after a value is changed; **Reset to default** removes it. The side inspector retains full node, port, and diagnostic details. Keyboard focus and Enter or Space operate the controls; Escape closes the card, and touch uses the same buttons.

Graph opens the last pool file selected in this browser, falling back to `demo` or the first available file. Graph → Advanced → **Graph file** switches files after confirming unsaved edits. **Save file** writes the bound file at its exact revision; the workbench says that saving replaces the same named file. If the file has disappeared, Save asks for a new name. **Apply graph** or **Save and Run** validates the draft, saves the pool file, and applies that exact file revision before running. If a type catalog cannot be loaded, the canvas still shows the layer and group from the saved path and explicitly says that port details are unavailable.

1. Install Node.js 22 and npm 10, then run `npm ci` and `npm run build` in this repository. Select the resulting `dist/local` build in IsOS. Alternatively, `npm run dev` starts the standalone frontend on `127.0.0.1:16001`; the IsOS loopback API and its managed EOS owner must be running for graph and live controls.
2. Start IsOS with `isos serve` and open `/en/isos` or `/cn/isos`. Home shows manager setup, installed package receipts, core readiness, process state, and all-source logs. An offline API is an explicit blocked state, not an empty graph.
3. Install or link the exact `island/mujoco-sim@0.1.1` package in IsOS. In Graph, choose the installed `sim-hardware` node set, preview its additions, and select a scene. In Graph, browse **core → layer → group → node type**. Select a scene from the searchable `/node-options` group for `isos/eos_process/mujoco/simulator` and choose each generic camera/base/manipulator device from its own exact `node_path` group. The device choice writes `adapter.settings.device_id` and `adapter.settings.simulator_node` together.

If EOS controls are still loading while the owner catalog is slow, wait for the current refresh to finish. The five-second background timer does not cancel an active read. Click the workbench **Refresh** to request a new owner snapshot; Invoke stays disabled until the fresh saved graph, selected catalog, process status and discovery agree. An unavailable or mismatched selected catalog leaves the control read-only with a visible reason.

The repository [README](../../README.md) gives the development commands and build outputs. IsOS owns package installation and process operations.

## P07 log navigation

Open Home Logs after a package or Apply failure. The panel asks IsOS for `view=current` and shows the manager-session first package/Apply failure above the source, level, node and correlation filters. While no such event has arrived, it says the anchor is pending; an empty list is not evidence of success. After IsOS admits a deployment run, the panel switches to that run's first ready or failure event, then shows later detail rows. It keeps that run selected after Stop, process exit or manager restart until IsOS admits another run. Use Home status for live readiness and the saved/active revision; the log event explains what happened at its own time.


An empty filtered page with an advancing cursor and no diagnostic means no matching detail row yet; it is not a gap. If status reports `run_id_state:unavailable` or current logs return `log_scope_unavailable`, keep prior rows marked stale and use live status while IsOS recovers the run identity. A `log_scope_publish_failed` Run error means no child was spawned, but the durable run record may have survived; refresh status and current logs before interpreting the scope. Neither condition authorizes the UI to choose a manager session or declare readiness.

## Current R07 mock-base control


### R07 first use with a managed mock base

Apply an IsOS graph with one `base.velocity` primitive binding connected to one `mock-cxx` base, then Run it. Open EOS and wait for the base status and health to be ready. Check the displayed binding and target paths and the SI limits. Enter a bounded velocity and lease, then click **Invoke** once. The card keeps its command ID, shows the exact EOS status and follows `InProgress` only while IsOS confirms the same run and revision. Use **Refresh result** if automatic reads pause. Click **Stop deployment** through IsOS, then open **Current run logs**. The log view opens without a command-ID filter; the command ID stays on the card for comparison.



If IsOS explicitly rejects Stop, the alert says **Stop rejected by IsOS** and shows its owner message. A null terminal with reaped children is still unconfirmed; duplicate child outcomes cannot confirm Stop.

## Current M4 workbench interaction



Graph can display its saved revision while process status, readiness or package node sets are still loading. Use **Reload saved graph** to request a new selected snapshot; the current canvas stays visible until the reply passes revision checks. Home polls process status every five seconds; use **Refresh** after an external package or graph change to reload the full owner view. See the [graph loading contract](interfaces/graph-loading.md).


Pause the pointer over a node for about half a second to open its floating info card beside the pointer. Keyboard focus or the information button opens it immediately; clicking or dragging brings the selected node above neighboring nodes. Show registered type description and instance description separately, plus placement, ports and units; show live health only when reported by its owning core. Clamp the card inside the viewport, make it available on touch, and keep dragging and wire editing usable.

Each wire exposes `priority` (nice `-20..19`, default `0`) and optional `frequency_hz` (positive finite delivery cap). The inspector edits them against an IsOS validation result. Labels show both values at normal zoom and in keyboard focus text; the legend explains priority and frequency separately from transport and payload type. Port shape identifies the channel, port color identifies the exact payload type, line pattern identifies its frequency cap, and line color identifies priority (red is higher; blue is lower). Static topology ports are dashed and carry no payload. A live connection is offered only after IsOS confirms an owner-supported route. Priority affects scheduling, never dispatcher safety admission.

The workspace docks or floats **control**, **monitor**, **Vision**, and **logs** independently. A floating panel has a drag handle, resize handle, focus order, close/dock action, bounded viewport, and local layout persistence. When reopening, clamp saved geometry to the viewport. On a narrow screen, stack panels and preserve the Stop action and significant connection or revision warnings. Routine messages appear briefly and remain in the browser section of Logs. Work pages use compact aligned rows and transparent or lightly translucent surfaces; visual overview pages may use wider spacing. Both share Island-web-inspired tokens, restrained borders, visible keyboard focus, controls around 30 px high in every layout.

For the full U04 primitive demo, inspect a node, check and apply its graph, then wait for EOS readiness. In the EOS control panel select one active primitive binding and inspect its single graph-resolved target; changing that target requires a graph edit and Apply. The panel shows both node descriptions and the typed primitive fields from the owner contract. Invoke through the existing loopback `primitive_typed` bridge. Show browser-local sending, then the exact `Completed`, `Failed`, `Cancelled`, `Rejected`, `InProgress` or `OutcomeUnknown` status from the correlated response; `InProgress` stays open until the authorized result read confirms an outcome. Show the current-run log and live health separately; filter logs by correlation only when an EOS owner event supplies a valid matching ID. A graph edge alone never means the command ran. Stop through IsOS.

### Edge QoS inspector

Select a wire, then choose **Check graph**. The inspector uses IsOS’s exact source pointer and shows its declared route class. Only an IsOS admitted, routed, QoS eligible, nonfixed edge offers editable controls. Nice priority is an integer from -20 (highest) to 19 (lowest), with effective weight `20 − priority`; an empty frequency cap means unlimited. The producer expected rate is a separate hint. Enter a value and press Enter or leave the field to commit it to the draft. Each edit clears the prior route authority; choose **Check graph** again before another QoS edit. Invalid values block Save and Save/Run. The editor validates the exact draft again before Apply; if IsOS rejects it or the draft changes while validation is pending, no Apply request is sent. A design only or fixed edge remains viewable without QoS controls.

## U04 node card and primitive controls

Pause over a Graph node, focus it with the keyboard, or tap its information button to inspect separate catalog type and instance descriptions, typed ports and any EOS owner-reported health. Escape or Close dismisses the clamped card. In EOS, each supported active `@v0.2` base or manipulator binding has its own control card. Check its one graph-resolved target, primitive, units and bounds; edit and Apply the graph to change that target. Wait for a known IsOS run, equal saved/active revisions and EOS ready/healthy status. Click Invoke once. Keep the displayed command ID while the browser shows Sending, then the exact EOS status. An `InProgress` result is followed by the authorized public result read for the same ID; a lost reply never resubmits. Stop deployment remains available through IsOS even if EOS control fails. See the [U04 contract](interfaces/control.md).

## U03 first use: workspace panels

Open Graph, then use the workspace toolbar to show Control, Monitor, Vision and Logs. Each panel can dock, float, collapse or close independently. Drag a floating title to move it; use its corner handle or arrow keys on that focused handle to resize. On narrow screens the panels stack. The Stop deployment button, IsOS connection state and active revision stay in the fixed strip above them. Stop is available when EOS or a panel is unavailable. A Stop click fences Invoke and result follow-up for that run immediately; an unknown Stop outcome remains an unknown outcome until IsOS confirms another run.

The control panel keeps each command ID and owner result while you switch pages, dock or collapse it. Closing Control hides it without creating a second client; reopen it from the toolbar. Monitor polling and Vision streaming pause when their panels are closed or collapsed. The Logs panel keeps its current-run cursor while docking or collapsing, pauses Follow while hidden, and starts a fresh status and scope check after closing and reopening. Panel geometry is stored locally for this robot and is clamped to the current viewport. See the [workspace contract](interfaces/workspace-panels.md).

For selected Vision evidence, open the Vision panel only after Home reports the current run ready. The panel uses IsOS's selected gate and source, subscribes to the public EOS bridge, and shows the run, gate generation, stream, JPEG digest, delivered FPS, and locally skipped generations. `No new frame` means the attested stream is ready but has not delivered an image; `Frame rejected` names a frame that failed local provenance or digest checks. Stop, a changed run or revision, or hiding the panel removes the image. See the [Vision provenance contract](interfaces/workspace-panels.md). The selected Q03 camera and merged-head Q04 final-5 Vision replays passed; final-5 used a simulated hosted Origin on loopback.

## Q03 qualification replay
