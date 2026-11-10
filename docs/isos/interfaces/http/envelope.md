# Local HTTP envelope v1

[中文](../../../../docs_cn/isos/interfaces/http/envelope_cn.md) · [Interface catalog](../README.md)

## Overview

The loopback API exposes several domains under `/api/isos/v1/`. Each handler returns a JSON envelope. Domain pages own route payloads; this page owns transport-wide rules. The API schema is independent of the IsOS product, graph, package, and owner protocol versions.

## Request admission

This page is for implementers of the IsOS loopback API. The current listener binds to `127.0.0.1` on `ISOS_HTTP_PORT` (default `16000`). Before a route runs, the transport checks that `Host` equals the configured loopback host and port. A supplied `Origin` must be the local IsOS origin, the configured IsosUI origin, or one of the hosted UI origins. A cross-site `sec-fetch-site` request without `Origin` is rejected. Current local requests do not carry an owner bearer token.

CORS preflight requires `Origin`; allowed requested methods are `GET`, `POST`, and `PUT`, and the only permitted requested header is `Content-Type`. A rejected Host, Origin, or preflight returns 403 before graph or runtime mutation. Individual routes still validate payload, run fence, and graph authority. The [node-event contract](../ui/node-events.md) gives the first UI control request.

## Interfaces

Success has `schema_version:"0.1.0"`, `ok:true`, domain-specific `data`, and `error:null`. Example from the implemented system route:

```json
{
  "schema_version": "0.1.0",
  "ok": true,
  "data": {
    "api_schema_version": "0.1.0",
    "isos_version": "0.1.0"
  },
  "error": null
}
```

Failure has `schema_version:"0.1.0"`, `ok:false`, `data:null`, and a structured error. The `correlation_id` value below is illustrative; the server generates one for each failure.

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "invalid_request",
    "message": "invalid request",
    "details": null,
    "correlation_id": "req-42"
  }
}
```

| Field | Type / unit | Purpose and validation |
| --- | --- | --- |
| `schema_version` | string / version | Required wire discriminator, exactly `0.1.0`; a product version cannot substitute. |
| `ok` | boolean / none | True only for a successful operation. |
| `data` | JSON value or null / domain-specific | Domain result; null on failure. Do not infer a result from an error response. |
| `error` | object or null / none | Null on success; structured fault on failure. |
| `error.code` | string / none | Stable machine-readable category or domain diagnostic. |
| `error.message` | string / none | Safe human-readable fault; do not parse it as a protocol field. |
| `error.details` | JSON value or null / domain-specific | Optional bounded diagnostics; null means none supplied. |
| `error.correlation_id` | string / none | Server-issued identifier for matching sanitized logs. |

Malformed JSON and invalid input use 400. Host/Origin rejection uses 403, missing resources 404, revision conflicts 409, admission failure 422, and unavailable owners 503. A domain may issue a more specific stable `error.code`; clients branch on code, not message. JSON mutations requiring `schema_version` reject another value before publication. Unknown fields are rejected where the route's request type declares a closed shape; clients must not assume one global unknown-field policy.

## Routes

[Version](version.md), [graph](graph.md), [catalog](catalog.md), [packages](packages.md), [cores](cores.md), [runtime](runtime.md), and [presentations](presentations.md) define their route payloads and side effects. The [node-event contract](../ui/node-events.md) defines the eighth domain. The router is the source of truth for method/path pairs.
