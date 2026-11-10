# Start and operate EI

[中文](../../../docs_cn/ei/start/quick-start_cn.md) · [Architecture](../design/architecture.md) · [Setup interface](../interfaces/setup.md) · [Runtime interface](../interfaces/runtime.md)

EI is normally launched by IsOS after the saved graph, packages, and owner setup exchange pass validation. The managed setup accepts only the isolated EI core projection. For the button demo, the EI owner passes a run-fenced activation through its input, intent, and safety nodes to EOS while the managed worker provides readiness and lifecycle. Use this page for a local source checkout and for finding the relevant owner status; use the interface pages for wire details.

## Prepare

From the repository root, `./install.sh` builds and installs the product. The EI-only wheel recipe is `cd ei && ./scripts/platform/dispatch.sh build`. Authored EI TOML lives under root `config/ei/`; factory defaults are under `share/ei/defaults/ei/`. An installed release retains `etc/ei/` and `etc/island.toml`. Use `EI_ETC` or `ei/bin/ei --etc <path>` to select another EI configuration root. `--mode` selects one mode from `config/ei/modes.toml` for the invocation.

Keep API keys in the ignored repository-root `.env` or exported environment variables. [`.env.example`](../../../.env.example) lists variable names without credentials. Existing process variables take precedence over `.env`; credentials do not belong in TOML or logs.

## Start

For a managed deployment, start IsOS through its [first-use guide](../../isos/start/README.md), apply a verified graph, then Run. IsOS selects the EI binaries, sends a bounded request to `ei-config`, and supervises the runtime. A successful setup health check alone does not prove the runtime or EOS is ready.

For a source-mode diagnostic from the repository root, run `ei/bin/ei --mode sandbox`. The same source launcher resolves configuration and writes startup, gateway, and shutdown events. `--managed` is reserved for IsOS ownership and rejects an internally launched sandbox. Check `ei/bin/ei --help` and `ei/bin/ei-config --help` for accepted options.

## Inspect, recover, and stop

Standalone diagnostics are under `data/ei/log/local/`; managed runs use `data/ei/log/<deployment>/`. Read `ei.jsonl` for all EI diagnostics in the shared machine-readable envelope. IsOS reads this local file for the log panel; EI does not forward logs through its owner interface. `EI_LOG_DIR` and `EI_DEPLOYMENT` can select a private diagnostic root and deployment. A bounded local check is `timeout 8s ei/bin/ei --mode sandbox`, followed by `tail -n 30 data/ei/log/local/ei.jsonl`.

If EOS discovery is absent or partial, inspect EOS health and the EI link diagnostics before retrying; EI must obtain a complete generation before admitting commands. A model, recall, or optional skill source may degrade, but that does not relax safety. Stop through IsOS for a managed run so the owner records child exit and subsystem evidence. Timeout, disconnect, or cancellation is an unknown physical outcome until EOS supplies a terminal result. Back up `data/ei/state/` and user files; clear cache or temporary data only while EI is stopped.

## Developer checks

From `ei/`, use `uv sync --extra dev --extra vec`, then run `uv run ruff check .`, `uv run pyright`, `PYTHONPATH=src:libexec uv run lint-imports --cache-dir ../data/cache/tools/import-linter-ei`, and focused or full `uv run pytest`. Run `python3 scripts/check_doc_links.py --all` from the repository root when changing documentation. Save complete verification output under ignored `data/test/<run-id>/`.
