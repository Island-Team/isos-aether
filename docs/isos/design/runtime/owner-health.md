# Three-core button runtime

[中文](../../../../docs_cn/isos/design/runtime/owner-health_cn.md) · [Node contract](../../interfaces/node/README.md)

Graph `0.2.0` has three peer cores. IsOS owns the saved revision, Apply, launch, and run-fenced button ingress. EI and EOS expose private Unix-socket owner services and supervise their workers. EI passes a typed activation through its input, cognition, and safety stages. EOS receives it through its interface, maps it to a fixed six-joint delta in the demo skill, admits it through the dispatcher, and updates the mock arm. Duplicate event IDs are idempotent within a run; Stop fences later movement.

The owner services remain live when IsOS exits. IsOS persists the run ID and graph revision under its private runtime directory. After a manager restart it reconnects to both services and reports each worker independently. Explicit deployment Stop sends both Stop requests and records confirmation before deleting the run identity.

If a later owner process no longer recognizes that run, the saved receipt keeps IsOS degraded and stops monitor access. The [dependent stop contract](../../interfaces/core/dependent-stop.md) defines both proof-based Stop and local emergency shutdown.

## Emergency shutdown

The CLI `isos stop` command works without a running IsOS server and without matching a saved run ID. It sends SIGTERM to checkout-owned processes and escalates to SIGKILL if necessary. Once no matching process remains, it retires the stale owner-run receipt with an emergency audit record. A successful process exit does not prove that an actuator is physically safe. The command warns that it may interrupt active hardware and that the operator must verify the hardware state.

Local OS permissions control which processes can be signaled. `config/isos/emergency-stop.toml` or its `ISOS_STOP_CODE_SHA256` environment override can add a code check, supplied through standard input with `--code-stdin`; the code does not enter process arguments. In a source checkout, `isos stop` invokes `kill.sh` for process signaling; the script also supports a dry-run preview. The [contract](../../interfaces/core/dependent-stop.md#local-emergency-stop) defines the command's scope and warning.

## Replay evidence

The 2026-11-09 replay is saved under ignored `data/test/20261109-arm-button-integration/`. It ran `isos config init`, validation, Apply, Run, two distinct activations, duplicate retry, stale-revision rejection, unified logs, and confirmed Stop. The mock arm positions were `[0.05,-0.04,0.03,-0.02,0.01,-0.01]` then `[0.1,-0.08,0.06,-0.04,0.02,-0.02]`. Duplicate retry preserved the first positions; stale revision returned 409. Rust workspace tests, UI checks and builds, formatting, Clippy, and bilingual link audits passed. Reproduce with a fresh `ISOS_HOME`, built `target/debug` owner binaries, `ISOS_AETHER_BIN_DIR=$PWD/target/debug`, `ISOS_EI_BIN_DIR=$PWD/ei/bin`, and `EI_PYTHON=$PWD/ei/.venv/bin/python`.

The 2026-11-10 emergency Stop replay is saved under ignored `data/test/emergency-stop-20261110/`. `cargo fmt --all -- --check`, `cargo test -p aether-runtime --bin aether-profile`, focused IsOS recovery tests, `ei/.venv/bin/python -m pytest ei/tests/ei/test_owner_service.py -q`, Ruff, and `python3 scripts/check_doc_links.py --all` passed. `target/debug/isos stop` cleared stale owners without a manager, a fresh Apply and Run reached EOS ready and EI running, `/api/isos/v1/nodes/monitor` returned HTTP 200 with the observer source, and emergency Stop cleared the active mock run. The broad IsOS test run failed two catalog queue/admission cases under the selected test environment; Clippy with `-D warnings` reports unused-code warnings in IsOS test targets. The complete outputs are in the evidence directory.
