# EOS owner lifecycle

[中文](../../../docs_cn/eos/interfaces/owner-health_cn.md) · [EOS interfaces](README.md) · [Arm control](arm-control.md)

The persistent `aether-profile owner --socket <absolute-path>` process owns EOS setup and its direct `aether-runtime` worker child. IsOS connects over a private Unix socket. The owner can survive an IsOS process exit. Both owner and worker produce local JSONL records under the supplied absolute `EOS_LOG_DIR`; a worker loss emits one error record when observed.

## Setup and graph projection

The owner transport uses schema version `1`. Setup requests use `POST /v1/identity`, `/v1/catalog`, `/v1/options`, `/v1/validate`, and `/v1/render` with `core:"eos"` and matching `op`; setup replies use schema version `2`. The graph projection has format `0.2.0` and one `cores.eos` entry. The catalog contains the eight live demo nodes and the three payload types described in [arm control](arm-control.md) and [arm monitor](arm-monitor.md). Removed health wrappers and `topology_link` are rejected as unknown types.

## Run, Status, and Stop

`POST /v1/run` requires `{schema_version:1,core:"eos",op:"run",run_id,revision,graph,launch}`. `run_id` is a bounded ASCII identifier and `revision` is a 64-character hexadecimal digest. The launch SHA-256 must match the sibling `aether-runtime` executable; args and env are empty. The owner starts `aether-runtime --serve <private-socket> --run-id <id> --revision <digest> --joint-limit <rad> --source-node <path> --joint-names <csv> --sample-interval-ms <ms> --freshness-timeout-ms <ms>` and waits for readiness. A run conflict fails closed.

`POST /v1/status`, `POST /v1/monitor`, and `POST /v1/stop` require the same identity envelope. Monitor returns the latest run-fenced observation without actuating; see [arm monitor](arm-monitor.md). Status and GET health preserve the live worker’s `ready`, `degraded`, or `stale` state; Status reports `mock_hardware:"failed"` after child loss. A degraded or stale worker stays available for read-only monitoring while movement admission fails closed. Stop closes admission, requests child exit, waits, and reaps it. If exit or socket cleanup cannot be confirmed, Stop returns `ok:false` and retains the run identity for retry. The child receives a parent-death signal if the owner crashes. `GET /v1/health` reports owner `idle` or `ready`. Requests over 1 MiB, malformed HTTP, and unsupported operations fail closed.

## Verification

Run `cargo test -p aether-node-profile` and `cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`. The integration test covers Run → activation → duplicate → stale rejection → Stop across the real owner and worker sockets.
