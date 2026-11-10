# Repository agent guide

Follow the [IslandTeam shared conventions](docs/island/conventions.v1.4.0.md). Read the [documentation index](docs/README.md) and the owning component's guide before changing a public contract. Keep English and Chinese documentation and examples aligned. This root guide is the only `AGENTS.md` in the repository. In this repository, owner docs use `start/` for the reading map and quick start, `interfaces/` for external contracts, and `design/` for current architecture and focused behavior. Keep future algorithms and target behavior only in clearly marked architecture sections; link to them from other guides instead of presenting targets as current behavior. `docs/island/` contains shared product guidance; do not revise IslandTeam-wide conventions as a repository layout change.

## Ownership

The root Cargo workspace contains the `isos` manager and `aether-*` EOS crates at source version `0.2.1`. IsOS owns saved graph revisions, package verification, Apply, deployment run identity, and its own direct children. Persistent EI and EOS owner services supervise their workers. EOS owns hardware adapters, health, dispatcher admission, and its direct simulator child. EI owns cognition and its safety gate. The Web and TUI clients are TypeScript code in `ui/`. Each domain has one authoritative contract under its `docs/<owner>/interfaces/` tree.

New graphs use format `0.2.0`, the `isos`, `ei`, and `eos` cores, and registered `<core>.<layer>.<group>.<node>@v0.2` types. The current demo has 16 nodes: an eight-node button-to-mock-arm control chain and an eight-node arm-monitor return chain. The registered type determines placement, settings, ports, capabilities, and lifecycle; the instance supplies identity and values. The runtime owns live status. Keep backend package metadata and node settings distinct from typed hardware payloads. A backend must not create another actuator route. IsOS resolves verified MuJoCo resources; EOS treats the applied graph as read-only and stops admitted actuators before terminating its simulator child.

Keep the Rust runtime path and only the C++ hardware adapters it invokes. Remove an unused component after checking build, install, runtime, and test callers. Update contracts and tests in the same slice. Current guides describe current design and code only.

## Delivery

Use focused `refactor(v2)/<topic>` branches. For each slice, plan the owner boundary and acceptance cases; update the owning interface, design, fixtures, and matching first-use guidance; obtain independent design and implementation review; test the real boundary; save complete evidence under ignored `data/test/<run-id>/`; and replay setup → Apply → readiness → control → logs → Stop where applicable. Push completed branches, open reviewed PRs against the integration branch, and squash merge them in dependency order. Confirm the remote target before deleting the completed topic branch and its worktree. Preserve unrelated changes.

For substantial modules, test success, rejection, failure, and Stop using mocks plus an integration boundary. Run formatting, lint, focused tests, applicable integration tests, and a bilingual link audit. Save complete outputs in ignored `data/test/<run-id>/` and record reproducible commands and outcomes in the owning development or design guide. Format the whole Rust workspace with root `rustfmt.toml` using `cargo fmt --all`, then verify `cargo fmt --all -- --check`.

For the Rust foundation, start with the [hardware interface](docs/eos/interfaces/hardware.md), [hardware design](docs/eos/design/runtime/hardware.md), [dispatcher design](docs/eos/design/runtime/dispatcher.md), and [hardware development guide](docs/eos/design/runtime/hardware-guide.md).

## IsOS source

`crates/isos/src/app/` handles commands and requests; `domain/` admits graphs, registered types, packages, and launch inputs; `runtime/` owns live processes, owner exchanges, and logs; `interfaces/` holds boundary values. Public definitions belong in [IsOS interfaces](docs/isos/interfaces/README.md). Verify exact package pins, contributions, resources, and contained digests without special-casing package or executable names. Owners revalidate their own payload semantics.

`isos serve` hosts the local API and workbench. Run and Restart recheck the saved revision and package bytes. Stop requires owner proof and child exit. Preserve loopback Host and Origin checks; fail closed on stale or uncertain state. The API uses `/api/isos/v1/*`. Resolve the private EI programs through `ISOS_EI_BIN_DIR`, render EI settings under the selected generation, and pass only the `ei` core projection to EI. Keep role names, generated directories, launch inputs, grants, and event sources aligned.

## UI source

Keep the React 18 Web client and TypeScript Ink TUI in `ui/`. Network calls belong in `ui/src/services/`; components do not fetch directly. Both clients use the IsOS API for graph and owner functions. Keep canvas layout separate from graph settings, use shared English/Chinese copy, and preserve accessible keyboard and touch controls. Preserve `/en/isos` and `/cn/isos`; local and hosted builds use validated loopback addresses. Run UI typecheck, tests, and both browser and terminal builds before pushing.

## EOS runtime source

The Rust EOS runtime reads the applied IsOS graph, builds only declared node adapters, owns live health, and admits commands through the dispatcher. Keep device adapters isolated by registered node. Resolve the MuJoCo executable and scene from verified IsOS resources, supervise the simulator as a direct child, and communicate on its owner-only Unix socket. Stop admitted actuators before terminating the simulator. Do not mutate the applied graph or create another actuator route. Retain C++ adapter code only where the Rust runtime invokes its ABI. For a public profile or payload change, update the [EOS interfaces](docs/eos/interfaces/README.md) and both language trees; run focused unit, simulator or stub integration, failure, and Stop tests.

## EI source

EI owns cognition, mission planning, and its safety gate. Its Python package and managed programs are defined by `ei/pyproject.toml`; keep generated protobuf modules synchronized with their source contract. Public behavior belongs in [EI interfaces](docs/ei/interfaces/README.md). Store hardware identities, calibration, endpoints, and field debug evidence only in approved private locations. For EI behavior changes, run Ruff, Pyright, import rules, focused tests, and applicable full tests.
