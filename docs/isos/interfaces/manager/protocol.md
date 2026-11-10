# Manager IPC v1

[中文](../../../../docs_cn/isos/interfaces/manager/protocol_cn.md) · [Boundary index](README.md) · [Runtime HTTP](../http/runtime.md)

## Overview

The IsOS CLI and loopback HTTP server call the IsOS-owned manager over an owner-only Unix socket. This wire is `isos.manager@v1`; it is separate from HTTP schema, graph version, and owner setup. The manager serializes deployment mutations.

Start and restart validate the saved `0.2.0` graph and its exact byte revision before preparing a launch. The resulting deployment plan always uses managed admission, including control grants and owner readiness checks.

## Authentication and transport

The socket checks same-UID peer identity. A request is one bounded newline-delimited JSON object with a deadline; an oversized, malformed, timed-out, or unknown operation fails before dispatch. The socket is private and must never be projected into a browser endpoint.

## Interfaces

The request has `schema_version:"0.1.0"`, snake-case `op`, and the payload belonging to that operation. Extra fields in a typed payload are rejected. Example stop request:

```json
{
  "schema_version": "0.1.0",
  "op": "deployment_stop",
  "payload": {
    "deployment": "default",
    "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  }
}
```

The response has `schema_version:"0.1.0"`, `ok`, operation-specific `data`, and optional `error` with stable `code`, safe `message`, and `correlation_id`. Example failure:

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "revision_conflict",
    "message": "request conflicts with current state",
    "correlation_id": "req-42"
  }
}
```

| Field | Type / unit | Purpose and validation |
| --- | --- | --- |
| `schema_version` | string / version | Required, exactly `0.1.0`. |
| `op` | closed snake-case string / none | Selects one operation and payload type; unknown values reject. |
| `payload.deployment` | slug string / none | Selects a manager-owned deployment; required for deployment and append operations. |
| `payload.revision` | optional string / SHA-256 | Exact saved-byte fence for deployment mutations; absence accepts the current saved revision. |
| `payload.service_id` | string / none | Required only for service target/log operations. |
| `payload.cursor` | optional opaque string / none | Service log continuation; clients do not decode it. |
| `payload.limit` | optional unsigned integer / records | Bounds a service log read. |


## Operations

No-payload operations: `manager_identity`, `manager_shutdown`, `services_list`, `services_validate`, and `services_status`. Deployment operations: `deployment_start`, `deployment_restart`, `deployment_stop` use `deployment` and optional `revision`; `deployment_status` and `deployment_active_packages` use `deployment` only. Service mutations: `services_start`, `services_stop`, `services_restart` use `service_id`; `services_logs` adds optional `cursor` and `limit`. A cross-operation field such as `service_id` in `deployment_stop` is invalid. A stop is confirmed only by owner proof; see [dependent stop](../core/dependent-stop.md).

## Validation

Run `cargo build --workspace --bins`, then `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos -- --test-threads=1`. The managed deployment admission check passed 317 unit tests and all IsOS integration tests on 2026-11-07. `cargo fmt --all -- --check`, `cargo clippy -p isos --all-targets -- -D warnings`, and `python3 scripts/check_doc_links.py --all` also passed.

The external input normalization check passed in the same 317-test IsOS suite; `python3 crates/isos/tests/fixtures/jsonl_events/check_cases.py` passed all 37 event cases.
