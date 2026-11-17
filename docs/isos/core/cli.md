# IsOS CLI

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS core](README.md)
  - [IsOS architecture](architecture.md)
  - [IsOS configuration](config.md)
  - [Local core logs](logger.md)
  - [IsOS packages](pkg.md)
  - [IsOS roadmap](roadmap.md)
  - [IsOS server](server.md)
  - [IslandOS services](service.md)
- Subfolders:
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/isos/core/cli_cn.md) · [First use](../start/quick-start.md) · [Server](server.md) · [Configuration](config.md)

The `isos` command selects a local home and manager, then performs graph, package, deployment, and diagnostic operations. The [first-use guide](../start/quick-start.md) gives an executable sequence; `isos --help` and each subcommand's `--help` are the exact option reference. This page groups commands by operator task without redefining their HTTP or manager contracts.

| Task | Commands | Owning detail |
| --- | --- | --- |
| Prepare a home | `isos config init`, `isos config validate`, `isos config apply --file <pool-file> --revision <sha256>` | [Configuration](config.md) and [graph API](../interfaces/api/http.md#http_graph) |
| Open the control plane | `isos`, `isos web`, `isos tui`, `isos serve`, `isos serve --no-ui` | [Server launch](server.md) |
| Run a deployment | `isos run`, `isos run --no-ui`, `isos start`, `isos status --json`, `isos deployment stop` | [Server and owner lifecycle](server.md) and [runtime API](../interfaces/api/http.md#http_runtime) |
| Exercise the package-free demo | `isos demo activate <event-id>`, `isos demo result <event-id>`, `isos demo monitor` | [Browser event contract](../interfaces/api/client.md) |
| Inspect evidence | `isos logs --current`, `isos doctor` | [Logging](logger.md) and [configuration](config.md) |
| Manage packages | `isos pkg install`, `isos pkg link`, `isos pkg update`, `isos pkg upgrade` | [Packages](pkg.md) and [package contract](../interfaces/package/README.md) |
| Update the product | `isos update`, `isos upgrade` | [Release](../../release.md) |
| Emergency shutdown | `isos stop` | [Dependent Stop](../interfaces/core.md#core_dependent_stop) |

Apply requires the exact current bytes of a graph-pool file. `serve` opens the control plane and idle EI/EOS owner services; it does not start deployment workers. `run` starts the deployment, while `deployment stop` stops its workers and retains the owner services for a later Run. Use `isos stop` for the separate emergency shutdown path.

The `demo` commands use the selected manager's local API. Activate and result obtain the active run ID and revision from the manager, then send the same fenced request as the workbench. Activate submits once and never retries; use a fresh event ID for each new activation, and query an uncertain result with that same ID. A non-success API response prints its JSON diagnostic and exits nonzero. These commands target only the package-free starter's `isos/ui/button/trigger`; other graph buttons use the [local HTTP API](../interfaces/api/client.md).

An isolated CLI demo replay on 2026-11-17 used a fresh short `ISOS_HOME`, `ISOS_MODE=dev`, `ISOS_CONFIG=$PWD/config/isos/isos.toml`, `ISOS_AETHER_BIN_DIR=$PWD/target/debug`, `ISOS_EI_BIN_DIR=$PWD/ei/bin`, `ISOS_EI_DEFAULTS_DIR=$PWD/config/ei`, `ISOS_EI_FACTORY_DEFAULTS_DIR=$PWD/config/defaults/ei`, and `EI_PYTHON=$PWD/ei/.venv/bin/python`. `isos config init`, `config validate <home>/config/graph-pool/demo.nodes.json`, exact-SHA-256 `config apply --revision <sha256> <pool-file>`, `serve --no-ui`, `start`, and `status --json` reached EI running and EOS/mock hardware ready on one run ID. A button activation succeeded and `/nodes/monitor` reported `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`. A deliberately stale run returned HTTP 409 `run_fence_stale`; `deployment stop` confirmed both owners and final CLI status was stopped. Evidence is under ignored `data/test/20261117-actionable-validation-cli/`.

After adding the demo commands, a second fresh home at ignored `data/test/dcli/` repeated init, validate, Apply, serve, and Start. `isos demo monitor` first returned six zeros; `isos demo activate cli-demo-2` returned `accepted`; `isos demo result cli-demo-2` returned `succeeded` with `[0.05,-0.04,0.03,-0.02,0.01,-0.01]`; the next monitor read matched those positions; and `isos deployment stop` confirmed both owners. `cargo build -p isos --bin isos` and strict IsOS Clippy passed.
