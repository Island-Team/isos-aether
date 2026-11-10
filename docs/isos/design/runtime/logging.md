# Local core logs

[中文](../../../../docs_cn/isos/design/runtime/logging_cn.md) · [Core log contract](../../interfaces/core/events.md) · [Quick start](../../start/quick-start.md)

Start with `./isos.sh status --json` to identify the deployment and run. Then use `./isos.sh logs all --lines 80` or the workbench log panel to read the first ready or failure record. IsOS reads local core files and merges their records by timestamp for display. EOS and EI do not send log records through their owner interfaces; those interfaces carry setup and process health.

## One file per core

For deployment `default`, the current files are `data/isos/log/deployments/default/isos.jsonl`, `data/eos/log/default/eos.jsonl`, and `data/ei/log/default/ei.jsonl`. Standalone owners use `local` in place of the deployment name. Launcher records use `data/isos/log/launch/dev/launcher.jsonl` in development or `launch/<session-id>/launcher.jsonl` in product mode. Install records and captured build output share `data/isos/log/install/<session-id>/install.jsonl`.

Every file is UTF-8 JSON Lines with the same required `timestamp`, `level`, `source`, and `message` fields. The source identifies a registered node as `<core>.<layer>.<group>.<node>@aether-v0.2.1`, or a system component such as `isos.runtime.manager.diagnostic@aether-v0.2.1`. Optional fields such as `event_code`, `run_id`, `correlation_id`, `node_path`, and bounded `fields` retain structured context. The source is a logical identity, not an absolute machine path. A readable rendering is produced when requested; there is no parallel `.log` text sink or combined event file.

Each core writes only its own file. IsOS reads bounded records from the three local files, validates their shape and source prefix, and returns a merged page to the UI. The files remain separate on disk, making a failed owner inspectable even when IsOS is unavailable. Run and Stop health remain authoritative in deployment status.

## Diagnostics and cleanup

Use `./isos.sh logs all --level warn --lines 100` for actionable failures, `--format jsonl` for exact records, and `--current` for the current run cursor. An HTTP error's correlation ID can be matched to the IsOS JSONL record. The reader rejects malformed or oversized records and marks gaps rather than treating them as trusted events.

Old `.log`, `events.jsonl`, `events/<source>.jsonl`, and build-output text files belong to the retired layout. They are not read by the current log panel. Clear them only after the new writers and reader are running, so a live file is not deleted while its process still holds it open.

Verification: `bash -n isos.sh install.sh`, `cargo test -p isos`, `cargo fmt --all -- --check`, `cargo clippy -p isos --bin isos -- -D warnings`, UI build/tests, and a setup → Apply → Run → logs → Stop replay. Save complete command output under ignored `data/test/<run-id>/`.

The 2026-11-09 replay used `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace` so catalog tests resolved the rebuilt EOS profile. It applied `config/deployments/default.nodes.json`, started EOS and EI, read records from all three core files through the current-run API, confirmed Stop receipts, and started a new run. Outputs and the old-file inventory are under `data/test/core-local-jsonl-20261109/`.
