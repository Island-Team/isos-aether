# IsOS node interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/node</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Subfolders:
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/isos/interfaces/node/README_cn.md) · [Shared node contracts](../../../node/interfaces/README.md) · [IsOS interfaces](../README.md)

This directory defines IsOS-owned node groups. Every instance uses the shared [type, port, instance, and edge rules](../../../node/interfaces/README.md). IsOS owns saved graph admission and run identity; these nodes expose UI input or read-only output and do not create an actuator route.

| Layer / group | Node interface |
| --- | --- |
| `ui/button` | [Button activation](ui/button.md) |
| `ui/monitor` | [Arm observation](ui/monitor.md) |
| `ui/vision` | [Vision frame](ui/vision.md) |

The starter graph contains `isos.ui.button.trigger@v0.2` and `isos.ui.monitor.arm@v0.2`. Contributed IsOS nodes using `isos.ui.external.v1` may occupy any declared IsOS layer and group; they use a private run-fenced worker. IsOS checks the selected type, exact package pin, current run, and saved revision before input or read. Apply rejects graph edges to a contributed external UI node until a typed delivery bridge is registered. The [client API](../api/client.md) defines browser requests and presentation.

## Contributed external IsOS nodes

A contributed node declares `implementation:{"kind":"external","interface":"isos.ui.external.v1","version":"1","lifecycle_adapter":"unix-socket-v1"}` and a process entrypoint in its exact verified package. IsOS starts it as a direct child with `ISOS_NODE_SOCKET`, `ISOS_NODE_PATH`, `ISOS_RUN_ID`, and `ISOS_GRAPH_REVISION`. The child binds a private Unix socket. One newline-terminated JSON request and reply are exchanged per connection; each is at most 64 KiB, with a two-second socket timeout.

The request has `schema_version:1`, `op:"health"|"input"|"read"|"stop"`, exact `run_id`, 64-hex `revision`, `node_path`, and `payload`. Health and Stop use `payload:null`; input carries `{port,value}` for a registered input; read carries `{port,value:null}` for a registered output. A successful reply echoes the identity fields, sets `ok:true`, and returns the typed value in `payload` for read. IsOS validates both input and read values against the selected data type.

For example, a registered `count` output can be read with the following fenced request and success reply. The output value must match that port’s selected data type.

```json
{
  "schema_version": 1,
  "op": "read",
  "run_id": "run-1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "node_path": "isos/ui/community/panel",
  "payload": {"port": "count", "value": null}
}
```

```json
{
  "schema_version": 1,
  "ok": true,
  "run_id": "run-1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "node_path": "isos/ui/community/panel",
  "payload": {"count": 1}
}
```

Input requires a running deployment; read also permits degraded state. Stop completes only after the fenced reply and child exit. The UI ports do not carry graph edges; unsupported native or external IsOS implementations return `owner_reader_unavailable`. The [package process contract](../package/README.md#process) defines the entrypoint declaration.
