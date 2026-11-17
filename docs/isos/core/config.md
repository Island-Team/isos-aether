# IsOS configuration

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS core](README.md)
  - [IsOS architecture](architecture.md)
  - [IsOS CLI](cli.md)
  - [Local core logs](logger.md)
  - [IsOS packages](pkg.md)
  - [IsOS roadmap](roadmap.md)
  - [IsOS server](server.md)
  - [IslandOS services](service.md)
- Subfolders:
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/isos/core/config_cn.md) · [Core index](README.md)

## Configure IsOS

This page is for operators choosing local or production settings. IsOS reads the selected TOML file first, then fills missing fields from the complete defaults in [`config/isos/isos.example.toml`](../../../config/isos/isos.example.toml). TOML `#` comments are notes and never become settings. The default selected file is [`config/isos/isos.toml`](../../../config/isos/isos.toml); `ISOS_CONFIG` selects another file by absolute path. The selected file may contain only the values you need to change.

The root `config/` tree contains IsOS settings in `config/isos/`, IsosUI settings in `config/isosui.toml`, EI component settings in `config/ei/`, and authored deployment graphs in `config/deployments/`. EOS settings are the EOS nodes of an applied graph; there is no separate EOS TOML. IsOS renders the verified EOS projection at Apply. The source defaults and dev launcher select `target/debug` for EOS programs.

Configuration precedence is **process environment → selected TOML → checked-in TOML defaults**. TOML remains the normal place for persistent non-secret settings; set an environment variable only for a temporary deployment or a secret. The selected IsOS TOML may omit fields supplied by `isos.example.toml`. Relative paths in TOML resolve from the IsOS checkout. The installed launcher selects its bundled TOML unless `ISOS_CONFIG` names another absolute file. The retired `~/.config/isos/active.env` is not read; migrate any needed values to TOML or process environment.

For a temporary override, use the matching `ISOS_*` variable: `ISOS_HTTP_PORT` and `ISOSUI_PORT` for `[server]`; `ISOS_AETHER_BIN_DIR`, `ISOS_EI_BIN_DIR`, `ISOS_EI_DEFAULTS_DIR`, `ISOS_EI_FACTORY_DEFAULTS_DIR`, `ISOSUI_DIST_DIR`, and `ISOSUI_REPO_PATH` for `[paths]`; `ISOS_DEPLOYMENT`, `ISOS_STOP_TIMEOUT_MS`, and `ISOS_EOS_STOP_GRACE_MS` for `[deployment]`; and `ISOS_LOG_DEFAULT_SOURCE`, `ISOS_LOG_LEVEL`, `ISOS_LOG_MAX_BYTES`, `ISOS_LOG_BACKUPS`, and `ISOS_LOG_MAX_SOURCES` for `[logging]`. Package host inputs use their declared uppercase key. Invalid overrides fail before the server starts.

`ISOS_HOME=/path/to/home` selects a portable home: configuration is under `config/`, IsOS records under `data/isos/state/`, logs under `data/isos/log/`, and manager sockets under `data/isos/run/`. EOS journals and sockets use `data/eos/state/journal/` and `data/eos/run/`; EI state uses `data/ei/state/`. Without `ISOS_HOME`, user mode uses `~/.config/isos` for configuration, `${XDG_DATA_HOME:-~/.local/share}/island/<owner>` for owner data, and `${XDG_RUNTIME_DIR:-/run/user/<uid>}/island/isos` for manager runtime. `ISLAND_MODE=system` selects `/etc/island/isos`, `/var/lib/island/<owner>`, and `/run/island/<owner>` instead. Stop the owners before clearing `cache/`, `tmp/`, or stale `run/`; back up `state/` and `user/`. Use `scripts/migrate_var_data.py` to copy durable files from an older portable `var/` tree before switching defaults; the old tree stays available for operator confirmation.

