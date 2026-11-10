# EOS setup exchange

[中文](../../../docs_cn/eos/interfaces/setup-v1_cn.md) · [Owner lifecycle](owner-health.md) · [EOS interfaces](README.md)

IsOS calls the persistent EOS owner over an owner-private Unix socket. The transport envelope uses `schema_version: 1`, `core: "eos"`, and an `op` matching `POST /v1/<op>`. Setup replies use schema version `2`; transport, setup, graph, and node type versions are independent. The one-shot `aether-profile setup --json` accepts the same setup content with schema version `2`.

## Operations

`identity` returns product and supported graph and node type versions. `catalog` returns four live demo nodes and two typed payload descriptors. `options` returns a graph-scoped `adapter.type` choice for the built-in mock arm. `validate` checks the EOS-only graph projection and returns diagnostics without files. `render` additionally returns a validated `eos/graph.json` file for IsOS to publish atomically after checking the saved revision again. Setup never launches a worker.

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "validate",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "graph": {
    "version": "0.2.0",
    "robot": {"id": "robot", "name": "Robot"},
    "cores": {"eos": {"layers": {"interface": {"groups": {"ingress": {"nodes": {}}}}}}}
  },
  "endpoints": {},
  "packages": []
}
```

The example shows the envelope shape; a valid graph needs at least one registered EOS node in each present group. The projection contains only `cores.eos`; its declared cross-core input may name an EI source admitted by IsOS. Invalid graph versions, missing EOS layers, unknown types, malformed settings, and incompatible local links return `ok:false`, bounded diagnostics, and no files. Requests and responses are limited to 1 MiB and 4 MiB respectively. The [owner lifecycle contract](owner-health.md) defines Run, Status, Stop, and health.
