# Owner control projection

[中文](../../../../docs_cn/isos/interfaces/http/owners_cn.md) · [HTTP routes](README.md) · [EOS control contract](../../../eos/interfaces/control.md)

## Overview

IsOS exposes one bounded projection of the selected EOS public control bridge for its Web UI. IsOS checks the active run and saved graph revision, resolves the bridge's loopback address from the verified graph, and forwards only public operations. EOS remains responsible for command admission and actuator safety. This route cannot select an arbitrary owner endpoint or supply a private grant.

## Authentication

The caller must pass the IsOS loopback Host and approved Origin checks. The request carries no token or owner secret. IsOS accepts only a running deployment with a known run ID, matching saved graph bytes, and a running bridge process. The owner bridge performs its own admission.

## Interfaces

Requests and replies use JSON. The IsOS [envelope](envelope.md) has schema `0.1.0`; its `data` is the bounded EOS public reply, retaining EOS's `ok` and operation fields. Request bodies are at most 20,480 bytes and owner replies at most 65,536 bytes. The operation must be one of `discover`, `status`, `health`, `observe`, `time`, `result`, `primitive`, `primitive_typed`, or `manual`. Top-level `grant`, `token`, and `endpoint` fields are rejected. EOS's [control contract](../../../eos/interfaces/control.md) owns operation-specific fields and validation.

## Routes

### POST /api/isos/v1/owners/eos/control

The IsOS Web UI sends an EOS public operation as a JSON object. For example, `{"op":"time"}` returns `{"schema_version":"0.1.0","ok":true,"data":{"ok":true,"monotonic_ns":123},"error":null}` when EOS accepts it. Status `400` rejects malformed JSON or an unlisted operation; `403` rejects Host or Origin; `409` rejects a stale graph revision; `413` rejects an oversized request; `422` reports EOS rejection; `502` reports an invalid or oversized owner reply; `503` reports unavailable run, manager, endpoint, or bridge. A failure uses the same envelope, for example `{"schema_version":"0.1.0","ok":false,"data":null,"error":{"code":"owner_bridge_unavailable","message":"EOS bridge is unavailable","details":null,"correlation_id":"req-1"}}`.

### GET /api/isos/v1/owners/eos/vision/gates

The Web UI reads the current EOS gate catalog through IsOS. A successful IsOS envelope contains the EOS `revision` and `gates` fields. IsOS requires an active bridge and caps the JSON reply at 65,536 bytes. Failure uses `403` for local admission, `409` for stale graph bytes, `502` for invalid or oversized catalog, or `503` for unavailable run or EOS.

### GET /api/isos/v1/owners/eos/vision/ws

The Web UI opens a WebSocket to IsOS after local Host/Origin admission. IsOS checks the same active bridge and graph fence, then relays one text subscription (at most 4,096 bytes), EOS text attestations, and binary frames (at most 8 MiB plus framing and metadata) without rewriting provenance. EOS validates the subscription and emits its attestation; the browser still verifies run, graph, source, projection, and digest. Closing either side closes the relay. The initial HTTP rejection uses `403`, `409`, or `503`; an upstream disconnect closes the socket.