To select EI, set `paths.ei_bin_dir` to the directory containing `ei` and `ei-config`, `paths.ei_defaults_dir` to the installed EI component settings, and `paths.ei_factory_defaults_dir` to `share/defaults/ei` under the same EI prefix. Their process environment equivalents are `ISOS_EI_BIN_DIR`, `ISOS_EI_DEFAULTS_DIR`, and `ISOS_EI_FACTORY_DEFAULTS_DIR`. `EI_PYTHON` selects the Python interpreter used by both EI programs. IsOS verifies the selected program and settings bytes during Apply, gives EI only its `ei_` graph layers, and starts the managed `ei` role from the selected generation.
In a source checkout, the example config resolves `paths.ei_factory_defaults_dir` to `config/defaults/ei` at the repository root. A different path fails Apply with `generated_invalid_default` at the `defaults` stage. After correcting the path, rebuild IsOS and restart an existing manager so its inherited environment uses the corrected value.
Factory-default path replay (2026-11-17): after the sample path correction, `cargo test -p isos --bin isos example_factory_defaults_resolve_to_checkout_directory -- --test-threads=1` passed. The stopped local manager was restarted, `./isos.sh config apply config/graph-pool/demo.nodes.json --revision "$(sha256sum config/graph-pool/demo.nodes.json | cut -d' ' -f1)"` returned `applied node graph`, and `GET /api/isos/v1/graph` returned no diagnostics. The current graph file was not edited for this fix. Output is under ignored `data/test/20261117-generated-default-path/`.

Verification for this path: build the workspace binaries, then run `cargo test -p isos --all-targets --no-fail-fast` with `ISOS_AETHER_BIN_DIR` set to that build directory. The stubbed supervisor tests cover managed EI launch and Stop; `python3 crates/isos/tests/unified_owner_replay_cli.py` checks the real EI configuration adapter against the same graph projection.



If `isos` reports a configuration error, read the file path and line or field in the error. Correct that file or the overriding environment variable, then run `isos --help` or `isos status --json` again. A malformed TOML error names the selected `ISOS_CONFIG` file. If `ISOS_CONFIG` points to a missing file, either create it or unset that variable. Process environment overrides still pass the same field validation as TOML.

A source-built `target/` IsOS binary uses the checkout root as its default portable home. An installed `isos` launcher started from the checkout root also selects that checkout's `config/` and `data/` unless `ISOS_HOME` is explicitly set; an explicit `ISOS_CONFIG` changes only the selected TOML. Installed commands started elsewhere keep the installed user and system layouts above. Run `isos doctor`, `isos serve`, and `isos logs all --lines 200` from the checkout root to inspect the same deployment and its files under `data/`. The IsOS JSONL is `data/isos/log/deployments/<deployment>/isos.jsonl`; EOS and EI write their own `eos.jsonl` and `ei.jsonl`, which IsOS merges when reading. See the [local logging guide](logger.md). Setup, API/workbench startup, request completion, failures, and Stop have stable event codes. Successful HTTP requests are `debug`; failures are `warn` or `error`. Request fields contain only method, matched route pattern, status, and duration, never query strings or request bodies. The default level is `debug`, with 8 MiB per file and five rotated backups; `[logging]` may override these bounded defaults. Managed EI receives its log, cache, run, and temporary roots under the selected `data/ei/`.

[中文](../../../docs_cn/isos/core/config_cn.md)

For a checkout diagnosis, run `isos doctor`, start `isos serve` in another terminal, request `curl -fsS http://127.0.0.1:16000/api/isos/v1/graph`, then inspect `isos logs all --level debug --lines 200` and `data/isos/log/deployments/default/isos.jsonl`. The command failure entry records the command and a bounded error code; a missing IsOS core also records its safe validation reason. EI's own diagnostic stream is `data/ei/log/default/ei.jsonl` with the same level names (`trace`, `debug`, `info`, `warn`, `error`).

On 2026-11-08, `cargo test -p isos runtime::logs`, `cargo clippy -p isos --all-targets -- -D warnings`, `cargo fmt --all -- --check`, the five installer tests, and EI logger safety tests passed. A rebuilt installed launcher run from the checkout passed `isos doctor`, `isos serve`, a graph API request, shutdown, and an invalid graph rejection; it wrote setup, HTTP debug, failure, and Stop events under `data/isos/log/default/`. A stub integration replay passed Apply → Run → Stop with EOS and EI processes. Evidence is under `data/test/20261108-root-config-e2e/` and `data/test/20261108-user-launch-smoke/`.

