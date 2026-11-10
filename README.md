# IslandOS · IsOS

**Build, run, and understand embodied agents.**

[![Source version](https://img.shields.io/badge/source-0.2.1-175c79)](Cargo.toml) [![Release status](https://img.shields.io/badge/status-release%20qualification-e8a33d)](docs/isos/start/quick-start.md) [![Graph format](https://img.shields.io/badge/graph-0.2.0-467b65)](docs/isos/interfaces/README.md) [![License](https://img.shields.io/badge/license-Apache--2.0-465a75)](LICENSE)

[中文](README_cn.md) · [Get started](docs/isos/start/README.md) · [Architecture](docs/isos/design/architecture.md) · [Documentation](docs/README.md)

<p align="center"><img src="docs/assets/island-team-hero.png" width="720" alt="IslandOS connecting sensing, intelligence, guarded execution, and a robot arm"></p>

IslandOS connects a saved deployment graph to robot execution and cognition. IsOS makes the path from observation to decision, admitted action, and outcome inspectable through one command and two interfaces.

## Start in one checkout

Product mode is the default. This checkout keeps its machine-local selection in ignored `config/isos/launch.toml`; on this device it is `dev`. To build and run from source without installing a release:

```sh
./install_dev.sh
./isos doctor
./isos web
```

`./isos` and `./isos.sh` use `target/debug/isos`, `target/debug/aether-*`, `ei/bin`, and `ui/dist/local` in dev mode. Rebuild changed components and restart the manager after source changes. Set `ISOS_MODE=product ./isos web` for one product-mode launch, or put `mode = "product"` under `[launch]` in `config/isos/launch.toml`. Product mode needs a release installed with `./install.sh --prefix "$HOME/.local/isos"`; `install.sh` always builds and stages a product release. See [installation](docs/isos/start/quick-start.md) for bundles and upgrades.

| Operator command | Action |
| --- | --- |
| `isos` | Open the interface selected in `config/isosui.toml` (`web` by default). |
| `isos web` · `isos tui` | Open the Web or terminal client; start the IsOS server if needed. Closing a client leaves the server running. |
| `isos doctor` | Check version, launch mode, manager presence, interface support, and local assets. |
| `isos update` · `isos upgrade [version]` | List available releases without changing the installation, or verify and stage an upgrade. |

IsOS owns Apply, Run, logs, and deployment Stop. EOS alone admits actuator commands. The [first-use guide](docs/isos/start/README.md) walks through a deployment. In the TUI, `1`–`4` switch views, `f` refreshes, `r` and `s` confirm Run and Stop on a second keypress, and `q` closes the client.

## Three cores, one operator surface

| Core | Responsibility | Explore |
| --- | --- | --- |
| **IsOS** | Verify packages, save graph revisions, Apply, and supervise owners and shared resources. | [Architecture](docs/isos/design/architecture.md) · [Start](docs/isos/start/README.md) |
| **EOS** | Run hardware adapters, health, Dispatcher admission, and its direct simulator child. | [Architecture](docs/eos/design/architecture.md) · [Start](docs/eos/start/README.md) |
| **EI** | Run cognition stages and its local safety gate. | [Architecture](docs/ei/design/architecture.md) · [Guide](docs/ei/README.md) |

The React Web client and TypeScript Ink TUI live together in [`ui/`](docs/isosui/README.md). Both attach only to the IsOS API. New graphs use format `0.2.0`, one `isos` core, and registered `isos.<layer>.<name>@v0.2` types. Source, graph, wire, and package versions are independent; MuJoCo is a separately verified `island/mujoco-sim@0.1.1` package. [Owner contracts](docs/island/architecture.md#cross-owner-contracts) define the boundaries.

## Configuration

| Purpose | File | Selection and precedence |
| --- | --- | --- |
| Launch mode | [`config/isos/launch.example.toml`](config/isos/launch.example.toml) | Product by default; ignored `config/isos/launch.toml` or `ISOS_MODE` selects dev for a checkout. |
| Product installation | [`config/isos/install.toml`](config/isos/install.toml) | `./install.sh --prefix` overrides `ISOS_INSTALL_ROOT`, then the TOML prefix. Release repository and channel have matching `ISOS_RELEASE_*` overrides. |
| IsOS runtime | [`config/isos/isos.toml`](config/isos/isos.toml) | `ISOS_CONFIG` selects an absolute TOML path. Process environment overrides selected TOML, which overrides [`isos.example.toml`](config/isos/isos.example.toml). The retired `active.env` is not read. |
| EI runtime | [`config/ei/`](config/ei/) | Factory TOML is overlaid by deployment TOML. Ordinary settings stay in TOML; API keys and development SSH values use the root [`.env.example`](.env.example) template as an environment fallback. |
| Web and TUI | [`config/isosui.toml`](config/isosui.toml) | `ISOSUI_CONFIG` selects an absolute file; `ISOSUI_DEFAULT_INTERFACE`, `ISOSUI_LOCALE`, and client timing variables override its fields. |

The installed launcher supplies bundled config paths unless you set overrides. See the [configuration guide](docs/isos/design/app/configuration.md) for modes, ports, data locations, and migration.

## Develop and verify

`crates/isos/` contains the manager, `crates/aether-*/` the EOS Rust runtime, `ei/` the Python runtime, and `ui/` both clients. Run Rust formatting, Clippy, and focused tests from the root; run EI's platform recipe and checks in `ei/`; run `npm ci && npm run typecheck && npm test && npm run build` in `ui/`. The UI build emits local and hosted Web bundles plus the TUI. Follow the [development guides](docs/README.md), [conventions](docs/island/conventions.v1.4.0.md), and [agent guide](AGENTS.md); keep English and Chinese documentation aligned.
