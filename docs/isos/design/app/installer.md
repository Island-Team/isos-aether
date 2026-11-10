# Install and upgrade IsOS

The root `install.sh` builds and installs one product release containing the IsOS manager, EOS programs, EI files, and both Web and terminal clients. It requires Rust Cargo, a system `python3.12` executable, uv, Node 22, and npm 10. The installer finds Cargo on `PATH` or at `${CARGO_HOME:-$HOME/.cargo}/bin/cargo`. From a complete checkout, run:

```sh
./install.sh --prefix "$HOME/.local/isos"
```

Product mode is the default when `config/isos/launch.toml` is absent. Run `./install_dev.sh` to build the Rust workspace, synchronize EI dependencies, build the UI, and select local dev mode, then run `./isos <command> [arguments]` from the repository root. `./isos.sh` is the equivalent explicit script. Dev mode selects `target/debug/isos`, `target/debug/aether-*`, `ei/bin`, and `ui/dist/local` directly; it never needs an installed release. The script selects this checkout's configuration files unless `ISOS_CONFIG` or `ISOSUI_CONFIG` is explicit, and selects this checkout's `data/` unless `ISOS_HOME` is explicit, even when called from another working directory. Rebuild changed components before restarting the manager. Set `ISOS_MODE=product` to run an installed release; `install.sh` is for product release staging. `install_dev.sh` writes only the ignored local launch selection and build outputs; it does not create a system installation.

If `./isos.sh web` reports unavailable manager IPC or a different manager at the HTTP origin, the API port is served by a different IsOS manager. Check that server from outside the checkout with `cd /tmp && ~/.local/isos/bin/isos status --json`; if it has an active deployment, stop it first, then send `kill -TERM <manager.pid>` using the PID in that status output. Alternatively, select unused `ISOS_HTTP_PORT` and `ISOSUI_PORT` values. Retry `./isos.sh web`, then check `./isos.sh status --json` and `./isos.sh logs all --level debug --lines 200`. Its durable events are in `data/isos/log/deployments/default/isos.jsonl`.

On 2026-11-08, this conflict was reproduced with an older idle IsOS server on port 16000. After confirming no active deployment, that server was stopped, the checkout was installed with `./install.sh --no-build`, and `./isos.sh web` succeeded at `http://127.0.0.1:16001/en/isos`. `./isos.sh status --json` confirmed the new manager, and `data/isos/log/deployments/default/isos.jsonl` contained startup and HTTP debug events. A two-manager live check also rejected an HTTP/IPC origin mismatch while the normal `web` command continued to succeed. The five installer tests, Rust formatting, script syntax, and bilingual link audit passed; outputs are under `data/test/20261108-launcher-manager-alignment/`.

The installer builds the Rust workspace, EI with its platform recipe, and the Web and terminal UI, then stages files under `releases/`. It installs the EI wheel and dependencies into the release so Run does not build Python packages. Root CMake remains a development build convenience and has no product install target. It writes `MANIFEST.sha256`, verifies every staged file, and atomically switches `current`. The operator command is `bin/isos`; add that directory to `PATH`. An existing server and deployment keep their old executable until explicitly restarted.

`isos install /absolute/source-checkout` delegates to that checkout's root installer. `isos update` lists available releases without changing files. `isos upgrade [version]` downloads the selected platform bundle and checksum from the configured GitHub repository, verifies the download, and stages it through the same installer. Create an export bundle during source installation with `./install.sh --bundle-output /absolute/isos-0.2.1-linux-x86_64.tar.gz`; the adjacent `.sha256` is the release checksum. To install a verified local bundle, run `./install.sh --bundle /absolute/archive.tar.gz`.

Installation settings live in [`config/isos/install.toml`](../../../../config/isos/install.toml). Runtime defaults and local overrides live in [`config/isos/`](../../../../config/isos); UI defaults live in [`config/isosui.toml`](../../../../config/isosui.toml). `ISOS_CONFIG` and `ISOSUI_CONFIG` may select absolute overrides. The installed launcher selects its bundled copies unless the operator provides overrides.

The `[launch] mode` in `config/isos/launch.toml` selects `dev` or `product` for `./isos`; `ISOS_MODE` overrides it for one command. `isos init --mode` writes the same TOML setting in the selected home. `install.sh` explicitly stages a product release and uses the prefix and release source in `install.toml`; it does not change the checkout's launch mode. Generated graphs in `config/deployments/` are local state and are ignored by Git.

The installer uses `--prefix`, then `ISOS_INSTALL_ROOT`, then `install.toml` for the install directory. `isos update` and `isos upgrade` use `ISOS_INSTALL_CONFIG` to select another TOML file; `ISOS_RELEASE_REPOSITORY` and `ISOS_RELEASE_CHANNEL` override its release source and channel. Keep persistent release preferences in TOML.

In `config/isosui.toml`, `ui.default_interface` selects `web` or `tui` for bare `isos`, and `ui.locale` selects `en` or `cn`. `connection.startup_timeout_ms` bounds how long the CLI waits for an auto-started server (1000–60000 ms); `connection.poll_interval_ms` controls TUI status refresh (500–10000 ms). IsOS validates these values and serves the non-secret UI settings at `/api/isos/v1/system/ui-config`. The local Web server serves its current API origin at `/isosui-runtime.json`, so an installed Web build follows a configured `ISOS_HTTP_PORT`.

Verification: `python3 crates/isos/tests/test_product_installer.py` performs a stubbed source-install and bundle replay, then checks that a corrupt bundle leaves `current` unchanged. Run `bash -n install.sh` for script syntax, and `isos doctor` after installation to inspect graph, interface support, owner, and installed asset readiness.

On 2026-11-08, the dev launcher and log migration were checked with `bash -n isos.sh install.sh`, `cargo fmt --all -- --check`, `cargo test -p isos legacy_json_log`, `cargo build --workspace`, `cargo clippy -p isos --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all`. The local `./isos web`, `./isos status --json`, and `./isos doctor` replay selected `target/debug/isos`, served the API, and reported the local build.

A manual rebuild replay replaced `target/debug/isos` while the manager was running. `./isos web` rejected the stale manager, `./isos stop` stopped it, and the next `./isos web` started the new local build. The stderr and stop outputs are in the same test directory.

The TOML mode follow-up passed script syntax, Rust formatting and Clippy, the `cli_guide` tests, the stubbed product installer tests, and the full bilingual link audit.

For repository publication, see the [shared release guide](../../../island/release.md).
