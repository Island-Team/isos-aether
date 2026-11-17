# Repository agent guide

Follow the [IslandTeam conventions](docs/conventions.v1.4.0.md) and the [documentation index](docs/README.md). This file is a short map of the current repository, not a second source for product contracts. Before changing behavior, read the owning component's `start/`, `interfaces/`, and relevant `core/` or `design/` pages. The owning interface defines public behavior. Keep English `docs/` and Chinese `docs_cn/` pages, examples, and links aligned.

## Current source map

The workspace is at source version `0.2.3`: Rust `crates/` contains the `isos` manager and EOS `aether-*` crates; `ei/` contains the Python EI package and managed programs; `ui/` contains the React Web client and Ink TUI. Source version, graph format, package versions, and wire protocol versions are independent. The current graph format is `0.2.1`. See [IsOS](docs/isos/interfaces/README.md), [EI](docs/ei/interfaces.md#README), [EOS](docs/eos/interfaces.md#README), and [UI](docs/isos/core/ui/webui.md) for their respective boundaries.

IsOS owns saved graphs, verified packages, Apply, run identity, and the local API. EI owns cognition and its safety gate. EOS owns hardware adapters, health, and dispatcher admission. Owner processes supervise their direct workers. The applied graph is read-only to owners; commands to actuators pass through EOS dispatcher admission. Use the [architecture overview](docs/isos/core/architecture.md) for current data flow and explicitly labeled targets.

## Change workflow

Create a focused `<change-type>/<topic>` branch from the confirmed integration target. Use `fix`, `test`, `docs`, or `refactor` as the change type. Preserve unrelated changes. For a public change, update the owning contract, implementation, fixtures, tests, and first-use guidance together. Check build, installation, runtime, and test callers before removing a component.

Run checks relevant to the changed boundary. Rust uses root `rustfmt.toml`; run `cargo fmt --all` and `cargo fmt --all -- --check`, plus relevant Clippy and tests. UI checks are `npm run typecheck`, `npm test`, and the browser and TUI builds from `ui/`. EI checks are Ruff, Pyright, import rules, and relevant tests. For runtime changes, exercise success, rejection, failure, and Stop at the real boundary where applicable. Save complete evidence in ignored `data/test/<run-id>/` and record reproducible commands and outcomes in the owning guide.

Follow the [shared publication workflow](docs/conventions.v1.4.0.md#change-and-release-workflow) for review, testing branch, PR, merge, and cleanup. This repository uses branch names without `(v2)`; that local rule supersedes the branch prefix in the shared workflow. Confirm the integration target before publishing. Do not treat a local check or install as a published package release; use the [package contract](docs/isos/interfaces/package/README.md) for qualification and release rules.

## Documentation boundaries


For the Rust foundation, start with the [hardware interface](docs/eos/interfaces/node/hardware/mock.md#hardware_mock--eos-mock-arm-hardware-boundary), [hardware design](docs/eos/core/hardware/hardware.md), [dispatcher design](docs/eos/core/dispatcher/dispatcher.md), and [hardware development guide](docs/eos/core/hardware/hardware-guide.md).

Current guides describe implemented behavior. Put future behavior in the owning `core/roadmap.md` and link to it from current guides. Keep shared `docs/` conventions separate from repository-specific layout changes. Do not duplicate wire fields or payload definitions outside the owning interface. Keep private hardware identities, credentials, calibration, endpoints, and field evidence out of public source and docs.
