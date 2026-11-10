# Dependent stop contract

[中文](../../../../docs_cn/isos/interfaces/core/dependent-stop_cn.md) · [Design](../../design/README.md)

Current IsOS reads a strict, newline-terminated terminal-stop receipt from the selected owner pipe. The receipt uses schema `1`. IsOS binds the receipt to the selected `run_id`, owner PID, active revision, admitted package process node and expected parts. The owner proves its descendants and physical stop; IsOS independently proves its own direct process group exit. Missing, malformed, stale, late, or contradictory proof yields `unknown`.

| Receipt field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `schema_version`, `type` | integer `1`; string `eos_terminal_stop` / identity | Exact currently accepted owner receipt shape. | Required. |
| `run_id`, `active_revision` | strings / run and SHA-256 | Must match selected run and graph revision. | Required. |
| `eos_pid`, `backend_node` | integer; graph path / owner and node | Must match admitted owner process and backend node. | Required. |
| `backend_pid`, `backend_pgid` | integers or null / process IDs | Bind admitted descendant process group. | Required fields; values may be null. |
| `generation_id`, `stop_epoch` | strings or null / owner generation | Owner-issued stop correlation. | Required fields; values may be null. |
| `admission_closed`, `backend_group_drained`, `backend_reaped`, `sockets_removed` | booleans / stop facts | All required facts are checked before `stopped`. | Required. |
| `parts` | array / selected physical outcomes | Each entry gives `node_path`, registered hardware group as `kind`, and `stopped` or `unknown`; sorted paths must cover the admitted backend references exactly. | Required. |
| `outcome`, `reason_code` | string and optional string / terminal result | `unknown` remains conservative. | Required outcome. |

Restart cannot replace a run when physical stop remains uncertain.

## Interrupted three-core owner run

The three-core demo keeps a private `default.owner-run.json` receipt with `run_id`, `revision`, `pending`, `attempted`, and `stopped`. The normal deployment Stop removes it only after both EI and EOS confirm Stop for that run. If the owners have lost the run, IsOS retains the receipt and reports the owners unavailable; monitor reads return 409 because the old run is no longer valid.

## Local emergency Stop

The CLI command `isos stop` is an emergency shutdown available even when the IsOS server is absent or the owners reject the saved run ID. It sends SIGTERM to local processes identified as belonging to the selected checkout and escalates to SIGKILL if they remain. The saved run ID is not an admission condition. Once no matching processes remain, IsOS retires the stale owner-run receipt and records an emergency audit entry. The audit means that processes exited; it **does not prove physical actuator safety** or replace a terminal-stop receipt. The command prominently warns that it may interrupt active hardware. The operator must independently inspect hardware before another Run.

The operating system's same-user process permissions govern this command by default. An optional `config/isos/emergency-stop.toml` file may contain `code_sha256 = "<64 lowercase hex characters>"`; `ISOS_STOP_CODE_SHA256` overrides that value. When configured, the command requires `--code-stdin` and reads the code from standard input. A mismatch rejects the command before signaling. The code must not be supplied as a command-line argument, where shell history and process listings could expose it. This optional code supplements local OS permissions; it does not grant the command authority over other users' processes.

The workspace `kill.sh` script is the process-signaling implementation used by a source-checkout `isos stop` and also supports a dry-run preview. It scopes candidate processes to this checkout and reports the PIDs it signals. IsOS retires the receipt only after the script verifies that no matching processes remain. A private emergency-stop marker makes offline status report `stopped` with terminal outcome `emergency_signal_unproven`; a new Run clears the marker. Verify physical safety before relaunching.

An operator who has independently verified physical safety may run `isos physical-stop recover --run-id <exact-id> --attest-safe` after stopping the IsOS manager. The CLI requires an exact, regular, private owner-run receipt and no process candidates from its local scan. It writes a private audit record with the run ID, revision, receipt SHA-256, and `operator_attestation` before retiring the receipt. A missing attestation, running manager, changed receipt, wrong run ID, or possible process candidate rejects recovery. This command does not signal an owner or prove physical safety; the operator's attestation supplies that judgment. Run must still pass normal graph, owner, and generation admission afterward.
