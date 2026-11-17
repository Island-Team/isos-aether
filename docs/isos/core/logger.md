# Local core logs

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS core](README.md)
  - [IsOS architecture](architecture.md)
  - [IsOS CLI](cli.md)
  - [IsOS configuration](config.md)
  - [IsOS packages](pkg.md)
  - [IsOS roadmap](roadmap.md)
  - [IsOS server](server.md)
  - [IslandOS services](service.md)
- Subfolders:
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/isos/core/logger_cn.md) · [Core log contract](../interfaces/core.md#core_events) · [Quick start](../start/quick-start.md)

Start with `./isos.sh status --json` to identify the deployment and run. Then use `./isos.sh logs all --lines 80` or the workbench log panel to read the first ready or failure record. IsOS merges its own records with selected EOS and EI diagnostics in its durable owner pools. EOS and EI supply those diagnostics over their private owner sockets, so a UI that opens later can read owner startup and runtime history.

## One JSONL and one readable file per core

For deployment `default`, the producer files are `data/isos/log/deployments/default/isos.jsonl`, `data/eos/log/default/eos.jsonl`, and `data/ei/log/default/ei.jsonl`. IsOS also stores selected owner records in `data/isos/log/deployments/default/owner-pool/eos.jsonl` and `ei.jsonl`, with per-core source cursors. Standalone owners use `local` in place of the deployment name. Launcher records use `data/isos/log/launch/dev/launcher.jsonl` in development or `launch/<session-id>/launcher.jsonl` in product mode. Install records and captured build output share `data/isos/log/install/<session-id>/install.jsonl`.

Each core also writes `isos.log`, `eos.log`, or `ei.log` alongside its JSONL file. Launcher, install, and supervised session JSONL files have same-name `.log` companions. Text lines use `IsOS-v0.2.3@<source>: [LEVEL][UTC timestamp]<message>`; embedded line breaks are escaped. The source is a logical component identity, not an absolute machine path. The browser's in-memory console buffer remains available in its log panel; it has no direct filesystem writer.

JSONL files retain the required `timestamp`, `level`, `source`, and `message` fields. A source identifies a registered node as `<core>.<layer>.<group>.<node>@aether-v0.2.3`, or a system component such as `isos.runtime.manager.diagnostic@aether-v0.2.3`. Optional `event_code`, `run_id`, `correlation_id`, `node_path`, and bounded `fields` preserve structured context for cursors and filtering.

Each core writes only its own producer file. Every 250 ms, IsOS requests bounded owner log pages, normalizes `warning` and `critical` to `warn` and `error`, validates each selected record, and appends overview information plus all warnings and errors to its per-core pools. An owner record without `run_id` is bound to the current IsOS manager or run scope. The pool uses the configured IsOS log rotation limit (8 MiB per file and five backups by default) and persists across UI reconnections. IsOS returns a merged page from its own JSONL and the pools. The producer files remain separate on disk, making a failed owner inspectable even when IsOS is unavailable. Run and Stop health remain authoritative in deployment status.

When a development manager starts a fresh session, its eligible history reset replaces IsOS session and launcher diagnostics only after the prior run is absent or confirmed stopped. It preserves the EOS and EI producer files, IsOS owner pools, and their source byte cursors. A restarted IsOS therefore resumes owner ingestion without erasing the history needed by a reattached UI. An uncertain prior Stop prevents the reset.

An idle workbench reads `manager_session` scope: its IsOS lines belong to the current manager session, while retained EOS and EI owner pool lines from earlier sessions remain visible. After Run, the log view switches to `deployment_run` and fences all three sources by the selected run ID. This keeps startup and reconnect history visible without mixing previous run diagnostics into a live run.

During Run, the manager handles lifecycle commands serially. HTTP process and monitor reads wait for the command budget before declaring the manager unavailable; this avoids false 503 records while owners start. A browser that closes a pending status read can leave the manager with a broken pipe after the command finishes. That read-only peer close is not a manager fault and creates no `manager_client_failed` warning; lifecycle command disconnects remain warnings. The status and monitor clients keep at most one pending poll each; repeated identical connection errors on the EI and EOS workbenches produce one browser message until recovery. Genuine manager or owner failures still retain their diagnostic and request records.

The 2026-11-13 isolated mixed-graph replay used a short private home and ports 16130/16131. Eight concurrent status polls and eight arm-monitor polls all returned 200 after a roughly 30-second startup; none returned 503. A button activation and arm observation, RoboCasa Vision frame, logs, and confirmed Stop passed. IsOS had zero errors, EOS and EI had zero warnings or errors. One IsOS warning pair was a monitor 409 when Stop retired the run during a read; that stale-fence rejection is expected. Full commands, responses, core logs, and Stop proof are under ignored `data/test/20261113-status-monitor-availability/`. Rust formatting, strict Clippy, IsOS tests, UI typecheck/tests/build, and both documentation audits passed with the reproducible commands in that evidence directory.

## Diagnostics and cleanup

Use `./isos.sh logs all --level warn --lines 100` for actionable failures, `--format jsonl` for exact records, and `--current` for the current run cursor. An HTTP error's correlation ID can be matched to the IsOS JSONL record. The reader rejects malformed or oversized records and marks gaps rather than treating them as trusted events.

Old `events.jsonl`, `events/<source>.jsonl`, and build-output text files belong to the retired layout. The log panel continues to read JSONL; direct inspection can use the new `.log` companions. Clear retired files only after the writers and reader are running, so a live file is not deleted while its process still holds it open.

Verification: `bash -n isos.sh install.sh`, `cargo test -p isos`, `cargo fmt --all -- --check`, `cargo clippy -p isos --bin isos -- -D warnings`, UI build/tests, and a setup → Apply → Run → logs → Stop replay. Save complete command output under ignored `data/test/<run-id>/`.

The 2026-11-09 replay used `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test --workspace` so catalog tests resolved the rebuilt EOS profile. It applied `config/deployments/default.nodes.json`, started EOS and EI, read records from all three core files through the current-run API, confirmed Stop receipts, and started a new run. Outputs and the old-file inventory are under `data/test/core-local-jsonl-20261109/`.

The 2026-11-15 readable-log change was checked with `cargo fmt --all -- --check`, strict Clippy for `isos`, `aether-runtime`, and `aether-common`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos` after building `aether-profile`, `cargo test -p aether-common -p aether-runtime`, focused EI pytest, Ruff, Pyright, both documentation audits, shell syntax checks, and a multiline launcher-log sample. Complete command output is under ignored `data/test/20261115-unified-readable-logs/`.
