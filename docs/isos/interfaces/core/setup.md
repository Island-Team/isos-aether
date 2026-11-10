# Selected core setup adapter

[中文](../../../../docs_cn/isos/interfaces/core/setup_cn.md) · [Cores HTTP](../http/cores.md)

The implemented external owner setup exchange uses **schema 2**. IsOS selects the configured owner executable; a client cannot supply a path. Input is bounded to 1 MiB, output to 4 MiB, and the deadline is 10 seconds. `identity`, `catalog`, and `options` are read-only; `options` returns contextual selector choices from the owner projection; `validate` rechecks the draft before Apply; `render` previews bounded generated files. IsOS rejects wrong core, incompatible identity, malformed or oversized reply, escaping path, and timeout before publication. This external wire is independent of the IsOS-owned HTTP `"0.1.0"` schema.

When `isos serve` is active, it starts idle EOS and EI services on owner-only Unix sockets under the IsOS runtime directory. IsOS calls `POST /v1/identity`, `/v1/catalog`, `/v1/options`, `/v1/validate`, and `/v1/render` over HTTP/1.1 on those sockets. The transport request uses `schema_version:1`; the owner translates it to this schema 2 setup operation and returns a schema 2 reply. The request includes only that owner's graph projection, verified package store receipts, endpoints, and revision. A missing owner socket fails the operation; IsOS never selects another owner. Standalone offline setup checks still invoke the selected adapter directly.

The service health route is `GET /v1/health` and returns `schema_version:1`, the fixed core identity, and `state:"idle"`. The current service returns HTTP 501 for Apply, Run, Stop, status, and events; those operations still use the established IsOS process supervisor and run fences until owner worker handoff is implemented.

When an owner rejects a setup request with typed diagnostics, IsOS records up to 32 diagnostic `path` and `code` pairs under `setup_owner_diagnostic` in its local manager log. The graph response retains its stable public rejection code; the local log gives the operator the owner's exact node mismatch location without copying arbitrary diagnostic messages into the shared cursor.
For direct offline adapter invocation, exit status 2 is accepted only with a valid schema 2 reply where `ok` is false and diagnostics are present. Other nonzero exits and a success reply with exit status 2 fail closed.

```json
{
  "schema_version": 2,
  "ok": true,
  "diagnostics": [],
  "files": [],
  "identity": {
    "core": "eos",
    "setup_schema_version": 2,
    "product_version": "0.2.1",
    "graph_version": "0.2.0",
    "node_type_version": "0.2"
  }
}
```

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `schema_version` | integer / external owner schema | Exactly `2` for request and reply. | Required. |
| `op` | `identity`, `catalog`, `options`, `validate`, `render` / operation | Selects one bounded operation. | Required in request. |
| `core` | string / owner role | Must match selected configured owner. | Required in request. |
| `graph`, `revision` | JSON object and optional 64-hex / graph and SHA-256 | Validate/render draft and optional saved-byte fence. | Operation-specific. |
| `ok`, `diagnostics`, `files` | boolean and arrays / result | Closed reply; diagnostic entries have `path`, `code`, `message`; file entries have safe relative `path`, `content`. | Required in reply. |
| `identity`, `catalog` | JSON or null / owner metadata | Present according to operation; identity has `setup_schema_version:2`. | Optional. |

For `options`, `catalog.setting_options` is a bounded array of `{node_path, setting, selected, options}` groups. Each option has `value`, `label`, `description`, `available`, `reason_code`, and `reason`. The `node_path` belongs to the requested owner projection; `setting` names a declared owner selector or an owner-owned hardware field. The owner reports an unavailable selected value rather than silently replacing it. IsOS checks group identity, limits, and the selected draft value before exposing the result. This read-only preview does not authorize Apply: `validate` and `render` recheck the selected settings against the current owner inputs.
