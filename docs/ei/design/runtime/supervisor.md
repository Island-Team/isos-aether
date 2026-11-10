# Supervised task lifecycle

[中文](../../../../docs_cn/ei/design/runtime/supervisor.md) · [Core overview](modules.md)


## Registration and state

A `SubsystemSpec` contains a unique name and an async entry callable receiving `(ready, stop)`. The entry initializes, sets `ready`, then normally runs until `stop`. `_Runtime` stores the task, fresh readiness event, state, first-ready time, cumulative restarts/failures and last error. Duplicate names in the input list collapse in the internal mapping; composition must therefore guarantee name uniqueness.

## Start and readiness


There is an intentional race boundary: a task can set readiness and exit before the next watchdog pass; readiness may briefly succeed, but `is_healthy()` also requires every task to be alive. Health snapshots preserve registration order, report zero uptime until first ready, clamp negative uptime to zero, and include restart count and the last exception representation.

## Failure and restart process


Restarts are processed sequentially inside the watchdog. A backoff for one task delays detection/restart of other completed tasks. There is no maximum restart count, dependency cascade, jitter, state checkpointing, or separate optional failure class. Domain loops must make their own initialization atomic and recover their own durable state.

## Stop and edge conditions

The persistent EI owner also supervises one arm-monitor polling thread per selected run, separately from the health-only worker. It starts after Run has validated the monitor path and joint names, polls EOS at the configured rate, and stores one latest validated snapshot. IsOS readers only inspect this cache. A failed EOS poll preserves the last snapshot for diagnostics but makes reads unavailable until a valid refresh; a regressed sequence is rejected. Stop joins the polling thread and clears its cache before admitting another run. The [owner exchange contract](../../interfaces/setup.md) defines response fields and errors. Focused stubbed boundary verification: `uv run --project ei --extra dev pytest ei/tests/ei/test_owner_service.py -q`.

`stop()` marks every runtime `STOPPING`, sets the shared event, cancels the watchdog, and gathers all subsystem tasks with `return_exceptions=true`. It has no internal graceful timeout and does not cancel entries itself, so every entry must observe `stop`; a non-cooperative entry can block shutdown. Exceptions during this gather are consumed, after which shutdown is logged. A concurrent watchdog backoff observes the stop event before respawning.

Calling `start()` or `stop()` repeatedly is not guarded by an explicit lifecycle state; composition calls each once. The supervisor itself does not enforce the desired provider-before-consumer startup or dependent-before-provider shutdown order. Those safety properties arise from composition, readiness-aware entries, the shared stop protocol, and explicit resource closure in `core.launch`.