## IsOS setup

[Configuration](config.md) and [installation](config.md) describe setup before Apply. Public setup fields belong to [core setup](../interfaces/core.md#core_setup).

Setup is a manager decision before a deployment is applied. Configuration selects the local home, package sources, and owner launch inputs; installation prepares verified resources. A saved graph still needs validation and Apply before it can start a run. Invalid configuration stops startup with an actionable error, while package or owner failures remain visible through the setup status API.

## Install and upgrade IsOS

The root `install.sh` builds and installs one product release containing the IsOS manager, EOS programs, EI files, and both Web and terminal clients. It requires Rust Cargo, a system `python3.12` executable, uv, Node 22, and npm 10. The installer finds Cargo on `PATH` or at `${CARGO_HOME:-$HOME/.cargo}/bin/cargo`. From a complete checkout, run:

```sh
./install.sh --prefix "$HOME/.local/isos"
```

Product mode is the default when `config/isos/launch.toml` is absent. Run `./install_dev.sh` to build the Rust workspace, synchronize EI dependencies, build the UI, and select local dev mode, then run `./isos <command> [arguments]` from the repository root. `./isos.sh` is the equivalent explicit script. Dev mode selects `target/debug/isos`, `target/debug/aether-*`, `ei/bin`, and `ui/dist/local` directly; it never needs an installed release. The script selects this checkout's configuration files unless `ISOS_CONFIG` or `ISOSUI_CONFIG` is explicit, and selects this checkout's `data/` unless `ISOS_HOME` is explicit, even when called from another working directory. Rebuild changed components before restarting the manager. Set `ISOS_MODE=product` to run an installed release; `install.sh` is for product release staging. `install_dev.sh` writes the ignored local launch selection and build outputs and refreshes selected local source packages; it does not create a system installation.
On repeated `./install_dev.sh` runs, Cargo uses its incremental build, `uv sync` checks the EI environment, and the UI step reuses its local Web and TUI outputs when the UI source, lockfile, build scripts, source commit, tool versions, and API origin have not changed. A changed UI input rebuilds only the local Web and TUI outputs. `./install.sh` likewise reuses npm dependencies and qualified UI outputs when those inputs match, while still running the Rust and EI product build steps. `--no-build` explicitly stages existing product outputs and should only be used after those outputs have been built and checked for the intended source.
After the local builds, `./install_dev.sh` refreshes source-installed package pins in the bound graph. Use `./install_dev.sh --no-source-refresh` to defer that package rebuild. Catalog-installed release pins retain their verified installed bytes.
Installer cache check (2026-11-17): `python3 scripts/ui_build_cache.py release write` followed by `release check` accepted unchanged inputs, rejected an added UI source file, and accepted the restored source. `dev check` accepted the current local outputs. `python3 crates/isos/tests/test_product_installer.py -q` passed all seven stubbed installer cases; `bash -n install.sh install_dev.sh` passed. Logs are under ignored `data/test/20261117-frontend-dev/`.
An actual `./install_dev.sh` replay after the shared-font change took 9.77 seconds to refresh npm dependencies and local UI assets; the unchanged second run took 0.45 seconds and reported reuse of local Web and TUI outputs. The installed uv currently emits a nonfatal warning about the repository root `pyproject.toml`, which intentionally has no Python distribution; EI still resolves from `ei/pyproject.toml` and `ei/uv.lock`.

If `./isos.sh web` reports unavailable manager IPC or a different manager at the HTTP origin, the API port is served by a different IsOS manager. Check that server from outside the checkout with `cd /tmp && ~/.local/isos/bin/isos status --json`; if it has an active deployment, stop it first, then send `kill -TERM <manager.pid>` using the PID in that status output. Alternatively, select unused `ISOS_HTTP_PORT` and `ISOSUI_PORT` values. Retry `./isos.sh web`, then check `./isos.sh status --json` and `./isos.sh logs all --level debug --lines 200`. Its durable events are in `data/isos/log/deployments/default/isos.jsonl`.

On 2026-11-08, this conflict was reproduced with an older idle IsOS server on port 16000. After confirming no active deployment, that server was stopped, the checkout was installed with `./install.sh --no-build`, and `./isos.sh web` succeeded at `http://127.0.0.1:16001/en/isos`. `./isos.sh status --json` confirmed the new manager, and `data/isos/log/deployments/default/isos.jsonl` contained startup and HTTP debug events. A two-manager live check also rejected an HTTP/IPC origin mismatch while the normal `web` command continued to succeed. The five installer tests, Rust formatting, script syntax, and bilingual link audit passed; outputs are under `data/test/20261108-launcher-manager-alignment/`.

The installer builds the Rust workspace, EI with its platform recipe, and the Web and terminal UI, then stages files under `releases/`. It installs the EI wheel and dependencies into the release so Run does not build Python packages. Root CMake remains a development build convenience and has no product install target. It writes `MANIFEST.sha256`, verifies every staged file, and atomically switches `current`. The operator command is `bin/isos`; add that directory to `PATH`. An existing server and deployment keep their old executable until explicitly restarted.

`isos install /absolute/source-checkout` delegates to that checkout's root installer. `isos update` lists available releases without changing files. `isos upgrade [version]` downloads the selected platform bundle and checksum from the configured GitHub repository, verifies the download, and stages it through the same installer. Create an export bundle during source installation with `./install.sh --bundle-output /absolute/isos-0.2.1-linux-x86_64.tar.gz`; the adjacent `.sha256` is the release checksum. To install a verified local bundle, run `./install.sh --bundle /absolute/archive.tar.gz`.

Installation settings live in [`config/isos/install.toml`](../../../config/isos/install.toml). Runtime defaults and local overrides live in [`config/isos/`](../../../config/isos); UI defaults live in [`config/isosui.toml`](../../../config/isosui.toml). `ISOS_CONFIG` and `ISOSUI_CONFIG` may select absolute overrides. The installed launcher selects its bundled copies unless the operator provides overrides.

The `[launch] mode` in `config/isos/launch.toml` selects `dev` or `product` for `./isos`; `ISOS_MODE` overrides it for one command. `isos init --mode` writes the same TOML setting in the selected home. `install.sh` explicitly stages a product release and uses the prefix and release source in `install.toml`; it does not change the checkout's launch mode. Generated graphs in `config/deployments/` are local state and are ignored by Git.

The installer uses `--prefix`, then `ISOS_INSTALL_ROOT`, then `install.toml` for the install directory. `isos update` and `isos upgrade` use `ISOS_INSTALL_CONFIG` to select another TOML file; `ISOS_RELEASE_REPOSITORY` and `ISOS_RELEASE_CHANNEL` override its release source and channel. Keep persistent release preferences in TOML.

In `config/isosui.toml`, `ui.default_interface` selects `web` or `tui` for bare `isos`, and `ui.locale` selects `en` or `cn`. `connection.startup_timeout_ms` bounds how long the CLI waits for an auto-started server (1000–60000 ms); `connection.poll_interval_ms` controls TUI status refresh (500–10000 ms). IsOS validates these values and serves the non-secret UI settings at `/api/isos/v1/system/ui-config`. The local Web server serves its current API origin at `/isosui-runtime.json`, so an installed Web build follows a configured `ISOS_HTTP_PORT`.

Verification: `python3 crates/isos/tests/test_product_installer.py` performs a stubbed source-install and bundle replay, then checks that a corrupt bundle leaves `current` unchanged. Run `bash -n install.sh` for script syntax, and `isos doctor` after installation to inspect graph, interface support, owner, and installed asset readiness.

On 2026-11-08, the dev launcher and log migration were checked with `bash -n isos.sh install.sh`, `cargo fmt --all -- --check`, `cargo test -p isos legacy_json_log`, `cargo build --workspace`, `cargo clippy -p isos --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all`. The local `./isos web`, `./isos status --json`, and `./isos doctor` replay selected `target/debug/isos`, served the API, and reported the local build.

A manual rebuild replay replaced `target/debug/isos` while the manager was running. `./isos web` rejected the stale manager, `./isos stop` stopped it, and the next `./isos web` started the new local build. The stderr and stop outputs are in the same test directory.

The TOML mode follow-up passed script syntax, Rust formatting and Clippy, the `cli_guide` tests, the stubbed product installer tests, and the full bilingual link audit.

For repository publication, see the [shared release guide](../../release.md).

The 2026-11-13 Stop output fix makes the source launcher wait for its stderr relay before returning to the shell. `bash -n isos.sh`, `python3 -m unittest crates.isos.tests.test_launcher_stop -v`, and the bilingual link audit passed. The launcher fixture covers dev and product modes, warning order, and success and failure exit codes without signaling live processes. Complete outputs are under ignored `data/test/20261113-stop-output/`.

## Mutable data layout

[中文](../../../docs_cn/isos/core/config_cn.md) · [IsOS design](README.md)

IsOS, EOS, and EI each own a separate mutable directory. Source and portable installations use `data/<owner>/`, where `<owner>` is `isos`, `eos`, or `ei`. System installations use `/var/lib/island/<owner>/` and place boot-lifetime sockets and locks under `/run/island/<owner>/`. Configuration under `etc/` and shipped resources under `share/` are separate from mutable data.

| Folder | What belongs here | Operator action |
| --- | --- | --- |
| `state/` | Official durable records and owner memory | Back up; do not clear as a cache. |
| `user/` | User-supplied or user-created files outside official records | Back up; delete only by user choice. |
| `attachments/` | Files attached to records or runs | Preserve with the related records. |
| `cache/` | Downloads and derived data that can be rebuilt | Clear only while the owner is stopped. |
| `log/` | Diagnostic and structured events | Retain according to the log policy. |
| `tmp/` | Short-lived work files | Clear after the owner stops. |
| `run/` | Live sockets, locks, and process files | The owner removes stale entries during startup. |

Folders are created only when needed. A directory named `cache` is always rebuildable; never put saved graphs, owner memory, or user files there. Repository test evidence lives under ignored `data/test/<run-id>/`; developer tool caches live under `data/cache/tools/`. The earlier root `logs/test/` evidence and `ei/data/ei/` runtime files were moved into this tree.

## Upgrading a source or portable installation

Stop all owners and back up the installation before migrating. From the installation root, run `python3 scripts/migrate_var_data.py .`. The utility handles legacy `var/lib/<owner>/` files and the old `ISOS_HOME/data/` layout. It checks for symlinks and special files, accepts an existing destination only when its contents match, copies durable files to temporary destinations, verifies their SHA-256 digests, and publishes the copy. It leaves the old files intact. An interrupted copy can leave a `.migration-pending` directory; inspect and remove that directory before retrying. Existing caches, sockets, and temporary files are not copied; old `var/log/` files remain available in the old tree. Start the owners, verify saved records and user files, then remove old files only when the new installation is confirmed.

If IsOS used its former XDG default instead of `ISOS_HOME`, run `python3 scripts/migrate_var_data.py --xdg "$XDG_DATA_HOME"` with the actual XDG data root; the usual default is `~/.local/share`. This copies old `<XDG data>/isos/` records into `<XDG data>/island/isos/` and managed EI state into `<XDG data>/island/ei/`.

System deployments need the same stop, backup, verification, and retention sequence. After confirming the installed service uses the standard `/var/lib/island/<owner>/` and `/var/log/island/<owner>/` roots, run `python3 scripts/migrate_var_data.py --system /` with permission to write those directories. It copies old owner files into their role folders and old owner logs into `log/`; it does not alter `/var/cache/island` or `/run/island`. Existing matching copies are accepted on retry, while differing destinations stop migration. Verify the new locations before retiring old files. Installations with custom roots require an operator-reviewed copy using the same folder mapping.
